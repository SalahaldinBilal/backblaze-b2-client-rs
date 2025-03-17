use std::collections::HashMap;

use serde::{Deserialize, Serialize, Serializer};
use serde_with::skip_serializing_none;
use typed_builder::TypedBuilder;

use super::shared::{
    B2BucketFileRetention, B2BucketRetention, B2BucketType, B2BucketTypeUpdate, B2CorsRule,
    B2CustomerAgnosticServerSideEncryption, B2FileLegalHold, B2KeyCapability, B2LifeCycleRules,
    B2MetadataDirective, B2ReplicationConfig, B2ServerSideEncryption,
};

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2CopyFileBody {
    #[builder(!default)]
    /// The ID of the source file being copied.
    pub source_file_id: String,
    /// The ID of the bucket where the copied file will be stored. If this is not set, the copied file will be added to the same bucket as the source file.
    /// <br> Note that the bucket containing the source file and the destination bucket must belong to the same account.
    pub large_file_id: Option<String>,
    #[builder(!default)]
    /// The name of the new file being created.
    pub file_name: String,
    /// The range of bytes to copy. If not provided, the whole source file will be copied.
    pub range: Option<String>,
    /// The strategy for how to populate metadata for the new file.
    /// <br> If COPY is the indicated strategy, then supplying the contentType or fileInfo param is an error.
    pub metadata_directive: Option<B2MetadataDirective>,
    /// Must only be supplied if the metadataDirective is REPLACE.
    /// <br> The MIME type of the content of the file, which will be returned in the Content-Type header when downloading the file.
    /// <br> Use the Content-Type b2/x-auto to automatically set the stored Content-Type post upload.
    /// <br> In the case where a file extension is absent or the lookup fails, the Content-Type is set to application/octet-stream. The Content-Type mappings can be perused [here](https://www.backblaze.com/docs/cloud-storage-b2-content-type-mappings).
    pub content_type: Option<String>,
    /// Must only be supplied if the metadataDirective is REPLACE.
    /// <br> This field stores the metadata that will be stored with the file. It follows the same rules that are applied to [b2_upload_file](super::headers::B2UploadFileHeaders).
    pub file_info: Option<HashMap<String, String>>,
    /// If present, specifies the Object Lock retention settings for the new file.
    /// <br> Setting the value requires the [writeFileRetentions](B2KeyCapability::WriteFileRetentions) capability and that the destination bucket is Object Lock-enabled. See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub file_retention: Option<B2BucketFileRetention>,
    /// If present, specifies the Object Lock legal hold status for the new file.
    /// <br> Setting the value requires the [writeFileLegalHolds](B2KeyCapability::WriteFileLegalHolds) capability and that the destination bucket is Object Lock-enabled. [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub legal_hold: Option<B2FileLegalHold>,
    /// If present, specifies the parameters for Backblaze B2 to use for accessing the source file data using Server-Side Encryption.
    /// <Br> This parameter is required if and only if the source file has been encrypted using [Server-Side Encryption with Customer-Managed Keys (SSE-C)](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api#enable-and-manage-ssec),
    /// and the provided encryption key must match the one with which the source file was encrypted. See [Server-Side Encryption][https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api] for details.
    pub source_server_side_encryption: Option<B2CustomerAgnosticServerSideEncryption>,
    /// If present, specifies the parameters for Backblaze B2 to use for encrypting the copied data before storing the destination file using Server-Side Encryption. See [Server-Side Encryption][https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api] for details.
    pub destination_server_side_encryption: Option<B2CustomerAgnosticServerSideEncryption>,
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2CopyPartBody {
    /// The ID of the source file being copied.
    pub source_file_id: String,
    /// The ID of the large file the part will belong to, as returned by [b2_start_large_file](super::responses::B2StartLargeFileResponse::file_id).
    pub large_file_id: String,
    /// A number from 1 to 10000. The parts uploaded for one file must have contiguous numbers, starting with 1.
    pub part_number: u16,
    #[builder(default)]
    /// The range of bytes to copy. If not provided, the whole source file will be copied.
    pub range: Option<String>,
    #[builder(default)]
    /// If present, specifies the parameters for Backblaze B2 to use for accessing the source file data using Server-Side Encryption.
    /// <Br> This parameter must be provided only if the source file has been encrypted using [Server-Side Encryption with Customer-Managed Keys (SSE-C)](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api#enable-and-manage-ssec),
    /// and the provided encryption key must match the one with which the source file was encrypted. See [Server-Side Encryption][https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api] for details.
    pub source_server_side_encryption: Option<B2CustomerAgnosticServerSideEncryption>,
    #[builder(default)]
    /// If present, specifies the parameters for Backblaze B2 to use for encrypting the copied data before storing the destination file using Server-Side Encryption
    /// <Br> This parameter must be provided only if the large file was started with [Server-Side Encryption with Customer-Managed Keys (SSE-C)](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api#enable-and-manage-ssec),
    /// and the provided encryption key must match the one with which the large file was started. See [Server-Side Encryption][https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api] for details.
    pub destination_server_side_encryption: Option<B2CustomerAgnosticServerSideEncryption>,
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2CreateBucketBody {
    #[builder(!default)]
    /// Your account ID.
    pub account_id: String,
    /// The name to give the new bucket.
    /// <br> Bucket names must be a minimum of 6 and a maximum of 63 characters long, and must be globally unique, two different B2 accounts cannot have buckets with the same name.
    /// Bucket names can consist of letters, digits, and "-". Bucket names cannot start with "b2-", these are reserved for internal Backblaze use.
    #[builder(!default)]
    pub bucket_name: String,
    /// Either ["allPublic"](B2BucketType::AllPublic), meaning that files in this bucket can be downloaded by anybody,
    /// or ["allPrivate"](B2BucketType::AllPrivate), meaning that you need a bucket authorization token to download the files.
    #[builder(!default)]
    pub bucket_type: B2BucketType,
    /// User-defined information to be stored with the bucket as a JSON object mapping names to values. See [Buckets](https://www.backblaze.com/docs/cloud-storage-buckets).
    /// <br> Cache-Control policies can be set here on a global level for all the files in the bucket.
    pub bucket_info: Option<HashMap<String, String>>,
    /// The initial list of CORS rules for this bucket.
    /// See [CORS Rules](https://www.backblaze.com/docs/cloud-storage-cross-origin-resource-sharing-rules) for an overview and the rule structure.
    pub cors_rules: Option<Vec<B2CorsRule>>,
    /// If present, the boolean value specifies whether bucket is Object Lock-enabled.
    /// <br> The default value is false. Setting the value to true requires the [writeBucketRetentions](super::shared::B2KeyCapability::WriteFileRetentions) capability.
    pub file_lock_enabled: Option<bool>,
    /// The initial list of lifecycle rules for this bucket. See [Lifecycle Rules](https://www.backblaze.com/docs/cloud-storage-lifecycle-rules).
    pub life_cycle_rules: Option<Vec<B2LifeCycleRules>>,
    /// The configuration to create a Replication Rule. See [Cloud Replication](https://www.backblaze.com/docs/cloud-storage-create-a-cloud-replication-rule-with-the-native-api) Rules.
    ///  At least one of the [`asReplicationSource`](B2ReplicationConfig::AsReplicationSource) or [`asReplicationDestination`](B2ReplicationConfig::AsReplicationDestination) parameters is required, but they can also both be present.
    /// <br><br> NOTE: The first time that you configure Cloud Replication, complete the following tasks to ensure that you have the correct permission:
    /// - Verify your email address.
    /// - Have a payment history on file or make a payment.
    pub replication_configuration: Option<B2ReplicationConfig>,
    /// The default server-side encryption settings for this bucket. See Server-Side Encryption for an overview and the parameter structure.
    /// <br> Setting the value requires the [`writeBucketEncryption`](B2KeyCapability::WriteBucketEncryption) application key capability.
    pub default_server_side_encryption: Option<B2ServerSideEncryption>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2UpdateFileRetentionBody {
    /// The name of the file.
    pub file_name: String,
    /// The ID of the file.
    pub file_id: String,
    /// Specifies the file retention settings for Backblaze B2 to use for this file.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub file_retention: B2BucketFileRetention,
    #[builder(default)]
    /// Must be specified and set to true if deleting an existing governance mode retention setting or shortening an existing governance mode retention period.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub bypass_governance: Option<bool>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2FinishLargeFileBody {
    /// The ID returned by [b2_start_large_file](crate::simple_client::B2SimpleClient::start_large_file).
    pub file_id: String,
    /// An array of hex SHA1 checksums of the parts of the large file. This is a double-check that the right parts were uploaded in the right order, and that none were missed.
    /// Note that the part numbers start at 1, and the SHA1 of the part 1 is the first string in the array, at index 0.
    pub part_sha1_array: Vec<String>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2StartLargeFileUploadBody {
    #[builder(!default)]
    /// The ID of the bucket that the file will go in.
    pub bucket_id: String,
    #[builder(!default)]
    /// The name of the file. See [Files](https://www.backblaze.com/docs/cloud-storage-files) for requirements on file names.
    pub file_name: String,
    #[builder(!default)]
    /// The MIME type of the content of the file, which will be returned in the `Content-Type` header when downloading the file.
    /// Use the Content-Type `b2/x-auto` to automatically set the stored `Content-Type` post upload.
    /// In the case where a file extension is absent or the lookup fails, the `Content-Type` is set to `application/octet-stream`.
    /// The `Content-Type` mappings can be perused [here](https://www.backblaze.com/docs/cloud-storage-b2-content-type-mappings).
    pub content_type: String,
    /// If this is present, B2 will use it as the value of the upload timestamp.
    /// The value should be a base 10 number that represents a UTC time when the original source file was uploaded.
    /// It is a base 10 number of milliseconds since midnight, January 1st, 1970 UTC. This fits in a 64-bit integer, such as the type long in Java,
    /// and so it can be passed directly into the Java call `Date.setTime(long time)`. The value must not use a date that is set to a time in the future.
    /// If the value is null, it will be ignored.
    /// <br><br> Note: The timestamp should not interfere with the bucket lifecycle rules.
    /// <br><br> For example, a conflict between the timestamp and a lifecycle rule, such as `daysFromStartingToCancelingUnfinishedLargeFiles`,
    /// may also result in a large file upload being canceled.
    pub custom_upload_timestamp: Option<u64>,
    /// A JSON object holding the name/value pairs for the custom file info.
    pub file_info: Option<HashMap<String, String>>,
    /// If present, specifies the Object Lock retention settings for the new file.
    /// <br> Setting the value requires the [writeFileRetentions](B2KeyCapability::WriteFileRetentions) capability and that the destination bucket is Object Lock-enabled.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub file_retention: Option<B2BucketFileRetention>,
    /// If present, specifies the Object Lock legal hold status for the new file.
    /// <br> Setting the value requires the [writeFileLegalHolds](B2KeyCapability::WriteFileLegalHolds) capability and that the destination bucket is Object Lock-enabled.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for details.
    pub legal_hold: Option<B2FileLegalHold>,
    /// If present, specifies the parameters for Backblaze B2 to use for encrypting the uploaded data before storing the file using Server-Side Encryption.
    /// See [`Server-Side Encryption`](https://www.backblaze.com/docs/cloud-storage-enable-server-side-encryption-with-the-native-api) for details.
    pub server_side_encryption: Option<B2ServerSideEncryption>,
}

/// The api for the b2_update_file_legal_hold endpoint returns same
/// schema as the passed body, so we can just reuse this struct for both
#[derive(Clone, Debug, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2UpdateFileLegalHoldBodyResponse {
    /// The name of the file.
    pub file_id: String,
    /// The ID of the file.
    pub file_name: String,
    /// The legal hold on this file.
    pub legal_hold: B2FileLegalHold,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2UpdateBucketBody {
    #[builder(!default)]
    /// The account that the bucket is in.
    pub account_id: String,
    #[builder(!default)]
    /// The unique identifier of the bucket.
    pub bucket_id: String,
    /// The bucket type, If not specified, the setting will remain unchanged
    pub bucket_type: Option<B2BucketTypeUpdate>,
    /// User-defined information to be stored with the bucket as a JSON object mapping names to values. See [Buckets](https://www.backblaze.com/docs/cloud-storage-buckets).
    /// <br> Cache-Control policies can be set here on a global level for all the files in the bucket.
    pub bucket_info: Option<HashMap<String, String>>,
    /// The initial list of CORS rules for this bucket.
    /// See [CORS Rules](https://www.backblaze.com/docs/cloud-storage-cross-origin-resource-sharing-rules) for an overview and the rule structure.
    pub cors_rules: Option<Vec<B2CorsRule>>,
    /// The default Object Lock retention settings for this bucket. See Object Lock for an overview and the parameter structure.
    /// <br><br> If specified, the existing default bucket retention settings will be replaced with the new settings.
    /// If not specified, the setting will remain unchanged. Setting the value requires the [writeBucketRetentions](super::shared::B2KeyCapability::WriteFileRetentions) capability and that the bucket is Object Lock-enabled.
    pub default_retention: Option<B2BucketRetention>,
    /// The default server-side encryption settings for this bucket. See Server-Side Encryption for an overview and the parameter structure.
    /// <br> Setting the value requires the [`writeBucketEncryption`](B2KeyCapability::WriteBucketEncryption) application key capability.
    pub default_server_side_encryption: Option<B2ServerSideEncryption>,
    /// If present, the boolean value specifies whether bucket is [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) enabled.
    /// Once Object Lock is enabled on a bucket, it cannot be disabled.
    /// <br><br> A value of true will be accepted if you have [`writeBucketRetentions`](B2KeyCapability::WriteBucketRetentions) capability.
    /// But you cannot enable Object Lock on a restricted bucket (e.g. share buckets, snapshot) or on a bucket that contains source replication configuration.
    /// <br><br> A value of false will only be accepted if the bucket does not have Object Lock enabled.
    pub file_lock_enabled: Option<bool>,
    /// The initial list of lifecycle rules for this bucket. See [Lifecycle Rules](https://www.backblaze.com/docs/cloud-storage-lifecycle-rules).\
    /// <br><br> If specified, the existing lifecycle rules will be replaced with this new list. If not specified, the setting will remain unchanged.
    pub life_cycle_rules: Option<Vec<B2LifeCycleRules>>,
    /// The configuration to create a Replication Rule. See [Cloud Replication](https://www.backblaze.com/docs/cloud-storage-create-a-cloud-replication-rule-with-the-native-api) Rules.
    ///  At least one of the [`asReplicationSource`](B2ReplicationConfig::AsReplicationSource) or [`asReplicationDestination`](B2ReplicationConfig::AsReplicationDestination) parameters is required, but they can also both be present.
    /// <br><br> NOTE: The first time that you configure Cloud Replication, complete the following tasks to ensure that you have the correct permission:
    /// - Verify your email address.
    /// - Have a payment history on file or make a payment.
    pub replication_configuration: Option<B2ReplicationConfig>,
    /// When set, the update will only happen if the revision number stored in the B2 service matches the one passed in.
    /// This can be used to avoid having simultaneous updates make conflicting changes.
    pub if_revision_is: Option<u32>,
}

#[derive(Clone, Debug)]
pub enum B2BucketTypeList {
    All,
    Types(Vec<B2BucketType>),
}

impl Serialize for B2BucketTypeList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use B2BucketTypeList::*;

        match self {
            All => {
                use serde::ser::SerializeSeq;
                let mut seq = serializer.serialize_seq(Some(1))?;
                seq.serialize_element("all")?;
                seq.end()
            }
            Types(types) => {
                if types.is_empty() {
                    use serde::ser::Error;
                    return Err(S::Error::custom(
                        "B2BucketTypeList Types array cannot be empty",
                    ));
                }
                types.serialize(serializer)
            }
        }
    }
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListBucketsBody {
    #[builder(!default)]
    /// Your account ID.
    pub account_id: String,
    /// When `bucketId` is specified, the result will be a list containing just this bucket, if it's present in the account,
    /// or no buckets if the account does not have a bucket with this ID.
    pub bucket_id: Option<String>,
    /// When `bucketName`` is specified, the result will be a list containing just this bucket, if it's present in the account,
    /// or no buckets if the account does not have a bucket with this name.
    pub bucket_name: Option<String>,
    /// If present, this will be used as a filter for bucket types returned in the list buckets response. If not present, only buckets with bucket types "allPublic", "allPrivate" and "snapshot" will be returned. A special filter value of ["all"] will return all bucket types.
    /// <br><br>If present, it must be in the form of a json array of strings containing valid bucket types in quotes and separated by a comma. Valid bucket types include "allPrivate", "allPublic", "restricted", "snapshot", "shared", and other values added in the future.
    /// <br><br>A bad request error will be returned if "all" is used with other bucketTypes, bucketTypes is empty, or invalid bucketTypes are requested.
    pub bucket_types: Option<Vec<B2BucketTypeList>>,
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2GetDownloadAuthorizationBody {
    #[builder(!default)]
    /// The identifier for the bucket.
    pub bucket_id: String,
    #[builder(!default)]
    /// The file name prefix of files the download authorization token will allow b2_download_file_by_name to access.
    /// For example, if you have a private bucket named `"photos"` and generate a download authorization token for the fileNamePrefix `"pets/"`
    /// you will be able to use the download authorization token to access:
    /// <br><br> https://f345.backblazeb2.com/file/photos/pets/kitten.jpg but not https://f345.backblazeb2.com/file/photos/vacation.jpg.
    pub file_name_prefix: String,
    #[builder(!default)]
    /// The number of seconds before the authorization token will expire. The minimum value is 1 second. The maximum value is 604800 which is one week in seconds.
    pub valid_duration_in_seconds: u64,
    /// If this is present, download requests using the returned authorization must include the same value for b2ContentDisposition.
    /// The value must match the grammar specified in RFC 6266 (except that parameter names that contain an '*' are not allowed).
    pub b2_content_disposition: Option<String>,
    /// If this is present, download requests using the returned authorization must include the same value for b2ContentLanguage.
    /// The value must match the grammar specified in RFC 2616.
    pub b2_content_language: Option<String>,
    /// If this is present, download requests using the returned authorization must include the same value for b2Expires.
    /// The value must match the grammar specified in RFC 2616.
    pub b2_expires: Option<String>,
    /// If this is present, download requests using the returned authorization must include the same value for b2CacheControl.
    /// The value must match the grammar specified in RFC 2616.
    pub b2_cache_control: Option<String>,
    /// If this is present, download requests using the returned authorization must include the same value for b2ContentEncoding.
    /// The value must match the grammar specified in RFC 2616.
    pub b2_content_encoding: Option<String>,
    /// If this is present, download requests using the returned authorization must include the same value for b2ContentType.
    /// The value must match the grammar specified in RFC 2616.
    pub b2_content_type: Option<String>,
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2DeleteFileVersionBody {
    /// The name of the file.
    pub file_name: String,
    /// The ID of the file, as returned by
    /// [b2_upload_file](crate::simple_client::B2SimpleClient::upload_file), [b2_list_file_names](crate::simple_client::B2SimpleClient::list_file_names),
    /// or [b2_list_file_versions](crate::simple_client::B2SimpleClient::list_file_versions).
    pub file_id: String,
    #[builder(default)]
    /// Must be specified and set to `true` if deleting a file version protected by Object Lock governance mode retention settings.
    /// Setting the value requires the [bypassGovernance](B2KeyCapability::BypassGovernance) application key capability.
    /// See [Object Lock](https://www.backblaze.com/docs/cloud-storage-enable-object-lock-with-the-native-api) for more information.
    pub bypass_governance: Option<bool>,
}

#[derive(Clone, Serialize, Debug, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct B2CreateKeyBody {
    /// Your account ID.
    pub account_id: String,
    /// The list of capabilities this key should have.
    pub capabilities: Vec<B2KeyCapability>,
    /// The name for this key. There is no requirement for the key name to be unique.
    /// Key names are limited to 100 characters and can contain letters, numbers, and "-", but not I18N characters, such as é, à, and ü.
    pub key_name: String,
    #[builder(default)]
    /// When provided, the key will expire after the given number of seconds, and will have expirationTimestamp set.
    /// Value must be a positive integer, and must be less than 1000 days (in seconds).
    pub valid_duration_in_seconds: Option<u64>,
    #[builder(default)]
    /// When provided, the new key can only access the specified bucket. Only the following capabilities can be specified:
    /// [listAllBucketNames](B2KeyCapability::ListAllBucketNames), [listBuckets](B2KeyCapability::ListBuckets), [readBuckets](B2KeyCapability::ReadBuckets),
    /// [readBucketEncryption](B2KeyCapability::ReadBucketEncryption), [writeBucketNotifications](B2KeyCapability::WriteBucketEncryption),
    /// [readBucketNotifications](B2KeyCapability::ReadBucketNotifications), [writeBucketEncryption](B2KeyCapability::WriteBucketEncryption),
    /// [readBucketRetentions](B2KeyCapability::ReadBucketRetentions), [writeBucketRetentions](B2KeyCapability::WriteBucketRetentions),
    /// [listFiles](B2KeyCapability::ListFiles), [readFiles](B2KeyCapability::ReadFiles), [shareFiles](B2KeyCapability::ShareFiles),
    /// [writeFiles](B2KeyCapability::WriteFiles), [deleteFiles](B2KeyCapability::DeleteFiles), [readFileLegalHolds](B2KeyCapability::ReadFileLegalHolds),
    /// [writeFileLegalHolds](B2KeyCapability::WriteFileLegalHolds), [readFileRetentions](B2KeyCapability::ReadFileRetentions),
    /// [writeFileRetentions](B2KeyCapability::WriteFileRetentions), and [bypassGovernance](B2KeyCapability::BypassGovernance).
    /// <br><br> For all buckets, this field can either be left empty or set to null.
    pub bucket_id: Option<String>,
    #[builder(default)]
    /// When provided, this parameter limits access to files with names starting with the specified prefix.
    /// By default, the restriction is applied to all buckets unless a [bucketId](B2CreateKeyBody::bucket_id) is included in the request.
    pub name_prefix: Option<String>,
}
