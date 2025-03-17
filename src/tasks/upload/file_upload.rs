use std::{
    collections::HashMap,
    convert::Infallible,
    ops::Deref,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};

use async_stream::stream;
use bytes::Bytes;
use sha1_smol::Sha1;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex, RwLock,
    },
    task::{AbortHandle, JoinHandle},
    time::sleep,
};

use crate::{
    definitions::{
        bodies::{B2FinishLargeFileBody, B2StartLargeFileUploadBody},
        headers::{B2UploadFileHeaders, B2UploadPartHeaders},
        shared::B2File,
    },
    error::B2Error,
    simple_client::B2SimpleClient,
    tasks::upload::{large_file_sha1::LargeFileSha1, upload_buffer::UploadBuffer},
    throttle::Throttle,
    util::{write_lock_arc::WriteLockArc, B2Callback, IsValid, SizeUnit},
};

use crate::tasks::shared::{AsyncFileReader, FileNetworkStats, FileStatus};

use super::{
    error::FileUploadError, upload_details::UploadFileDetails, FileUploadOptions,
    LargeFileLoadStrategy,
};
pub struct FileUpload {
    id: u64,
    client: Arc<B2SimpleClient>,
    details: UploadFileDetails,
    status: WriteLockArc<FileStatus>,
    file: Arc<RwLock<dyn AsyncFileReader>>,
    stats: Arc<FileNetworkStats>,
    large_file_id: Arc<RwLock<Option<String>>>,
    completion_callbacks: Arc<RwLock<Vec<B2Callback<()>>>>,
    abort_channel: (WriteLockArc<Sender<()>>, WriteLockArc<Receiver<()>>),
}

impl FileUpload {
    pub fn new<F: AsyncFileReader + 'static>(
        file: F,
        file_name: String,
        bucket_id: String,
        optional_info: Option<HashMap<String, String>>,
        file_size: u64,
        options: FileUploadOptions,
        client: Arc<B2SimpleClient>,
    ) -> Arc<Self> {
        let (tx, rx) = mpsc::channel::<()>(1);

        Arc::new(Self {
            id: rand::random(),
            client,
            details: UploadFileDetails {
                file_size,
                file_name,
                bucket_id,
                optional_info,
                options: Arc::new(options),
            },
            large_file_id: Arc::new(RwLock::new(None)),
            status: WriteLockArc::new(FileStatus::Pending),
            file: Arc::new(RwLock::new(file)),
            stats: Arc::new(FileNetworkStats::new(file_size as f64)),
            completion_callbacks: Arc::new(RwLock::new(vec![])),
            abort_channel: (WriteLockArc::new(tx), WriteLockArc::new(rx)),
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn stats(&self) -> &FileNetworkStats {
        &self.stats
    }

    pub fn status(&self) -> FileStatus {
        (*self.status).clone()
    }

    /// Returns true when the file has finished or has been aborted.
    pub fn has_stopped(&self) -> bool {
        *self.status == FileStatus::Finished || *self.status == FileStatus::Aborted
    }

    /// Whether it was started or not, will only start if status is [`Pending`](FileStatus::Pending)
    pub async fn start(&self) -> Result<B2File, FileUploadError> {
        if *self.status != FileStatus::Pending {
            return Err(FileUploadError::AlreadyStarted);
        }

        self.details.options.is_valid()?;

        self.status.set(FileStatus::Working).await;

        let retry_count = self.details.options.retry_strategy.count();
        let mut curr_retry_count = 1;
        let abort_receiver = self.abort_channel.1.clone();

        let result = loop {
            curr_retry_count += 1;

            let result = match self.details.file_size {
                size if size <= self.details.options.large_file_cutoff => {
                    self.upload_small_file().await
                }
                _ => {
                    let file_strat = match &self.details.options.file_load_strategy {
                        LargeFileLoadStrategy::Constant(strat) => strat,
                        LargeFileLoadStrategy::Dynamic(strat) => {
                            &strat.get_load_strategy(self.details.file_size)
                        }
                    };

                    file_strat.is_valid()?;

                    self.upload_large_file().await
                }
            };

            if *self.status == FileStatus::Aborted {
                break Err(FileUploadError::Aborted);
            }

            if result.is_err() && curr_retry_count <= retry_count.get() {
                let wait = self.details.options.retry_strategy.wait(curr_retry_count);
                let mut receiver_lock = abort_receiver.lock_write().await;

                let mut status = self.status.lock_write().await;
                if *status == FileStatus::Working {
                    *status = FileStatus::Retrying;
                }
                drop(status);

                tokio::select! {
                    _ = sleep(wait) => {},
                    _ = receiver_lock.recv() => {
                        break Err(FileUploadError::Aborted)
                    }
                };

                continue;
            }

            break result;
        };

        let mut status = self.status.lock_write().await;
        if *status == FileStatus::Working {
            *status = FileStatus::Finished;
        }
        drop(status);

        self.call_finish_callbacks().await;

        if *self.status == FileStatus::Aborted {
            return Err(FileUploadError::Aborted);
        }

        return result;
    }

    /// Will abort ongoing upload if status is [`Working`](FileStatus::Working) or [`Retrying`](FileStatus::Retrying), does nothing otherwise.
    pub async fn abort(&self) {
        // If its not working there's nothing to do
        if *self.status != FileStatus::Working || *self.status != FileStatus::Retrying {
            return;
        }

        self.status.set(FileStatus::Aborted).await;

        let sender = &self.abort_channel.0;
        sender.send(()).await.ok();

        self.cancel_large_file().await;
    }

    pub async fn add_finish_callback(&self, callback: B2Callback<()>) {
        let mut callbacks = self.completion_callbacks.write().await;
        callbacks.push(callback);
    }

    async fn upload_large_file(&self) -> Result<B2File, FileUploadError> {
        let file = self.file.clone();

        let start_large_upload_body = B2StartLargeFileUploadBody::builder()
            .bucket_id(self.details.bucket_id.clone())
            .file_name(self.details.file_name.clone())
            .content_type("b2/x-auto".into())
            .file_info(self.details.optional_info.clone())
            .build();

        let start_large_upload_body = self
            .details
            .options
            .options
            .clone()
            .apply_large_file_upload(start_large_upload_body);

        let start_large_file_response = self
            .client
            .start_large_file(start_large_upload_body)
            .await?;

        let file_id = start_large_file_response.file_id;
        let total_uploaded = self.stats.clone();

        let mut large_file = self.large_file_id.write().await;
        *large_file = Some(file_id.clone());
        drop(large_file);

        let file_strat = match &self.details.options.file_load_strategy {
            LargeFileLoadStrategy::Constant(strat) => strat,
            LargeFileLoadStrategy::Dynamic(strat) => {
                &strat.get_load_strategy(self.details.file_size)
            }
        };

        let mut parts: Vec<((u64, u64), u16)> = vec![];
        let mut current_range_start: u16 = 0;

        loop {
            let start = file_strat.part_size * u64::from(current_range_start);
            let end = file_strat.part_size * (u64::from(current_range_start) + 1);

            current_range_start += 1;

            if end >= self.details.file_size {
                parts.push(((start, self.details.file_size), current_range_start));
                break;
            } else {
                parts.push(((start, end), current_range_start));
            }
        }

        let sha1s = Arc::new(LargeFileSha1::new(parts.len()));
        let mut join_handles: Vec<JoinHandle<Result<(), FileUploadError>>> = vec![];
        let abort_handles: Arc<RwLock<Vec<AbortHandle>>> = Arc::new(RwLock::new(vec![]));
        self.start_timer().await;

        let upload_throttle = Arc::new(
            self.details
                .options
                .speed_throttle
                .clone()
                .map(|t| Mutex::new(t)),
        );

        let status = self.status.clone();

        for chunk in parts.chunks(file_strat.chunk_size as usize) {
            let task_chunk = chunk.to_owned();
            let file_id = file_id.clone();
            let sha1s = sha1s.clone();
            let task_abort_handles = abort_handles.clone();
            let total_uploaded = total_uploaded.clone();
            let status = status.clone();

            if *status == FileStatus::Aborted {
                break;
            }

            let upload_throttle = upload_throttle.clone();
            let file = file.clone();
            let client = self.client.clone();

            let options = self.details.options.clone();

            let task_func = FileUpload::part_upload(
                client,
                file_id,
                status,
                task_chunk,
                file,
                sha1s,
                total_uploaded,
                upload_throttle,
                options,
            );

            let join_handle = tokio::spawn(async move {
                let result = task_func.await;

                if let Err(err) = result {
                    for handle in task_abort_handles.read().await.iter() {
                        handle.abort();
                    }

                    return Err(err);
                }

                Ok(())
            });

            let abort_handle = join_handle.abort_handle();

            join_handles.push(join_handle);
            abort_handles.write().await.push(abort_handle);
        }

        for handle in join_handles {
            match handle.await {
                Ok(res) => res,
                Err(err) => match err.is_cancelled() {
                    true => continue,
                    false => panic!("{:#?}", err),
                },
            }?;
        }

        Ok(self
            .client
            .finish_large_file(B2FinishLargeFileBody {
                file_id: file_id.clone(),
                part_sha1_array: Arc::into_inner(sha1s)
                    .expect("sha1s shouldn't be referenced any where else")
                    .into(),
            })
            .await?)
    }

    async fn upload_small_file(&self) -> Result<B2File, FileUploadError> {
        let mut buffer = Vec::with_capacity(self.details.file_size as usize);
        let mut file = self.file.write().await;
        file.read_to_end(&mut buffer).await?;
        drop(file);

        let file_size = buffer.len() as u64;
        let sha1 = Sha1::from(&buffer).digest().to_string();

        let upload_url_response = self
            .client
            .get_upload_url(self.details.bucket_id.clone())
            .await?;

        let b2_upload_headers = B2UploadFileHeaders::builder()
            .authorization(upload_url_response.authorization_token)
            .file_name(urlencoding::encode(&self.details.file_name).into_owned())
            .content_type("b2/x-auto".into())
            .content_length(file_size as u32)
            .content_sha1(sha1)
            .build();

        let b2_upload_headers = self
            .details
            .options
            .options
            .clone()
            .apply_file_upload(b2_upload_headers);

        let buffer = UploadBuffer::new(buffer);
        let uploaded = self.stats.clone();
        let status = self.status.clone();
        let upload_throttle = Arc::new(
            self.details
                .options
                .speed_throttle
                .clone()
                .map(|t| Mutex::new(t)),
        );

        let stream = stream! {
            for chunk in buffer.chunks((SizeUnit::KIBIBYTE * 80) as usize) {
                if let Some(ref throttle) = upload_throttle.as_ref() {
                    let mut throttle = throttle.lock().await;
                    throttle.advance_by(chunk.len() as u64).await;
                    drop(throttle);
                }


                if *status == FileStatus::Aborted {
                    break;
                }

                uploaded.add_done_bytes(chunk.len() as u64).await;

                yield Ok::<Bytes, Infallible>(chunk);
            }
        };

        self.start_timer().await;

        let file = self
            .client
            .upload_file(
                reqwest::Body::wrap_stream(stream),
                upload_url_response.upload_url,
                b2_upload_headers,
                self.details.optional_info.clone(),
            )
            .await?;

        Ok(file)
    }

    async fn start_timer(&self) {
        self.stats.start_time.set(Instant::now()).await;
    }

    async fn cancel_large_file(&self) {
        let large_file = self.large_file_id.read().await;

        if let Some(id) = large_file.deref() {
            self.client.cancel_large_file(id.clone()).await.ok();
        }
    }

    async fn call_finish_callbacks(&self) {
        let callbacks = self.completion_callbacks.read().await;

        for callback in callbacks.deref() {
            match callback {
                B2Callback::Fn(fun) => fun(()),
                B2Callback::AsyncFn(fun) => fun(()).await,
            }
        }
    }

    async fn part_upload(
        client: Arc<B2SimpleClient>,
        file_id: String,
        status: WriteLockArc<FileStatus>,
        task_chunk: Vec<((u64, u64), u16)>,
        file: Arc<RwLock<dyn AsyncFileReader>>,
        sha1s: Arc<LargeFileSha1>,
        total_uploaded: Arc<FileNetworkStats>,
        upload_throttle: Arc<Option<Mutex<Throttle<u64>>>>,
        options: Arc<FileUploadOptions>,
    ) -> Result<(), FileUploadError> {
        let mut upload_part_url_response = client.get_upload_part_url(file_id.clone()).await?;

        for ((start, end), part_number) in task_chunk {
            let status = status.clone();
            let mut buffer = vec![0u8; (end - start) as usize];

            let mut file = file.write().await;
            file.seek(std::io::SeekFrom::Start(start)).await?;
            file.read_exact(&mut buffer).await?;
            drop(file);

            let sha1 = Sha1::from(&buffer).digest().to_string();

            sha1s.set_sha1((part_number - 1) as usize, sha1.clone());

            let buffer = UploadBuffer::new(buffer);

            if *status == FileStatus::Aborted {
                break;
            }

            loop {
                let status = status.clone();

                if *status == FileStatus::Aborted {
                    break;
                }

                let total_uploaded = total_uploaded.clone();
                let sha1 = sha1.clone();
                let upload_part_headers = B2UploadPartHeaders::builder()
                    .authorization(upload_part_url_response.authorization_token.clone())
                    .part_number(part_number)
                    .content_length((end - start) as u32)
                    .content_sha1(sha1.clone())
                    .build();

                let upload_part_headers = options
                    .options
                    .clone()
                    .apply_file_part_upload(upload_part_headers);

                let upload_throttle = upload_throttle.clone();

                let mut total_uploaded_here: u64 = 0;
                let total_uploaded_other = total_uploaded.clone();
                let buffer = buffer.chunks((SizeUnit::KIBIBYTE * 160) as usize);

                let stream = stream! {
                    for chunk in buffer {
                        if *status == FileStatus::Aborted {
                            break;
                        }

                        if let Some(ref throttle) = upload_throttle.as_ref() {
                            let mut throttle = throttle.lock().await;
                            throttle.advance_by(chunk.len() as u64).await;
                            drop(throttle);
                        }

                        total_uploaded.add_done_bytes(chunk.len() as u64).await;
                        *(&mut total_uploaded_here) += chunk.len() as u64;

                        yield Ok::<_, Infallible>(chunk);
                    }

                };

                let stream = reqwest::Body::wrap_stream(stream);

                let result = client
                    .upload_part(
                        upload_part_headers,
                        stream,
                        upload_part_url_response.upload_url.clone(),
                    )
                    .await;

                match result {
                    Ok(_) => break,
                    Err(error) => match error {
                        B2Error::RequestError(error) => match error.status.get() {
                            503 => {
                                upload_part_url_response =
                                    match client.get_upload_part_url(file_id.clone()).await {
                                        Ok(resp) => resp,
                                        Err(err) => return Err(err.into()),
                                    };

                                total_uploaded_other
                                    .done
                                    .fetch_sub(total_uploaded_here, Ordering::Relaxed);

                                sleep(Duration::from_millis(200)).await;
                            }
                            _ => return Err(B2Error::RequestError(error).into()),
                        },
                        err => return Err(err.into()),
                    },
                };
            }
        }

        Ok(())
    }
}
