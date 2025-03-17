use serde::{
    de::{self, MapAccess, Visitor},
    ser::SerializeMap,
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{collections::HashMap, fmt};
use strum_macros::Display;

use crate::util::B2FileStream;

#[derive(Debug, Display, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum B2Endpoint {
    B2AuthorizeAccount,
    B2CancelLargeFile,
    B2CopyFile,
    B2CopyPart,
    B2CreateBucket,
    B2CreateKey,
    B2DeleteBucket,
    B2DeleteFileVersion,
    B2DeleteKey,
    B2DownloadFileById,
    B2DownloadFileByName,
    B2FinishLargeFile,
    B2GetBucketNotificationRules,
    B2GetDownloadAuthorization,
    B2GetFileInfo,
    B2GetUploadPartUrl,
    B2GetUploadUrl,
    B2HideFile,
    B2ListBuckets,
    B2ListFileNames,
    B2ListFileVersions,
    B2ListKeys,
    B2ListParts,
    B2ListUnfinishedLargeFiles,
    B2SetBucketNotificationRules,
    B2StartLargeFile,
    B2UpdateBucket,
    B2UpdateFileLegalHold,
    B2UpdateFileRetention,
    B2UploadFile,
    B2UploadPart,
}

#[derive(Debug, Display, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2KeyCapability {
    ListKeys,
    WriteKeys,
    DeleteKeys,
    ListBuckets,
    ListAllBucketNames,
    ReadBuckets,
    WriteBuckets,
    DeleteBuckets,
    ReadBucketRetentions,
    WriteBucketRetentions,
    ReadBucketEncryption,
    WriteBucketEncryption,
    ListFiles,
    ReadFiles,
    ShareFiles,
    WriteFiles,
    DeleteFiles,
    ReadFileLegalHolds,
    WriteFileLegalHolds,
    ReadFileRetentions,
    WriteFileRetentions,
    BypassGovernance,
    ReadBucketReplications,
    WriteBucketReplications,
    WriteBucketNotifications,
    ReadBucketNotifications,
    ReadBucketLogging,
    WriteBucketLogging,
}

#[derive(Debug, Display, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2Action {
    /// file that was uploaded to B2 Cloud Storage.
    Upload,
    /// large file has been started, but not finished or canceled.
    Start,
    /// file version marking the file as hidden, so that it will not show up in b2_list_file_names.
    Hide,
    /// is used to indicate a virtual folder when listing files.
    Folder,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2LifeCycleRules {
    pub days_from_hiding_to_deleting: Option<u32>,
    pub days_from_uploading_to_hiding: Option<u32>,
    pub file_name_prefix: Box<str>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum B2ReplicationStatus {
    Pending,
    Completed,
    Failed,
    Replica,
}

// #[derive(Clone, Deserialize, Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct B2ServerSideEncryption {
//     pub mode: Option<String>,
//     pub algorithm: Option<B2ServerSideEncryptionAlgorithm>,
// }

#[derive(Clone, Debug)]
pub enum B2ServerSideEncryption {
    /// Disable SSC, similar to
    Disabled,
    SseB2 {
        algorithm: B2ServerSideEncryptionAlgorithm,
    },
    SseC {
        algorithm: B2ServerSideEncryptionAlgorithm,
        customer_key: String,
        customer_key_md5: String,
    },
}

impl Serialize for B2ServerSideEncryption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use B2ServerSideEncryption::*;

        match self {
            Disabled => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("mode", &Option::<String>::None)?;
                map.end()
            }
            SseB2 { algorithm } => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("mode", "SSE-B2")?;
                map.serialize_entry("algorithm", algorithm)?;
                map.end()
            }
            SseC {
                algorithm,
                customer_key,
                customer_key_md5,
            } => {
                let mut map = serializer.serialize_map(Some(4))?;
                map.serialize_entry("mode", "SSE-C")?;
                map.serialize_entry("algorithm", algorithm)?;
                map.serialize_entry("customerKey", customer_key)?;
                map.serialize_entry("customerKeyMd5", customer_key_md5)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for B2ServerSideEncryption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct B2ServerSideEncryptionVisitor;

        impl<'de> Visitor<'de> for B2ServerSideEncryptionVisitor {
            type Value = B2ServerSideEncryption;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a B2ServerSideEncryption object")
            }

            fn visit_map<M>(self, mut map: M) -> Result<B2ServerSideEncryption, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut mode: Option<String> = None;
                let mut algorithm = None;
                let mut customer_key = None;
                let mut customer_key_md5 = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        "mode" => mode = map.next_value()?,
                        "algorithm" => algorithm = map.next_value()?,
                        "customerKey" => customer_key = map.next_value()?,
                        "customerKeyMd5" => customer_key_md5 = map.next_value()?,
                        _ => continue,
                    }
                }

                match mode {
                    Some(mode_str) => match mode_str.as_str() {
                        "SSE-B2" => {
                            let algorithm =
                                algorithm.ok_or_else(|| de::Error::missing_field("algorithm"))?;
                            Ok(B2ServerSideEncryption::SseB2 { algorithm })
                        }
                        "SSE-C" => {
                            let algorithm =
                                algorithm.ok_or_else(|| de::Error::missing_field("algorithm"))?;
                            let customer_key = customer_key
                                .ok_or_else(|| de::Error::missing_field("customerKey"))?;
                            let customer_key_md5 = customer_key_md5
                                .ok_or_else(|| de::Error::missing_field("customerKeyMd5"))?;
                            Ok(B2ServerSideEncryption::SseC {
                                algorithm,
                                customer_key,
                                customer_key_md5,
                            })
                        }
                        _ => Err(de::Error::unknown_variant(
                            mode_str.as_str(),
                            &["SSE-B2", "SSE-C"],
                        )),
                    },
                    None => Ok(B2ServerSideEncryption::Disabled),
                }
            }
        }

        deserializer.deserialize_map(B2ServerSideEncryptionVisitor)
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2FileRetentionPeriod {
    pub duration: u64,
    pub unit: String,
}

pub struct B2DownloadFileContent {
    pub file: B2FileStream,
    pub file_details: B2FileDownloadDetails,
    pub remaining_headers: HashMap<String, String>,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct B2BucketRetention {
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<B2FileRetentionPeriod>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2ObjectLockValue {
    pub default_retention: B2BucketRetention,
    pub is_file_lock_enabled: bool,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2BucketFileRetention {
    /// Retention mode
    pub mode: Option<B2FileRetentionMode>,
    /// Timestamp for time in the future, in milliseconds
    pub retain_until_timestamp: Option<u64>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2ObjectLock<T> {
    pub is_client_authorized_to_read: bool,
    pub value: Option<T>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum B2FileAction {
    Start,
    Upload,
    Hide,
    Folder,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2File {
    /// The account that owns the file.
    pub account_id: String,
    // The B2 file action
    pub action: B2Action,
    /// The unique identifier of the bucket.
    pub bucket_id: String,
    /// The number of bytes stored in the file. Only useful when the action is ["upload"](B2Action::Upload).
    /// Always 0 when the action is ["start"](B2Action::Start), ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub content_length: u64,
    /// The SHA1 of the bytes stored in the file as a 40-digit hex string.
    /// Large files do not have SHA1 checksums, and the value is "none". The value is null when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub content_sha1: Option<String>,
    /// The MD5 of the bytes stored in the file as a 32-digit hex string.
    /// Not all files have an MD5 checksum, so this field is optional, and set to null for files that do not have one.
    /// Large files do not have MD5 checksums, and the value is null. The value is also null when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub content_md5: Option<String>,
    /// When the action is ["upload"](B2Action::Upload) or ["start"](B2Action::Start), the MIME type of the file, as specified when the file was uploaded.
    /// For ["hide"](B2Action::Hide) action, always "application/x-bz-hide-marker". For ["folder"](B2Action::Folder) action, always null.
    pub content_type: Option<String>,
    /// The unique identifier for this version of this file.
    /// Used with b2_get_file_info, b2_download_file_by_id, and b2_delete_file_version.
    /// The value is null when for action ["folder"](B2Action::Folder).
    pub file_id: String,
    /// The custom information that was uploaded with the file. This is a JSON object, holding the name/value pairs that were uploaded with the file.
    pub file_info: HashMap<String, String>,
    /// The name of this file, which can be used with b2_download_file_by_name.
    pub file_name: String,
    /// The Object Lock retention settings for this file, if any.
    /// This field is filtered based on application key capabilities; the [`readFileRetentions`](B2KeyCapability::ReadFileRetentions) capability is required to access the value.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api)
    /// for more details on response structure. This field is omitted when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub file_retention: Option<B2ObjectLock<B2BucketFileRetention>>,
    /// The Object Lock legal hold status for this file, if any.
    /// This field is filtered based on application key capabilities; the [`readFileLegalHolds`](B2KeyCapability::ReadFileLegalHolds) capability is required to access the value.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api)
    /// for more details on response structure. This field is omitted when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub legal_hold: Option<B2ObjectLock<B2FileLegalHold>>,
    /// The Replication Status for this file, if any. This field is omitted when the file is not part of a replication rule.
    pub replication_status: Option<B2ReplicationStatus>,
    /// When the file is encrypted with [Server-Side Encryption](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api),
    /// the mode ("SSE-B2" or "SSE-C") and algorithm used to encrypt the data.
    /// If the file is not encrypted with Server-Side Encryption, then both mode and algorithm will be null.
    /// This field is omitted when the action is ["hide"](B2Action::Hide), or ["folder"](B2Action::Folder).
    pub server_side_encryption: Option<B2ServerSideEncryption>,
    /// This is a UTC time when this file was uploaded.
    /// It is a base 10 number of milliseconds since midnight, January 1, 1970 UTC.
    /// This fits in a 64 bit integer such as the type "long" in the programming language Java.
    /// It is intended to be compatible with Java's time long.
    /// For example, it can be passed directly into the java call Date.setTime(long time).
    /// Always 0 when the action is ["folder"](B2Action::Folder).
    pub upload_timestamp: u64,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum B2ServerSideEncryptionAlgorithm {
    AES256,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum B2FileRetentionMode {
    Governance,
    Compliance,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2FileLegalHold {
    On,
    Off,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2BucketType {
    /// Anybody can download the files is the bucket
    AllPublic,
    /// You need an authorization token to download the files is the bucket
    AllPrivate,
    Restricted,
    /// Private bucket containing snapshots created in the Backblaze web UI
    Snapshot,
    Shared,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2BucketTypeUpdate {
    /// Anybody can download the files is the bucket
    AllPublic,
    /// You need an authorization token to download the files is the bucket
    AllPrivate,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2CorsRule {
    pub cors_rule_name: String,
    pub allowed_origins: Vec<String>,
    pub allowed_operations: Vec<B2Endpoint>,
    pub expose_headers: Vec<String>,
    pub max_age_seconds: u32,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2ReplicationRule {
    pub destination_bucket_id: String,
    pub file_name_prefix: String,
    pub include_existing_files: bool,
    pub is_enabled: bool,
    pub priority: u16,
    pub replication_rule_name: String,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2ReplicationConfig {
    #[serde(rename_all = "camelCase")]
    AsReplicationSource {
        replication_rules: Vec<B2ReplicationRule>,
        source_application_key_id: String,
    },
    #[serde(rename_all = "camelCase")]
    AsReplicationDestination {
        source_application_key_id: HashMap<String, String>,
    },
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// At least one of the two keys must be set
pub enum B2BucketOption {
    S3,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Clone, Serialize, Debug, Deserialize)]
/// References https://www.backblaze.com/docs/cloud-storage-event-notifications-reference-guide#:~:text=for%20more%20details.-,event%20types,-Backblaze%20B2%20currently
pub enum B2EventNotificationEventType {
    /// A new object that is uploaded to Backblaze B2 that is not copied or replicated. This does not include multipart objects.
    /// <br> Resolves to `b2:ObjectCreated:Upload`
    #[serde(rename = "b2:ObjectCreated:Upload")]
    ObjectCreatedUpload,
    /// A multipart object that was completed in Backblaze B2 that was not replicated.
    /// <br> Resolves to `b2:ObjectCreated:MultipartUpload`
    #[serde(rename = "b2:ObjectCreated:MultipartUpload")]
    ObjectCreatedMultipartUpload,
    /// A copied object in Backblaze B2.
    /// <br> Resolves to `b2:ObjectCreated:Copy`
    #[serde(rename = "b2:ObjectCreated:Copy")]
    ObjectCreatedCopy,
    /// An object that was replicated in Backblaze B2. This does not include multipart objects. This is the replicated object, and not the source object.
    /// <br> Resolves to `b2:ObjectCreated:Replica`
    #[serde(rename = "b2:ObjectCreated:Replica")]
    ObjectCreatedReplica,
    /// A multipart object that was replicated in Backblaze B2. This is the replicated object, and not the source object.
    /// <br> Resolves to `b2:ObjectCreated:MultipartReplica`
    #[serde(rename = "b2:ObjectCreated:MultipartReplica")]
    ObjectMultipartReplica,
    /// Listens to all object creation events.
    /// <br> Resolves to `b2:ObjectCreated:*`
    #[serde(rename = "b2:ObjectCreated:*")]
    ObjectCreatedAll,
    /// An object that was deleted by user action, such as with an API call or by using the Backblaze web console.
    /// <br> Resolves to `b2:ObjectDeleted:Delete`
    #[serde(rename = "b2:ObjectDeleted:Delete")]
    ObjectDeleted,
    /// An object that was deleted by a Lifecycle Rule.
    /// <br> Resolves to `b2:ObjectDeleted:LifecycleRule`
    #[serde(rename = "b2:ObjectDeleted:LifecycleRule")]
    ObjectDeletedLifecycle,
    /// Listens to all object deletion events.
    /// <br> Resolves to `b2:ObjectCreated:*`
    #[serde(rename = "b2:ObjectDeleted:*")]
    ObjectDeletedAll,
    /// A hide marker that was created by user action, such as with an API call.
    /// <br> Resolves to `b2:HideMarkerCreated:Hide`
    #[serde(rename = "b2:HideMarkerCreated:Hide")]
    HideMarkerCreated,
    /// A hide marker that was created by a Lifecycle Rule.
    /// <br> Resolves to `b2:ObjectCreated:*`
    #[serde(rename = "b2:HideMarkerCreated:LifecycleRule")]
    HideMarkerCreatedLifeCycle,
    /// Listens to all object hide marker creation events.
    /// <br> Resolves to `b2:HideMarkerCreated:*`
    #[serde(rename = "b2:HideMarkerCreated:*")]
    HideMarkerAll,
    /// A multipart upload that was started from the S3-Compatible API with Live Read enabled.
    /// <br> Resolves to `b2:MultipartUploadCreated:LiveRead`
    #[serde(rename = "b2:MultipartUploadCreated:LiveRead")]
    MultiPartUploadCreatedLiveRead,
    /// Listens to all object hide marker creation events.
    /// <br> Resolves to `b2:MultipartUploadCreated:*`
    #[serde(rename = "b2:MultipartUploadCreated:*")]
    MultiPartUploadCreatedAll,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum B2EventNotificationTargetType {
    Webhook,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2EventNotificationRule {
    /// The list of event types for the event notification rule.
    pub event_types: Vec<B2EventNotificationEventType>,
    /// Whether the event notification rule is enabled.
    pub is_enabled: bool,
    /// A name for the event notification rule. The name must be unique among the bucket's notification rules.
    pub name: String,
    /// Specifies which object(s) in the bucket the event notification rule applies to.
    pub object_name_prefix: String,
    /// Whether the event notification rule is suspended.
    pub is_suspended: Option<bool>,
    /// Represents the maximum number of events a user will receive per webhook invocation. The value must be a number between 1 and 50. The default value is 1.
    pub max_events_per_batch: Option<u8>,
    /// A brief description of why the event notification rule was suspended.
    pub suspension_reason: Option<String>,
    /// The target configuration for the event notification rule.
    /// <br><br>This object will always contain the `targetType`` field. Currently, the only valid value for `targetType`` is "webhook."
    /// <br><br>The fields for "webhook" objects are defined below. However, other `targetType`` values and collections of fields will be available in the future.
    pub target_configuration: B2NotificationConfiguration,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2NotificationConfiguration {
    /// The URL for the webhook.
    pub url: String,
    /// The type of the target configuration, currently "webhook" only.
    pub target_type: B2EventNotificationTargetType,
    /// The signing secret for use in verifying the `X-Bz-Event-Notification-Signature``.
    pub hmac_sha256_signing_secret: Option<String>,
    /// When present, additional header name/value pairs to be sent on the webhook invocation.
    pub custom_headers: Option<HashMap<String, String>>,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2FilePart {
    pub file_id: String,
    pub part_number: u32,
    pub content_length: u64,
    pub content_sha1: String,
    pub content_md5: Option<String>,
    pub server_side_encryption: B2ServerSideEncryption,
    pub upload_timestamp: u64,
}

#[derive(Clone, Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2AppKey {
    /// Your account ID.
    pub account_id: String,
    /// The ID of the newly created key.
    pub application_key_id: String,
    /// The secret part of the key. Only returned when creating a new key with [b2_create_key](crate::simple_client::B2SimpleClient::create_key).
    pub application_key: Option<String>,
    /// When present, restricts access to one bucket.
    pub bucket_id: Option<String>,
    /// The list of capabilities this key has.
    pub capabilities: Vec<B2KeyCapability>,
    /// When present, says when this key will expire, in milliseconds since 1970.
    pub expiration_timestamp: Option<u64>,
    /// The name assigned when the key was created.
    pub key_name: String,
    /// When present, restricts access to files whose names start with the prefix.
    pub name_prefix: Option<String>,
    /// When present and set to s3, the key can be used to sign requests to the [S3 Compatible API](https://www.backblaze.com/apidocs/introduction-to-the-s3-compatible-api).
    pub options: Option<Vec<B2BucketOption>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B2Bucket {
    /// Your account ID.
    pub account_id: String,
    /// The unique identifier of the bucket.
    pub bucket_id: String,
    /// The unique name of the bucket.
    pub bucket_name: String,
    /// The bucket type.
    pub bucket_type: B2BucketType,
    /// The user data stored with this bucket.
    pub bucket_info: HashMap<String, String>,
    /// The initial list of CORS rules for this bucket.
    /// See [CORS Rules](https://www.backblaze.com/docs/cloud-storage-cross-origin-resource-sharing-rules) for an overview and the rule structure.
    pub cors_rules: Vec<B2CorsRule>,
    /// The Object Lock configuration for this bucket.
    /// This field is filtered based on application key capabilities; the [`readBucketRetentions`](B2KeyCapability::ReadBucketRetentions) capability is required to access the value.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for more details on response structure.
    pub file_lock_configuration: B2ObjectLock<B2BucketFileRetention>,
    /// The default bucket Server-Side Encryption settings for new files uploaded to this bucket.
    /// This field is filtered based on application key capabilities; the [`readBucketEncryption`](B2KeyCapability::ReadBucketEncryption) capability is required to access the value.
    /// See [ Server-Side Encryption](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api) for more details on response structure
    pub default_server_side_encryption: B2ServerSideEncryption,
    /// The initial list of lifecycle rules for this bucket.
    /// See [Lifecycle Rules](https://www.backblaze.com/docs/cloud-storage-lifecycle-rules) for an overview and the rule structure.
    pub life_cycle_rules: Option<Vec<B2LifeCycleRules>>,
    /// The list of replication rules for this bucket. See [Cloud Replication](https://www.backblaze.com/docs/cloud-storage-create-a-cloud-replication-rule-with-the-native-api) Rules.
    /// <br><br> NOTE: The first time that you configure Cloud Replication, complete the following tasks to ensure that you have the correct permission:
    /// - Verify your email address.
    /// - Have a payment history on file or make a payment.
    pub replication_configuration: B2ReplicationConfig,
    /// A counter that is updated every time the bucket is modified,
    /// and can be used with the [`ifRevisionIs`](super::bodies::B2UpdateBucketBody::if_revision_is) parameter to b2_update_bucket to prevent colliding, simultaneous updates.
    pub revision: u32,
    /// When present and set to s3, the bucket can be accessed through the [`S3 Compatible API`](https://www.backblaze.com/apidocs/introduction-to-the-s3-compatible-api).
    pub options: Option<Vec<B2BucketOption>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2CustomerAgnosticServerSideEncryption {
    pub customer_key: String,
    pub customer_key_md5: String,
    #[serde(flatten)]
    pub server_side_encryption: B2ServerSideEncryption,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum B2MetadataDirective {
    Copy,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct B2FileDownloadDetails {
    pub content_length: u64,
    pub content_type: String,
    pub file_id: String,
    pub file_name: String,
    pub content_sha1: Option<String>,
    pub upload_timestamp: u64,
    pub file_info: Option<HashMap<String, String>>,
}
