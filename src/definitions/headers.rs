use super::shared::{
    B2FileLegalHold, B2FileRetentionMode, B2ServerSideEncryption, B2ServerSideEncryptionAlgorithm,
};
use crate::util::IntoHeaderMap;
use serde::Serialize;
use typed_builder::TypedBuilder;

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct B2UploadPartHeaders {
    #[builder(!default)]
    #[serde(rename = "Authorization")]
    #[builder(!default)]
    pub authorization: String,
    #[serde(rename = "X-Bz-Part-Number")]
    #[builder(!default)]
    pub part_number: u16,
    #[serde(rename = "Content-Length")]
    #[builder(!default)]
    pub content_length: u64,
    #[serde(rename = "X-Bz-Content-Sha1")]
    #[builder(!default)]
    pub content_sha1: String,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Algorithm")]
    pub server_side_encryption_customer_algorithm: Option<B2ServerSideEncryptionAlgorithm>,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Key")]
    pub server_side_encryption_customer_key: Option<String>,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Key-Md5")]
    pub server_side_encryption_customer_key_md5: Option<String>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[builder(field_defaults(default))]
pub struct B2UploadFileHeaders {
    #[builder(!default)]
    #[serde(rename = "Authorization")]
    pub authorization: String,
    #[serde(rename = "X-Bz-File-Name")]
    #[builder(!default)]
    pub file_name: String,
    #[builder(!default)]
    #[serde(rename = "Content-Type")]
    pub content_type: String,
    #[builder(!default)]
    #[serde(rename = "Content-Length")]
    pub content_length: u64,
    #[builder(!default)]
    #[serde(rename = "X-Bz-Content-Sha1")]
    pub content_sha1: String,
    #[serde(rename = "X-Bz-Info-src_last_modified_millis")]
    pub src_last_modified_millis: Option<u64>,
    #[serde(rename = "X-Bz-Info-b2-content-disposition")]
    pub b2_content_disposition: Option<String>,
    #[serde(rename = "X-Bz-Info-b2-content-language")]
    pub b2_content_language: Option<String>,
    #[serde(rename = "X-Bz-Info-b2-expires")]
    pub b2_expires: Option<String>,
    #[serde(rename = "X-Bz-Info-b2-cache-control")]
    pub b2_cache_control: Option<String>,
    #[serde(rename = "X-Bz-Info-b2-content-encoding")]
    pub b2_content_encoding: Option<String>,
    #[serde(rename = "X-Bz-Custom-Upload-Timestamp")]
    pub custom_upload_timestamp: Option<u64>,
    #[serde(rename = "X-Bz-File-Legal-Hold")]
    pub legal_hold: Option<B2FileLegalHold>,
    #[serde(rename = "X-Bz-File-Retention-Mode")]
    pub retention_mode: Option<B2FileRetentionMode>,
    #[serde(rename = "X-Bz-File-Retention-Retain-Until-Timestamp")]
    pub retention_retain_until_timestamp: Option<u64>,
    #[serde(rename = "X-Bz-Server-Side-Encryption")]
    pub server_side_encryption: Option<B2ServerSideEncryption>,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Algorithm")]
    pub server_side_encryption_customer_algorithm: Option<B2ServerSideEncryptionAlgorithm>,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Key")]
    pub server_side_encryption_customer_key: Option<String>,
    #[serde(rename = "X-Bz-Server-Side-Encryption-Customer-Key-Md5")]
    pub server_side_encryption_customer_key_md5: Option<String>,
}

impl IntoHeaderMap for B2UploadPartHeaders {}
impl IntoHeaderMap for B2UploadFileHeaders {}
