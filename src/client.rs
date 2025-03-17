use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

use tokio::{sync::RwLock, task::JoinHandle, time::sleep};

use crate::{
    error::B2Error,
    simple_client::B2SimpleClient,
    tasks::{
        shared::AsyncFileReader,
        upload::{file_upload::FileUpload, FileUploadOptions},
    },
    util::{B2Callback, WriteLockArc},
};

#[derive(Debug, Clone)]
pub enum B2ClientStatus {
    /// Default state, and should be the only state if nothing else went wrong.
    Authed,
    /// The provided key to the client has expired, cannot re-auth, please re-create the client.
    KeyExpired,
}

pub struct B2Client {
    client: Arc<B2SimpleClient>,
    uploading_files: Arc<RwLock<Vec<Option<Arc<FileUpload>>>>>,
    reauth_handle: JoinHandle<()>,
    status: WriteLockArc<B2ClientStatus>,
}

impl B2Client {
    pub async fn new(key_id: String, application_key: String) -> Result<Self, B2Error> {
        let key_id: Arc<str> = Arc::from(key_id.into_boxed_str());
        let application_key: Arc<str> = Arc::from(application_key.into_boxed_str());
        let status = WriteLockArc::new(B2ClientStatus::Authed);

        let client = Arc::new(B2SimpleClient::new(&key_id, &application_key).await?);

        let reauth_client = client.clone();
        let status_expire = status.clone();

        let reauth_handle = tokio::spawn(async move {
            let client = reauth_client.clone();
            let status = status_expire.clone();

            loop {
                let now = SystemTime::now();
                let mut epoch = SystemTime::UNIX_EPOCH + Duration::from_secs(85800);
                let mut expiring = false;

                if let Some(timestamp) = client.auth_data().application_key_expiration_timestamp {
                    let end = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp);

                    if end < epoch {
                        expiring = true;
                        epoch = end;
                    }
                }

                let wait = match epoch.duration_since(now) {
                    Ok(dur) => dur,
                    Err(error) => error.duration(),
                };

                sleep(wait).await;

                if expiring {
                    status.set(B2ClientStatus::KeyExpired).await;
                    break;
                }

                let _ = client.authorize_account(&key_id, &application_key).await;
            }
        });

        let uploading_files = Arc::new(RwLock::new(vec![]));

        Ok(Self {
            client,
            reauth_handle,
            uploading_files,
            status,
        })
    }

    /// Gets current client status
    pub fn status(&self) -> B2ClientStatus {
        (*self.status).clone()
    }

    /// Returns reference to inner basic client
    pub fn basic_client(&self) -> Arc<B2SimpleClient> {
        self.client.clone()
    }

    /// Creates files upload tracker and returns reference to it. <br><br>
    /// Tracker doesn't start upload automatically, it needs to be started manually.
    pub async fn create_upload<T>(
        &self,
        file: T,
        file_name: String,
        bucket_id: String,
        optional_info: Option<HashMap<String, String>>,
        file_size: u64,
        options: Option<FileUploadOptions>,
    ) -> Arc<FileUpload>
    where
        T: AsyncFileReader + 'static,
    {
        let file_handle = FileUpload::new(
            file,
            file_name,
            bucket_id,
            optional_info,
            file_size,
            options.unwrap_or_else(|| FileUploadOptions::default()),
            self.client.clone(),
        );

        self.push_upload(file_handle.clone()).await;
        let id = file_handle.id();
        let uploading_files = self.uploading_files.clone();

        file_handle
            .add_finish_callback(B2Callback::from_async_fn(move |_| {
                let uploading_files = uploading_files.clone();

                async move {
                    B2Client::abort_upload_inner(uploading_files, id).await;
                }
            }))
            .await;

        file_handle
    }

    /// Gets the list of current tracked upload tasks
    pub async fn get_current_tracked_uploads(&self) -> Vec<Arc<FileUpload>> {
        let lock_guard = self.uploading_files.read().await;

        lock_guard.iter().filter_map(|e| e.clone()).collect()
    }

    /// Aborts a specific upload using its ID
    pub async fn abort_upload(&self, upload_id: u64) {
        B2Client::abort_upload_inner(self.uploading_files.clone(), upload_id).await;
    }

    async fn push_upload(&self, upload: Arc<FileUpload>) {
        let lock_guard = self.uploading_files.read().await;
        let set_index = lock_guard.iter().position(|slot| slot.is_none());
        drop(lock_guard);

        let mut lock_guard = self.uploading_files.write().await;

        match set_index {
            Some(index) => lock_guard[index] = Some(upload),
            None => lock_guard.push(Some(upload)),
        };
    }

    async fn abort_upload_inner(
        uploads: Arc<RwLock<Vec<Option<Arc<FileUpload>>>>>,
        upload_id: u64,
    ) {
        let uploads_lock = uploads.read().await;
        let upload_to_remove = uploads_lock.iter().position(|slot| match slot {
            Some(upload) => upload.id() == upload_id,
            None => false,
        });
        drop(uploads_lock);

        if let Some(index) = upload_to_remove {
            let mut uploads_lock = uploads.write().await;
            uploads_lock[index] = None;
        }
    }
}

impl Drop for B2Client {
    fn drop(&mut self) {
        self.reauth_handle.abort();
    }
}
