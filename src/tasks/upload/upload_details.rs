use std::{collections::HashMap, sync::Arc};

use super::FileUploadOptions;

pub(super) struct UploadFileDetails {
    pub(super) file_size: u64,
    pub(super) file_name: String,
    pub(super) bucket_id: String,
    pub(super) optional_info: Option<HashMap<String, String>>,
    pub(super) options: Arc<FileUploadOptions>,
}
