use crate::{
    definitions::{
        bodies::B2StartLargeFileUploadBody,
        headers::{B2UploadFileHeaders, B2UploadPartHeaders},
        shared::{B2BucketFileRetention, B2FileLegalHold, B2ServerSideEncryption},
    },
    throttle::Throttle,
    util::{InvalidValue, IsValid, RetryStrategy, SizeUnit},
};

/// File upload options
#[derive(Debug)]
pub struct FileUploadOptions {
    /// Cut off point for the file to count as a big file, from 5 Mib - 5 Gib.
    /// <br> Default is 200 Mib.
    pub large_file_cutoff: u64,
    /// The large file load strategy, refer to [ConstantLargeFileLoadStrategy] to find how they work.
    /// <br> Defaults to LargeFileLoadStrategy::Dynamic([DefaultLargeFileLoadStrategy])
    pub file_load_strategy: LargeFileLoadStrategy,
    /// Upload speed throttle, can be used as
    /// ```rust
    /// // Translates to a MiBPS upload speed limit
    /// let throttle = Throttle::per_second(SizeUnit::MEBIBYTE * 5);
    /// ```
    /// <br> Default is None.
    pub speed_throttle: Option<Throttle<u64>>,
    /// Retry strategy on request failure.
    /// <br> Defaults to RetryStrategy::Dynamic([crate::util::DefaultRetryStrategy]).
    pub retry_strategy: RetryStrategy,
    /// The extra file upload options B2 provides
    /// <br> Check default for [B2FileUploadSettings]
    pub options: B2FileUploadSettings,
}

impl Default for FileUploadOptions {
    fn default() -> Self {
        Self {
            large_file_cutoff: SizeUnit::MEBIBYTE * 200,
            file_load_strategy: Default::default(),
            speed_throttle: None,
            retry_strategy: Default::default(),
            options: Default::default(),
        }
    }
}

impl IsValid for FileUploadOptions {
    fn is_valid(&self) -> Result<(), InvalidValue> {
        if self.large_file_cutoff < SizeUnit::MEBIBYTE * 5
            && self.large_file_cutoff > SizeUnit::GIBIBYTE * 5
        {
            return Err(InvalidValue {
                object_name: "FileUploadOptions".into(),
                value_name: "large_file_cutoff".into(),
                value_as_string: SizeUnit::from(self.large_file_cutoff as f64).to_string(),
                expected: "5 MiB - 5 GiB".into(),
            });
        }

        Ok(())
    }
}

/// The large file load strategy, refer to [ConstantLargeFileLoadStrategy] to find how they work.
#[derive(Debug)]
pub enum LargeFileLoadStrategy {
    Constant(ConstantLargeFileLoadStrategy),
    Dynamic(Box<dyn DynamicLargeFileLoadStrategy + Send + Sync>),
}

impl Default for LargeFileLoadStrategy {
    fn default() -> Self {
        Self::Dynamic(Box::new(DefaultLargeFileLoadStrategy))
    }
}

/// Dictates how large file parts are loaded
/// the approximate total bytes of the file that would be loaded at once will equal `file_size / chunk_size` rounded up to biggest number.
/// part_size must be smaller than the calculated number.
///
/// <br> For example, if we take the default values for bytes and chunk_size of `5 Mib` and `3`, and we're upload a `500 Mib` file
/// the total bytes of the file that would be loaded at once will equal `500 / 3` which is ~166 mibs.
#[derive(Debug, Clone)]
pub struct ConstantLargeFileLoadStrategy {
    /// size of the file part, from 5 Mib - 5 Gib.
    /// <br> Default 5 Mib.
    pub part_size: u64,
    /// How many parts are handled per task. must be at least 1.
    /// <br> Default 3.
    pub chunk_size: u16,
}

impl IsValid for ConstantLargeFileLoadStrategy {
    fn is_valid(&self) -> Result<(), InvalidValue> {
        if self.chunk_size < 1 {
            return Err(InvalidValue {
                object_name: "ConstantLargeFileLoadStrategy".into(),
                value_name: "chunk_size".into(),
                value_as_string: self.chunk_size.to_string(),
                expected: "at least 1".into(),
            });
        }

        if self.part_size < SizeUnit::MEBIBYTE * 5 && self.part_size > SizeUnit::GIBIBYTE * 5 {
            return Err(InvalidValue {
                object_name: "ConstantLargeFileLoadStrategy".into(),
                value_name: "part_size".into(),
                value_as_string: SizeUnit::from(self.part_size as f64).to_string(),
                expected: "5 MiB - 5 GiB".into(),
            });
        }

        Ok(())
    }
}

impl Default for ConstantLargeFileLoadStrategy {
    fn default() -> Self {
        Self {
            part_size: SizeUnit::MEBIBYTE * 5,
            chunk_size: 3,
        }
    }
}

/// A dynamic file load strategy, refer to [ConstantLargeFileLoadStrategy] to find how they work.
pub trait DynamicLargeFileLoadStrategy: std::fmt::Debug {
    fn get_load_strategy(&self, file_size: u64) -> ConstantLargeFileLoadStrategy;
}

#[derive(Debug)]
pub struct DefaultLargeFileLoadStrategy;

impl DynamicLargeFileLoadStrategy for DefaultLargeFileLoadStrategy {
    fn get_load_strategy(&self, file_size: u64) -> ConstantLargeFileLoadStrategy {
        // tries to limit number of parts to 600
        let chunk_size = ((file_size / (SizeUnit::MEBIBYTE * 5)) / 200).max(3);
        let chunk_size = chunk_size.min(u16::MAX as u64) as u16;

        ConstantLargeFileLoadStrategy {
            part_size: SizeUnit::MEBIBYTE * 5,
            chunk_size,
        }
    }
}

/// File upload settings, check [file upload](crate::simple_client::B2SimpleClient::upload_file) to learn mode
#[derive(Clone, Debug)]
pub struct B2FileUploadSettings {
    /// Default to `b2/x-auto`
    pub content_type: String,
    pub src_last_modified_millis: Option<u64>,
    pub b2_content_disposition: Option<String>,
    pub b2_content_language: Option<String>,
    pub b2_expires: Option<String>,
    pub b2_cache_control: Option<String>,
    pub b2_content_encoding: Option<String>,
    pub custom_upload_timestamp: Option<u64>,
    pub legal_hold: Option<B2FileLegalHold>,
    pub file_retention: Option<B2BucketFileRetention>,
    pub server_side_encryption: Option<B2ServerSideEncryption>,
}

impl B2FileUploadSettings {
    pub(super) fn apply_file_upload(self, mut u: B2UploadFileHeaders) -> B2UploadFileHeaders {
        u.content_type = self.content_type;
        u.src_last_modified_millis = self.src_last_modified_millis;
        u.b2_content_disposition = self.b2_content_disposition;
        u.b2_content_language = self.b2_content_language;
        u.b2_expires = self.b2_expires;
        u.b2_cache_control = self.b2_cache_control;
        u.b2_content_encoding = self.b2_content_encoding;
        u.custom_upload_timestamp = self.custom_upload_timestamp;
        u.legal_hold = self.legal_hold;

        if let Some(retention) = self.file_retention {
            u.retention_mode = retention.mode;
            u.retention_retain_until_timestamp = retention.retain_until_timestamp;
        }

        u.server_side_encryption = self.server_side_encryption;

        u
    }

    pub(super) fn apply_large_file_upload(
        self,
        mut u: B2StartLargeFileUploadBody,
    ) -> B2StartLargeFileUploadBody {
        u.content_type = self.content_type;
        u.custom_upload_timestamp = self.custom_upload_timestamp;
        u.legal_hold = self.legal_hold;
        u.file_retention = self.file_retention;
        u.server_side_encryption = self.server_side_encryption;

        if let Some(ref mut info) = u.file_info {
            if let Some(v) = self.src_last_modified_millis {
                info.insert("src_last_modified_millis".into(), v.to_string());
            }

            if let Some(v) = self.b2_content_disposition {
                info.insert("b2-content-disposition".into(), v);
            }

            if let Some(v) = self.b2_content_language {
                info.insert("b2-content-language".into(), v);
            }

            if let Some(v) = self.b2_expires {
                info.insert("b2-expires".into(), v);
            }

            if let Some(v) = self.b2_cache_control {
                info.insert("b2-cache-control".into(), v);
            }

            if let Some(v) = self.b2_content_encoding {
                info.insert("b2-content-encoding".into(), v);
            }
        }

        u
    }

    pub(super) fn apply_file_part_upload(self, mut u: B2UploadPartHeaders) -> B2UploadPartHeaders {
        if let Some(enc) = self.server_side_encryption {
            use B2ServerSideEncryption::*;

            match enc {
                SseB2 { algorithm } => {
                    u.server_side_encryption_customer_algorithm = Some(algorithm);
                }
                SseC {
                    algorithm,
                    customer_key,
                    customer_key_md5,
                } => {
                    u.server_side_encryption_customer_algorithm = Some(algorithm);
                    u.server_side_encryption_customer_key = Some(customer_key);
                    u.server_side_encryption_customer_key_md5 = Some(customer_key_md5);
                }
                Disabled => {}
            }
        }

        u
    }
}

impl Default for B2FileUploadSettings {
    fn default() -> Self {
        Self {
            content_type: "b2/x-auto".into(),
            src_last_modified_millis: None,
            b2_content_disposition: None,
            b2_content_language: None,
            b2_expires: None,
            b2_cache_control: None,
            b2_content_encoding: None,
            custom_upload_timestamp: None,
            legal_hold: None,
            file_retention: None,
            server_side_encryption: None,
        }
    }
}
