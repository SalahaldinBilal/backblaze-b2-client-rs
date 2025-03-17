use std::num::NonZeroU64;

use serde::{Deserialize, Serialize};

use super::shared::{
    B2AppKey, B2Bucket, B2EventNotificationRule, B2File, B2BucketFileRetention, B2KeyCapability,
    B2ServerSideEncryption,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2UpdateFileRetentionResponse {
    /// The unique identifier for this version of this file.
    pub file_id: String,
    /// The name of this file.
    pub file_name: String,
    /// The updated file retention settings.
    pub file_retention: B2BucketFileRetention,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2GetUploadPartUrlResponse {
    /// The unique ID of file being uploaded.
    pub file_id: String,
    /// The URL that can be used to upload parts of this file, see [b2_upload_part](crate::simple_client::B2SimpleClient::upload_part).
    pub upload_url: String,
    /// The `authorizationToken` that must be used when uploading files with this URL.
    /// This token is valid for 24 hours or until the `uploadUrl` endpoint rejects an upload, see [b2_upload_part](crate::simple_client::B2SimpleClient::upload_part).
    pub authorization_token: String,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2ListFilesResponse {
    /// The array of files
    pub files: Vec<B2File>,
    /// What to pass in to [`startFileName`](super::query_params::B2ListFileNamesQueryParameters::start_file_name) for the next search to continue where this one left off,
    /// or null if there are no more files. Note this this may not be the name of an actual file, but using it is guaranteed to find the next file in the bucket.
    pub next_file_name: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2GetUploadUrlResponse {
    /// The identifier for the bucket.
    pub bucket_id: String,
    /// The URL that can be used to upload files to this bucket, see [b2_upload_file](crate::simple_client::B2SimpleClient::upload_file).
    pub upload_url: String,
    /// The `authorizationToken` that must be used when uploading files with this URL.
    /// This token is valid for 24 hours or until the `uploadUrl` endpoint rejects an upload, see [b2_upload_file](crate::simple_client::B2SimpleClient::upload_file).
    pub authorization_token: String,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2ListFileVersionsResponse {
    /// Array of B2 files.
    pub files: Vec<B2File>,
    /// What to pass in to startFileName for the next search to continue where this one left off, or null if there are no more files.
    /// Note this this may not be the name of an actual file, but using it is guaranteed to find the next file version in the bucket.
    pub next_file_name: Option<String>,
    /// What to pass in to startFileId for the next search to continue where this one left off, or null if there are no more files.
    /// Note this this may not be the ID of an actual file, but using it is guaranteed to find the next file version in the bucket.
    pub next_file_id: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2FilePart {
    /// The file ID for uploading this file.
    pub file_id: String,
    /// Which part this is.
    pub part_number: u16,
    /// The number of bytes stored in the part.
    pub content_length: u64,
    /// The SHA1 of the bytes stored in the file as a 40-digit hex string.
    /// Large files do not have SHA1 checksums, and the value is "none". The value is null when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub content_sha1: String,
    /// The MD5 of the bytes stored in the part. Not all parts have an MD5 checksum, so this field is optional, and set to null for parts that do not have one.
    pub content_md5: Option<String>,
    /// When the part is encrypted with [Server-Side Encryption](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api),
    /// the mode ("SSE-B2" or "SSE-C") and algorithm used to encrypt the data.
    pub server_side_encryption: B2ServerSideEncryption,
    /// This is a UTC time when this file was uploaded.
    /// It is a base 10 number of milliseconds since midnight, January 1, 1970 UTC.
    /// This fits in a 64 bit integer such as the type "long" in the programming language Java.
    /// It is intended to be compatible with Java's time long.
    /// For example, it can be passed directly into the java call Date.setTime(long time).
    /// Always 0 when the action is ["folder"](B2Action::Folder).
    pub upload_timestamp: u64,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum B2AuthDataApiInfoType {
    GroupsApi,
    StorageApi,
    BackupApi,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2AuthDataStorageApiInfo {
    pub absolute_minimum_part_size: NonZeroU64,
    pub api_url: String,
    pub bucket_id: Option<String>,
    pub bucket_name: Option<String>,
    pub capabilities: Vec<B2KeyCapability>,
    pub download_url: String,
    pub info_type: B2AuthDataApiInfoType,
    pub name_prefix: Option<String>,
    pub recommended_part_size: NonZeroU64,
    pub s3_api_url: String,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2AuthDataGroupsApiInfo {
    pub capabilities: Vec<String>,
    pub groups_api_url: String,
    pub info_type: B2AuthDataApiInfoType,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2AuthDataBackupApiInfo {
    pub capabilities: Vec<String>,
    pub backup_api_url: String,
    pub info_type: B2AuthDataApiInfoType,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2AuthDataApiInfo {
    // pub groups_api: B2AuthDataGroupsApiInfo,
    /// A data structure that contains the information you need for the B2 Native API.
    pub storage_api: B2AuthDataStorageApiInfo,
    // pub backup_api: B2AuthDataBackupApiInfo,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2AuthData {
    /// The identifier for the account.
    pub account_id: String,
    /// A data structure that groups the information you need by API suite.
    pub api_info: B2AuthDataApiInfo,
    /// An authorization token to use with all calls, other than b2_authorize_account, that need an Authorization header. This authorization token is valid for at most 24 hours.
    pub authorization_token: String,
    /// Expiration timestamp for the application key.
    pub application_key_expiration_timestamp: Option<u64>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2ListUnfinishedLargeFilesResponse {
    /// An array of objects, each one describing one unfinished file.
    pub files: Vec<B2File>,
    /// What to pass in to [`startFileId`](super::query_params::B2ListUnfinishedLargeFilesQueryParameters::start_file_id) for the next search to continue where this one left off, or null if there are no more files.
    /// Note this this may not be the ID of an actual upload, but using it is guaranteed to find the next upload.
    pub next_file_id: Option<String>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2ListPartsResponse {
    /// What to pass in to [`startPartNumber`](super::query_params::B2ListPartsQueryParameters::start_part_number)
    /// for the next search to continue where this one left off, or null if there are no more files.
    /// Note this this may not be the number of an actual part, but using it is guaranteed to find the next file in the bucket.
    pub next_part_number: Vec<u32>,
    /// Array of B2 file parts
    pub parts: Option<B2FilePart>,
}

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2ListKeysResponse {
    /// An array of keys.
    pub keys: Vec<B2AppKey>,
    /// Set if there are more keys beyond the ones that were returned. Pass this value the startApplicationKeyId in the next query to continue listing keys.
    /// <br>Note that this value may not be a valid application key ID, but can still be used as the starting point for the next query.
    pub next_application_key_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2ListBucketsResponse {
    pub buckets: Vec<B2Bucket>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2GetDownloadAuthorizationBodyResponse {
    /// The identifier for the bucket.
    pub bucket_id: String,
    /// The prefix for files the authorization token will allow [b2_download_file_by_name](crate::simple_client::B2SimpleClient::download_file_by_name) to access.
    pub file_name_prefix: String,
    /// The authorization token that can be passed in the Authorization header or as an Authorization parameter to
    /// [b2_download_file_by_name](crate::simple_client::B2SimpleClient::download_file_by_name) to access files beginning with the file name prefix.
    pub authorization_token: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2BucketNotificationRulesResponseBody {
    /// The unique identifier for the bucket containing the event notification rules.
    pub bucket_id: String,
    /// An array containing event notification rules.
    /// <br><br>The event notification rules in this array replace the bucket’s existing rules.
    pub event_notification_rules: Vec<B2EventNotificationRule>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2DeleteFileVersionResponse {
    /// The unique ID of the file version that was deleted.
    pub file_id: String,
    /// The name of the file.
    pub file_name: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2CancelLargeFileResponse {
    /// The ID of the file whose upload that was canceled.
    pub file_id: String,
    /// The account that the bucket is in.
    pub account_id: String,
    /// The unique identifier of the bucket.
    pub bucket_id: String,
    /// The name of the file that was canceled.
    pub file_name: String,
}
