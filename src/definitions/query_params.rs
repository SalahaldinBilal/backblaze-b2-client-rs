use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListUnfinishedLargeFilesQueryParameters {
    #[builder(!default)]
    /// The bucket to look for file names in.
    pub bucket_id: String,
    /// When a `namePrefix` is provided, only files whose names match the prefix will be returned.
    /// Whe using an application key that is restricted to a name prefix, you must provide a prefix here that is at least as restrictive.
    pub name_prefix: Option<String>,
    /// The first upload to return. If there is an upload with this ID, it will be returned in the list. If not, the first upload after this the first one after this ID.
    pub start_file_id: Option<String>,
    /// The maximum number of files to return from this call. The default value is 100, and the maximum allowed is 100.
    pub max_file_count: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListPartsQueryParameters {
    #[builder(!default)]
    /// The ID returned by [b2_start_large_file](crate::simple_client::B2SimpleClient::start_large_file).
    /// This is the file whose parts will be listed.
    pub file_id: String,
    /// The first part to return. If there is a part with this number, it will be returned as the first in the list.
    /// If not, the returned list will start with the first part number after this one.
    pub start_part_number: Option<u32>,
    /// The maximum number of parts to return from this call. The default value is 100, and the maximum allowed is 1000.
    pub max_part_count: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListKeysParameters {
    #[builder(!default)]
    /// The ID of your account.
    pub account_id: String,
    /// The maximum number of files to return from this call. The default value is 100, and the maximum is 10000. Passing in 0 means to use the default of 100.
    /// <br><br><Br>
    /// NOTE: [b2_list_keys](crate::simple_client::B2SimpleClient::list_keys) is a Class C transaction (see [Pricing](https://www.backblaze.com/b2/cloud-storage-pricing.html)).
    /// The maximum number of files returned per transaction is 1000. If you set maxFileCount to more than 1000 and more than 1000 are returned,
    /// the call will be billed as multiple transactions, as if you had made requests in a loop asking for 1000 at a time. For example:
    /// if you set maxFileCount to 10000 and 3123 items are returned, you will be billed for 4 Class C transactions.
    pub max_key_count: Option<u16>,
    /// The first key to return. Used when a query hits the maxKeyCount, and you want to get more.
    /// Set to the value returned as the nextApplicationKeyId in the previous query.
    pub start_application_key_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListFileVersionsQueryParameters {
    #[builder(!default)]
    /// The bucket to look for file names in.
    pub bucket_id: String,
    /// The first file name to return.
    /// <br> If there are no files with this name, the first version of the file with the first name after the given name will be the first in the list.
    /// <br> If startFileId is also specified, the name-and-id pair is the starting point. If there is a file with the given name and ID, it will be first in the list.
    /// Otherwise, the first file version that comes after the given name and ID will be first in the list.
    pub start_file_name: Option<String>,
    /// The first file ID to return. `startFileName` must also be provided if `startFileId` is specified. (See [startFileName](B2ListFileVersionsQueryParameters::start_file_name))
    pub start_file_id: Option<String>,
    /// The maximum number of files to return from this call. The default value is 100, and the maximum is 10000. Passing in 0 means to use the default of 100.
    /// <br><br><Br>
    /// NOTE: [b2_list_file_versions](crate::simple_client::B2SimpleClient::list_file_versions) is a Class C transaction (see [Pricing](https://www.backblaze.com/b2/cloud-storage-pricing.html)).
    /// The maximum number of files returned per transaction is 1000. If you set maxFileCount to more than 1000 and more than 1000 are returned,
    /// the call will be billed as multiple transactions, as if you had made requests in a loop asking for 1000 at a time. For example:
    /// if you set maxFileCount to 10000 and 3123 items are returned, you will be billed for 4 Class C transactions.
    pub max_file_count: Option<NonZeroU32>,
    /// Files returned will be limited to those with the given prefix. Defaults to the empty string, which matches all files.
    pub prefix: Option<String>,
    /// Files returned will be limited to those within the top folder, or any one subfolder. Defaults to NULL.
    /// Folder names will also be returned. The delimiter character will be used to "break" file names into folders.
    pub delimiter: Option<String>,
}

#[derive(Clone, Debug, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2ListFileNamesQueryParameters {
    #[builder(!default)]
    /// The bucket to look for file names in. Returned by [b2_list_buckets](crate::simple_client::B2SimpleClient::list_buckets).
    pub bucket_id: String,
    /// The first file name to return. If there is a file with this name, it will be returned in the list. If not, the first file name after this the first one after this name.
    pub start_file_name: Option<String>,
    /// The maximum number of files to return from this call. The default value is 100, and the maximum is 10000. Passing in 0 means to use the default of 100.
    /// <br><br><Br>
    /// NOTE: [b2_list_file_names](crate::simple_client::B2SimpleClient::list_file_names) is a Class C transaction (see [Pricing](https://www.backblaze.com/b2/cloud-storage-pricing.html)).
    /// The maximum number of files returned per transaction is 1000. If you set maxFileCount to more than 1000 and more than 1000 are returned,
    /// the call will be billed as multiple transactions, as if you had made requests in a loop asking for 1000 at a time. For example:
    /// if you set maxFileCount to 10000 and 3123 items are returned, you will be billed for 4 Class C transactions.
    pub max_file_count: Option<NonZeroU32>,
    /// Files returned will be limited to those with the given prefix. Defaults to the empty string, which matches all files.
    pub prefix: Option<String>,
    /// Files returned will be limited to those within the top folder, or any one subfolder. Defaults to NULL.
    /// Folder names will also be returned. The delimiter character will be used to "break" file names into folders.
    pub delimiter: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
#[builder(field_defaults(default))]
pub struct B2DownloadFileQueryParameters {
    /// If this is present, B2 will use it as the value of the 'Content-Disposition' header, overriding any 'b2-content-disposition' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 6266. Parameter continuations are not supported.
    /// 'Extended-value's are supported for charset 'UTF-8' (case-insensitive) when the language is empty.
    /// Note that this file info will not be included in downloads as a x-bz-info-b2-content-disposition header.
    /// Instead, it (or the value specified in a request) will be in the Content-Disposition.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_content_disposition: Option<String>,
    /// If this is present, B2 will use it as the value of the 'Content-Language' header, overriding any 'b2-content-language' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 2616. Note that this file info will not be included in downloads as a x-bz-info-b2-content-language header.
    /// Instead, it (or the value specified in a request) will be in the Content-Language.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_content_language: Option<String>,
    /// If this is present, B2 will use it as the value of the 'Expires' header, overriding any 'b2-expires' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 2616. Note that this file info will not be included in downloads as a x-bz-info-b2-expires header.
    /// Instead, it (or the value specified in a request) will be in the Expires.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_expires: Option<String>,
    /// If this is present, B2 will use it as the value of the 'Cache-Control' header, overriding any 'b2-cache-control' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 2616. Note that this file info will not be included in downloads as a x-bz-info-b2-cache-control header.
    /// Instead, it (or the value specified in a request) will be in the Cache-Control.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_cache_control: Option<String>,
    /// If this is present, B2 will use it as the value of the 'Content-Encoding' header, overriding any 'b2-content-encoding' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 2616. Note that this file info will not be included in downloads as a x-bz-info-b2-content-encoding header.
    /// Instead, it (or the value specified in a request) will be in the Content-Encoding.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_content_encoding: Option<String>,
    /// If this is present, B2 will use it as the value of the 'Content-Type' header, overriding any 'Content-Type' specified when the file was uploaded.
    /// <br><br>The value must match the grammar specified in RFC 2616. Note that this file info will not be included in downloads as a x-bz-info-b2-content-type header.
    /// Instead, it (or the value specified in a request) will be in the Content-Type.
    /// <br><br>If including this header in the response exceeds the 7000-byte overall limit (2048 bytes for files encrypted with SSE), this request will be rejected.
    /// See [Files](https://www.backblaze.com/docs/cloud-storage-files) for further details about HTTP header size limit.
    /// <br><br>Requests with this specified must also have an authorization token.
    pub b2_content_type: Option<String>,
}
