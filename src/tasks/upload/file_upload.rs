use std::{
    collections::HashMap,
    convert::Infallible,
    io::SeekFrom,
    ops::Deref,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
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
        bodies::{B2DeleteFileVersionBody, B2FinishLargeFileBody, B2StartLargeFileUploadBody},
        headers::{B2UploadFileHeaders, B2UploadPartHeaders},
        query_params::B2ListPartsQueryParameters,
        responses::B2FilePart,
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
    error::FileUploadError, upload_details::UploadFileDetails, ConstantLargeFileLoadStrategy,
    FileUploadOptions, LargeFileLoadStrategy, B2_MAX_PART_COUNT, B2_MIN_PART_SIZE,
};

/// B2's maximum `maxPartCount` for a single `b2_list_parts` call.
const LIST_PARTS_PAGE_SIZE: u16 = 1000;
/// How many times one part is sent before the whole attempt fails (and resumes on the next retry).
const MAX_PART_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
struct PlannedPart {
    /// 1-based, as B2 numbers parts.
    number: u16,
    start: u64,
    end: u64,
}

impl PlannedPart {
    fn len(&self) -> u64 {
        self.end - self.start
    }
}

/// What every part upload task of one large file shares.
#[derive(Clone)]
struct PartUploadContext {
    client: Arc<B2SimpleClient>,
    file_id: String,
    status: WriteLockArc<FileStatus>,
    file: Arc<RwLock<dyn AsyncFileReader>>,
    sha1s: Arc<LargeFileSha1>,
    total_uploaded: Arc<FileNetworkStats>,
    upload_throttle: Arc<Option<Mutex<Throttle<u64>>>>,
    options: Arc<FileUploadOptions>,
}

pub struct FileUpload {
    id: u64,
    client: Arc<B2SimpleClient>,
    details: UploadFileDetails,
    status: WriteLockArc<FileStatus>,
    file: Arc<RwLock<dyn AsyncFileReader>>,
    stats: Arc<FileNetworkStats>,
    large_file_id: Arc<RwLock<Option<String>>>,
    completion_callbacks: Arc<RwLock<Vec<B2Callback<()>>>>,
    abort_channel: (Sender<()>, Arc<Mutex<Receiver<()>>>),
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
            abort_channel: (tx, Arc::new(Mutex::new(rx))),
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn stats(&self) -> &FileNetworkStats {
        &self.stats
    }

    pub fn status(&self) -> FileStatus {
        self.status.get()
    }

    /// Returns true when the file has finished or has been aborted.
    pub fn has_stopped(&self) -> bool {
        matches!(self.status.get(), FileStatus::Finished | FileStatus::Aborted)
    }

    /// Moves to `to` only when the current status is one of `from`, under a single lock. Returns whether it moved.
    fn transition_status(&self, from: &[FileStatus], to: FileStatus) -> bool {
        let mut status = self.status.lock_write();

        if !from.contains(&*status) {
            return false;
        }

        *status = to;
        true
    }

    /// Whether it was started or not, will only start if status is [`Pending`](FileStatus::Pending).
    ///
    /// Large files that fail are retried on the same unfinished B2 file, re-uploading only the parts B2 doesn't have.
    /// If the upload fails for good or is aborted, its unfinished large file is cancelled.
    pub async fn start(&self) -> Result<B2File, FileUploadError> {
        if !self.transition_status(&[FileStatus::Pending], FileStatus::Working) {
            return Err(FileUploadError::AlreadyStarted);
        }

        let load_strategy = match &self.details.options.file_load_strategy {
            LargeFileLoadStrategy::Constant(strat) => strat.clone(),
            LargeFileLoadStrategy::Dynamic(strat) => strat.get_load_strategy(self.details.file_size),
        };

        let is_large_file = self.details.file_size > self.details.options.large_file_cutoff;

        let validation = self.details.options.is_valid().and_then(|_| match is_large_file {
            true => load_strategy.is_valid(),
            false => Ok(()),
        });

        if let Err(error) = validation {
            // Mark it stopped so progress pollers and the client's tracking don't wait on it forever.
            self.status.set(FileStatus::Finished);
            self.call_finish_callbacks().await;
            return Err(error.into());
        }

        let max_retries = self.details.options.retry_strategy.count().get();
        let mut retries: u64 = 0;
        let abort_receiver = self.abort_channel.1.clone();

        let result = loop {
            self.transition_status(&[FileStatus::Retrying], FileStatus::Working);

            let result = if is_large_file {
                self.upload_large_file(&load_strategy).await
            } else {
                self.upload_small_file().await
            };

            // Keep the real result: a file that completed despite the abort is deleted below.
            if self.status.get() == FileStatus::Aborted {
                break result;
            }

            if result.is_err() && retries < max_retries {
                retries += 1;
                let wait = self.details.options.retry_strategy.wait(retries);
                let mut receiver_lock = abort_receiver.lock().await;

                self.transition_status(&[FileStatus::Working], FileStatus::Retrying);

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

        // Covers an abort that landed before the large file's ID was known, and a final failure:
        // the upload can't be restarted, so its unfinished parts would only be billed storage.
        if result.is_err() {
            self.cancel_large_file().await;
        }

        let aborted = !self.transition_status(
            &[FileStatus::Working, FileStatus::Retrying],
            FileStatus::Finished,
        );

        self.call_finish_callbacks().await;

        if aborted {
            if let Ok(file) = &result {
                self.delete_uploaded_file(file).await;
            }

            return Err(FileUploadError::Aborted);
        }

        result
    }

    /// Will abort ongoing upload if status is [`Working`](FileStatus::Working) or [`Retrying`](FileStatus::Retrying), does nothing otherwise.
    ///
    /// In-flight requests stop within one streamed chunk and any unfinished large file is cancelled.
    /// If the upload still completed on B2 in the meantime, [`start`](Self::start) deletes it and returns [`Aborted`](FileUploadError::Aborted).
    pub async fn abort(&self) {
        if !self.transition_status(&[FileStatus::Working, FileStatus::Retrying], FileStatus::Aborted) {
            return;
        }

        self.abort_channel.0.try_send(()).ok();

        self.cancel_large_file().await;
    }

    pub async fn add_finish_callback(&self, callback: B2Callback<()>) {
        let mut callbacks = self.completion_callbacks.write().await;
        callbacks.push(callback);
    }

    async fn upload_large_file(
        &self,
        file_strat: &ConstantLargeFileLoadStrategy,
    ) -> Result<B2File, FileUploadError> {
        let file = self.file.clone();

        let parts = plan_parts(self.details.file_size, file_strat.part_size);
        let (file_id, uploaded_sha1s) = self.prepare_large_file(&parts).await?;

        let sha1s = Arc::new(LargeFileSha1::new(parts.len()));
        let mut already_uploaded: u64 = 0;

        for part in &parts {
            if let Some(sha1) = uploaded_sha1s.get(&part.number) {
                sha1s.set_sha1((part.number - 1) as usize, sha1.clone());
                already_uploaded += part.len();
            }
        }

        self.stats.set_done_bytes(already_uploaded);

        let missing_parts: Vec<PlannedPart> = parts
            .into_iter()
            .filter(|part| !uploaded_sha1s.contains_key(&part.number))
            .collect();

        let mut join_handles: Vec<JoinHandle<Result<(), FileUploadError>>> = vec![];
        let abort_handles: Arc<RwLock<Vec<AbortHandle>>> = Arc::new(RwLock::new(vec![]));
        self.start_timer().await;

        let context = PartUploadContext {
            client: self.client.clone(),
            file_id: file_id.clone(),
            status: self.status.clone(),
            file,
            sha1s: sha1s.clone(),
            total_uploaded: self.stats.clone(),
            upload_throttle: Arc::new(self.details.options.speed_throttle.clone().map(Mutex::new)),
            options: self.details.options.clone(),
        };

        for chunk in missing_parts.chunks(file_strat.chunk_size as usize) {
            if self.status.get() == FileStatus::Aborted {
                break;
            }

            let task_abort_handles = abort_handles.clone();
            let task_func = FileUpload::part_upload(context.clone(), chunk.to_owned());

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

        // Releases its `sha1s` reference so the finished list can be taken out of the Arc below.
        drop(context);

        for handle in join_handles {
            match handle.await {
                Ok(res) => res,
                Err(err) if err.is_cancelled() => continue,
                Err(err) => Err(FileUploadError::TaskFailed(err.to_string())),
            }?;
        }

        if self.status.get() == FileStatus::Aborted {
            return Err(FileUploadError::Aborted);
        }

        let finished = self
            .client
            .finish_large_file(B2FinishLargeFileBody {
                file_id: file_id.clone(),
                part_sha1_array: Arc::into_inner(sha1s)
                    .expect("sha1s shouldn't be referenced any where else")
                    .into(),
            })
            .await?;

        *self.large_file_id.write().await = None;

        Ok(finished)
    }

    /// Reuses the unfinished large file from a previous attempt when B2 still has it, returning the
    /// SHA1s of the parts that don't need re-uploading. Otherwise starts a new large file.
    async fn prepare_large_file(
        &self,
        parts: &[PlannedPart],
    ) -> Result<(String, HashMap<u16, String>), FileUploadError> {
        let previous_file_id = self.large_file_id.read().await.clone();

        if let Some(file_id) = previous_file_id {
            match self.list_uploaded_parts(&file_id).await {
                Ok(uploaded) => return Ok((file_id, reusable_part_sha1s(parts, &uploaded))),
                Err(B2Error::RequestError(error))
                    if error.status.get() == 400 || error.status.get() == 404 =>
                {
                    self.cancel_large_file().await;
                }
                Err(error) => return Err(error.into()),
            }
        }

        if self.status.get() == FileStatus::Aborted {
            return Err(FileUploadError::Aborted);
        }

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
        *self.large_file_id.write().await = Some(file_id.clone());

        Ok((file_id, HashMap::new()))
    }

    async fn list_uploaded_parts(&self, file_id: &str) -> Result<Vec<B2FilePart>, B2Error> {
        let mut parts = vec![];
        let mut start_part_number = None;

        loop {
            let response = self
                .client
                .list_parts(
                    B2ListPartsQueryParameters::builder()
                        .file_id(file_id.to_string())
                        .start_part_number(start_part_number)
                        .max_part_count(Some(LIST_PARTS_PAGE_SIZE))
                        .build(),
                )
                .await?;

            parts.extend(response.parts);

            match response.next_part_number {
                Some(next) => start_part_number = Some(next),
                None => break,
            }
        }

        Ok(parts)
    }

    async fn upload_small_file(&self) -> Result<B2File, FileUploadError> {
        self.stats.set_done_bytes(0);

        let mut buffer = Vec::with_capacity(self.details.file_size as usize);
        let mut file = self.file.write().await;
        file.seek(SeekFrom::Start(0)).await?;
        file.read_to_end(&mut buffer).await?;
        drop(file);

        let sha1 = Sha1::from(&buffer).digest().to_string();

        let upload_url_response = self
            .client
            .get_upload_url(self.details.bucket_id.clone())
            .await?;

        let b2_upload_headers = B2UploadFileHeaders::builder()
            .authorization(upload_url_response.authorization_token)
            .file_name(self.details.file_name.clone())
            .content_type("b2/x-auto".into())
            .content_length(self.details.file_size)
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


                if status.get() == FileStatus::Aborted {
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
        self.stats.start_time.set(Instant::now());
    }

    async fn cancel_large_file(&self) {
        let large_file_id = self.large_file_id.write().await.take();

        if let Some(id) = large_file_id {
            self.client.cancel_large_file(id).await.ok();
        }
    }

    /// Best effort removal of a file that finished uploading after the upload was aborted.
    async fn delete_uploaded_file(&self, file: &B2File) {
        self.client
            .delete_file_version(
                B2DeleteFileVersionBody::builder()
                    .file_name(file.file_name.clone())
                    .file_id(file.file_id.clone())
                    .build(),
            )
            .await
            .ok();
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
        context: PartUploadContext,
        task_chunk: Vec<PlannedPart>,
    ) -> Result<(), FileUploadError> {
        let PartUploadContext {
            client,
            file_id,
            status,
            file,
            sha1s,
            total_uploaded,
            upload_throttle,
            options,
        } = context;

        let mut upload_part_url_response = client.get_upload_part_url(file_id.clone()).await?;

        for PlannedPart {
            number: part_number,
            start,
            end,
        } in task_chunk
        {
            let status = status.clone();
            let mut buffer = vec![0u8; (end - start) as usize];

            let mut file = file.write().await;
            file.seek(std::io::SeekFrom::Start(start)).await?;
            file.read_exact(&mut buffer).await?;
            drop(file);

            let sha1 = Sha1::from(&buffer).digest().to_string();

            sha1s.set_sha1((part_number - 1) as usize, sha1.clone());

            let buffer = UploadBuffer::new(buffer);

            if status.get() == FileStatus::Aborted {
                break;
            }

            let mut attempt: u32 = 1;

            loop {
                let status = status.clone();

                if status.get() == FileStatus::Aborted {
                    break;
                }

                let total_uploaded = total_uploaded.clone();
                let sha1 = sha1.clone();
                let upload_part_headers = B2UploadPartHeaders::builder()
                    .authorization(upload_part_url_response.authorization_token.clone())
                    .part_number(part_number)
                    .content_length(end - start)
                    .content_sha1(sha1.clone())
                    .build();

                let upload_part_headers = options
                    .options
                    .clone()
                    .apply_file_part_upload(upload_part_headers);

                let upload_throttle = upload_throttle.clone();

                let sent_this_attempt = Arc::new(AtomicU64::new(0));
                let stream_sent_this_attempt = sent_this_attempt.clone();
                let total_uploaded_other = total_uploaded.clone();
                let buffer = buffer.chunks((SizeUnit::KIBIBYTE * 160) as usize);

                let stream = stream! {
                    for chunk in buffer {
                        if status.get() == FileStatus::Aborted {
                            break;
                        }

                        if let Some(ref throttle) = upload_throttle.as_ref() {
                            let mut throttle = throttle.lock().await;
                            throttle.advance_by(chunk.len() as u64).await;
                            drop(throttle);
                        }

                        total_uploaded.add_done_bytes(chunk.len() as u64).await;
                        stream_sent_this_attempt.fetch_add(chunk.len() as u64, Ordering::Relaxed);

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
                    Err(error) if is_retryable_upload_error(&error) && attempt < MAX_PART_ATTEMPTS => {
                        total_uploaded_other
                            .done
                            .fetch_sub(sent_this_attempt.load(Ordering::Relaxed), Ordering::Relaxed);

                        sleep(Duration::from_secs(1 << (attempt - 1))).await;
                        attempt += 1;

                        // B2 asks for a fresh upload URL and token after these errors.
                        upload_part_url_response = client.get_upload_part_url(file_id.clone()).await?;
                    }
                    Err(error) => return Err(error.into()),
                };
            }
        }

        Ok(())
    }
}

/// Errors after which B2 says to get a new upload URL and try again: connection failures, 401, 408, 429 and any 5xx.
fn is_retryable_upload_error(error: &B2Error) -> bool {
    match error {
        B2Error::RequestSendError(_) => true,
        B2Error::RequestError(error) => matches!(error.status.get(), 401 | 408 | 429 | 500..=599),
        _ => false,
    }
}

/// Splits the file into parts. B2 needs at least two parts and at most 10,000, so the part size shrinks
/// for a file no bigger than one part and grows for a file that would need too many.
/// `file_size` must be larger than [`B2_MIN_PART_SIZE`], which holds for anything above a valid cutoff.
fn plan_parts(file_size: u64, part_size: u64) -> Vec<PlannedPart> {
    let part_size = match part_size < file_size {
        true => part_size,
        false => B2_MIN_PART_SIZE.max(file_size.div_ceil(2)),
    };
    let part_size = part_size.max(file_size.div_ceil(B2_MAX_PART_COUNT));
    let mut parts = vec![];
    let mut number: u16 = 0;

    loop {
        let start = part_size * u64::from(number);
        let end = part_size * (u64::from(number) + 1);

        number += 1;

        if end >= file_size {
            parts.push(PlannedPart {
                number,
                start,
                end: file_size,
            });
            break;
        }

        parts.push(PlannedPart { number, start, end });
    }

    parts
}

/// Parts already on B2 that match the planned layout, keyed by part number, with B2's SHA1 for each.
fn reusable_part_sha1s(planned: &[PlannedPart], uploaded: &[B2FilePart]) -> HashMap<u16, String> {
    uploaded
        .iter()
        .filter(|part| {
            let planned_part = (part.part_number as usize)
                .checked_sub(1)
                .and_then(|index| planned.get(index));

            planned_part.is_some_and(|planned_part| planned_part.len() == part.content_length)
                && part.content_sha1 != "none"
        })
        .map(|part| (part.part_number, part.content_sha1.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definitions::shared::B2ServerSideEncryption;

    const MIB: u64 = 1024 * 1024;

    fn assert_send<T: Send>(_: T) {}

    // Compile-time only: callers spawn these on multi-threaded runtimes, so no lock guard may cross an `.await`.
    #[allow(dead_code)]
    fn upload_futures_are_send(upload: &FileUpload) {
        assert_send(upload.start());
        assert_send(upload.abort());
    }

    fn uploaded_part(part_number: u16, content_length: u64, sha1: &str) -> B2FilePart {
        B2FilePart {
            file_id: "large-file".into(),
            part_number,
            content_length,
            content_sha1: sha1.into(),
            content_md5: None,
            server_side_encryption: B2ServerSideEncryption::Disabled,
            upload_timestamp: 0,
        }
    }

    #[test]
    fn plan_parts_splits_with_short_last_part() {
        let parts = plan_parts(12 * MIB, 5 * MIB);

        assert_eq!(
            parts,
            vec![
                PlannedPart { number: 1, start: 0, end: 5 * MIB },
                PlannedPart { number: 2, start: 5 * MIB, end: 10 * MIB },
                PlannedPart { number: 3, start: 10 * MIB, end: 12 * MIB },
            ]
        );
    }

    #[test]
    fn plan_parts_exact_multiple_has_no_empty_part() {
        let parts = plan_parts(10 * MIB, 5 * MIB);

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], PlannedPart { number: 2, start: 5 * MIB, end: 10 * MIB });
    }

    #[test]
    fn plan_parts_always_makes_at_least_two_parts() {
        let parts = plan_parts(6_000_000, 100 * MIB);
        assert_eq!(
            parts,
            vec![
                PlannedPart { number: 1, start: 0, end: B2_MIN_PART_SIZE },
                PlannedPart { number: 2, start: B2_MIN_PART_SIZE, end: 6_000_000 },
            ]
        );

        let parts = plan_parts(40 * MIB, 40 * MIB);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].len(), 20 * MIB);
    }

    #[test]
    fn plan_parts_stays_within_b2_part_limit() {
        let file_size = 100 * 1024 * MIB;
        let parts = plan_parts(file_size, 5 * MIB);

        assert!(parts.len() as u64 <= B2_MAX_PART_COUNT);
        assert_eq!(parts.last().map(|part| part.end), Some(file_size));
        assert!(parts.windows(2).all(|pair| pair[0].end == pair[1].start));
    }

    #[test]
    fn retryable_upload_errors_follow_b2_guidance() {
        let request_error = |status: u16| {
            B2Error::RequestError(crate::error::B2RequestError {
                status: std::num::NonZeroU16::new(status).unwrap(),
                code: String::new(),
                message: None,
            })
        };

        for status in [401, 408, 429, 500, 503, 599] {
            assert!(is_retryable_upload_error(&request_error(status)), "{status} should retry");
        }

        for status in [400, 403, 404] {
            assert!(!is_retryable_upload_error(&request_error(status)), "{status} shouldn't retry");
        }
    }

    #[test]
    fn reusable_parts_keeps_matching_parts_only() {
        let planned = plan_parts(12 * MIB, 5 * MIB);
        let uploaded = vec![
            uploaded_part(1, 5 * MIB, "aaa"),
            // Wrong length: a different part layout, must be re-uploaded.
            uploaded_part(2, 4 * MIB, "bbb"),
            uploaded_part(3, 2 * MIB, "ccc"),
            // Not part of this file's plan.
            uploaded_part(7, 5 * MIB, "ddd"),
            uploaded_part(0, 5 * MIB, "eee"),
        ];

        let reusable = reusable_part_sha1s(&planned, &uploaded);

        assert_eq!(reusable.len(), 2);
        assert_eq!(reusable.get(&1).map(String::as_str), Some("aaa"));
        assert_eq!(reusable.get(&3).map(String::as_str), Some("ccc"));
    }

    #[test]
    fn reusable_parts_skips_parts_without_sha1() {
        let planned = plan_parts(12 * MIB, 5 * MIB);
        let uploaded = vec![uploaded_part(1, 5 * MIB, "none")];

        assert!(reusable_part_sha1s(&planned, &uploaded).is_empty());
    }

    #[test]
    fn list_parts_response_parses_b2_shape() {
        let json = r#"{
            "nextPartNumber": null,
            "parts": [
                {
                    "fileId": "4_ze73ede9c9c8412db49f60715_f200b4e93fbae6252_d20150824_m224353_c900_v8881000_t0001",
                    "partNumber": 1,
                    "contentLength": 100000000,
                    "contentSha1": "062685a84ab248d2488f02f6b01b948de2514ad8",
                    "contentMd5": null,
                    "serverSideEncryption": {"algorithm": "AES256", "mode": "SSE-B2"},
                    "uploadTimestamp": 1462212184000
                }
            ]
        }"#;

        let response: crate::definitions::responses::B2ListPartsResponse =
            serde_json::from_str(json).expect("valid list parts response");

        assert_eq!(response.next_part_number, None);
        assert_eq!(response.parts.len(), 1);
        assert_eq!(response.parts[0].part_number, 1);
    }
}
