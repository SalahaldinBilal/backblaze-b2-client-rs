use base64::{engine::general_purpose, Engine as _};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Method, RequestBuilder, Response,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::{collections::HashMap, num::NonZeroU16, str::FromStr};

use crate::{
    definitions::{
        bodies::{
            B2CopyFileBody, B2CopyPartBody, B2CreateBucketBody, B2CreateKeyBody,
            B2DeleteFileVersionBody, B2FinishLargeFileBody, B2GetDownloadAuthorizationBody,
            B2ListBucketsBody, B2StartLargeFileUploadBody, B2UpdateBucketBody,
            B2UpdateFileLegalHoldBodyResponse, B2UpdateFileRetentionBody,
        },
        headers::{B2UploadFileHeaders, B2UploadPartHeaders},
        query_params::{
            B2DownloadFileQueryParameters, B2ListFileNamesQueryParameters,
            B2ListFileVersionsQueryParameters, B2ListKeysParameters, B2ListPartsQueryParameters,
            B2ListUnfinishedLargeFilesQueryParameters,
        },
        responses::{
            B2AuthData, B2BucketNotificationRulesResponseBody, B2CancelLargeFileResponse,
            B2DeleteFileVersionResponse, B2FilePart, B2GetDownloadAuthorizationBodyResponse,
            B2GetUploadPartUrlResponse, B2GetUploadUrlResponse, B2ListBucketsResponse,
            B2ListFileVersionsResponse, B2ListFilesResponse, B2ListKeysResponse,
            B2ListPartsResponse, B2ListUnfinishedLargeFilesResponse, B2UpdateFileRetentionResponse,
        },
        shared::{
            B2AppKey, B2Bucket, B2DownloadFileContent, B2Endpoint, B2File, B2FileDownloadDetails,
            B2KeyCapability,
        },
    },
    error::{B2Error, B2RequestError},
    util::{B2FileStream, IntoHeaderMap, WriteLockArc},
};

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

const ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b']')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'\\')
    .add(b'^')
    .add(b'%')
    .add(b'`');

#[derive(Clone, Debug)]
pub struct B2SimpleClient {
    client: reqwest::Client,
    auth_data: WriteLockArc<B2AuthData>,
}

impl B2SimpleClient {
    pub async fn new<S: AsRef<str>, K: AsRef<str>>(
        key_id: S,
        application_key: K,
    ) -> Result<B2SimpleClient, B2Error> {
        let auth_token = format!(
            "Basic {}",
            general_purpose::STANDARD_NO_PAD.encode(format!(
                "{}:{}",
                key_id.as_ref(),
                application_key.as_ref()
            ))
        );

        let client = reqwest::Client::new();

        let auth_response = client
            .get("https://api.backblazeb2.com/b2api/v3/b2_authorize_account")
            .header("Authorization", auth_token)
            .send()
            .await;

        Ok(B2SimpleClient {
            client,
            auth_data: WriteLockArc::new(B2SimpleClient::handle_response(auth_response).await?),
        })
    }

    pub fn auth_data(&self) -> B2AuthData {
        (*self.auth_data).clone()
    }

    pub async fn authorize_account<S: AsRef<str>, K: AsRef<str>>(
        &self,
        key_id: S,
        application_key: K,
    ) -> Result<B2AuthData, B2Error> {
        let auth_token = format!(
            "Basic {}",
            general_purpose::STANDARD_NO_PAD.encode(format!(
                "{}:{}",
                key_id.as_ref(),
                application_key.as_ref()
            ))
        );

        let auth_response = self
            .client
            .get("https://api.backblazeb2.com/b2api/v3/b2_authorize_account")
            .header("Authorization", auth_token)
            .send()
            .await;

        self.auth_data
            .set(B2SimpleClient::handle_response(auth_response).await?)
            .await;
        Ok(self.auth_data())
    }

    /// [b2_cancel_large_file](https://www.backblaze.com/apidocs/b2-cancel-large-file)
    pub async fn cancel_large_file(
        &self,
        file_id: String,
    ) -> Result<B2CancelLargeFileResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2CancelLargeFile)
            .json(&json!({ "fileId": file_id }))
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_copy_file](https://www.backblaze.com/apidocs/b2-copy-file)
    pub async fn copy_file(&self, body: B2CopyFileBody) -> Result<B2File, B2Error> {
        let mut needed_capabilities = vec![B2KeyCapability::WriteFiles];

        if body.file_retention.is_some() {
            needed_capabilities.push(B2KeyCapability::WriteFileRetentions);
        }

        if body.legal_hold.is_some() {
            needed_capabilities.push(B2KeyCapability::WriteFileLegalHolds);
        }

        self.has_capabilities(&needed_capabilities)?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2CopyFile)
            .json(&body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_copy_part](https://www.backblaze.com/apidocs/b2-copy-part)
    pub async fn copy_part(&self, request_body: B2CopyPartBody) -> Result<B2FilePart, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2CopyPart)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_create_bucket](https://www.backblaze.com/apidocs/b2-create-bucket)
    pub async fn create_bucket(&self, body: B2CreateBucketBody) -> Result<B2Bucket, B2Error> {
        let mut needed_capabilities = vec![B2KeyCapability::WriteBuckets];

        if let Some(file_lock_enabled) = body.file_lock_enabled {
            if file_lock_enabled {
                needed_capabilities.push(B2KeyCapability::WriteBucketRetentions);
            }
        }

        if body.default_server_side_encryption.is_some() {
            needed_capabilities.push(B2KeyCapability::WriteBucketEncryption);
        }

        self.has_capabilities(&needed_capabilities)?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2CreateBucket)
            .json(&body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_create_key](https://www.backblaze.com/apidocs/b2-create-key)
    pub async fn create_key(&self, request_body: B2CreateKeyBody) -> Result<B2AppKey, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteKeys])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2CreateKey)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_delete_bucket](https://www.backblaze.com/apidocs/b2-delete-bucket)
    pub async fn delete_bucket(
        &self,
        account_id: String,
        bucket_id: String,
    ) -> Result<B2Bucket, B2Error> {
        self.has_capabilities(&[B2KeyCapability::DeleteBuckets])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2DeleteBucket)
            .json(&json!({ "accountId": account_id, "bucketId": bucket_id }))
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_delete_file_version](https://www.backblaze.com/apidocs/b2-delete-file-version)
    pub async fn delete_file_version(
        &self,
        request_body: B2DeleteFileVersionBody,
    ) -> Result<B2DeleteFileVersionResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::DeleteFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2DeleteFileVersion)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_delete_key](https://www.backblaze.com/apidocs/b2-delete-key)
    pub async fn delete_key(&self, application_key_id: String) -> Result<B2AppKey, B2Error> {
        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2DeleteKey)
            .json(&json!({ "applicationKeyId": application_key_id }))
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_download_file_by_id](https://www.backblaze.com/apidocs/b2-download-file-by-id)
    pub async fn download_file_by_id(
        &self,
        file_id: String,
        request_query_params: Option<B2DownloadFileQueryParameters>,
    ) -> Result<B2DownloadFileContent, B2Error> {
        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2DownloadFileById)
            .query(&[("fileId", file_id)])
            .query(&request_query_params)
            .send()
            .await;

        B2SimpleClient::handle_file_response(response).await
    }

    /// [b2_download_file_by_name](https://www.backblaze.com/apidocs/b2-download-file-by-name)
    pub async fn download_file_by_name(
        &self,
        bucket_name: String,
        file_name: String,
        request_query_params: Option<B2DownloadFileQueryParameters>,
    ) -> Result<B2DownloadFileContent, B2Error> {
        let response = self
            .client
            .get(format!(
                "{}/file/{}/{}",
                self.auth_data.api_info.storage_api.download_url, bucket_name, file_name
            ))
            .header("Authorization", self.get_authorization_token())
            .query(&request_query_params)
            .send()
            .await;

        B2SimpleClient::handle_file_response(response).await
    }

    /// [b2_finish_large_file](https://www.backblaze.com/apidocs/b2-finish-large-file)
    pub async fn finish_large_file(
        &self,
        request_body: B2FinishLargeFileBody,
    ) -> Result<B2File, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2FinishLargeFile)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_get_bucket_notification_rules](https://www.backblaze.com/apidocs/b2-get-bucket-notification-rules)
    pub async fn get_bucket_notification_rules(
        &self,
        bucket_id: String,
    ) -> Result<B2BucketNotificationRulesResponseBody, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ReadBucketNotifications])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2GetBucketNotificationRules)
            .query(&[("bucketId", bucket_id)])
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_get_download_authorization](https://www.backblaze.com/apidocs/b2-get-download-authorization)
    pub async fn get_download_authorization(
        &self,
        request_body: B2GetDownloadAuthorizationBody,
    ) -> Result<B2GetDownloadAuthorizationBodyResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ShareFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2GetDownloadAuthorization)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_get_file_info](https://www.backblaze.com/apidocs/b2-get-file-info)
    pub async fn get_file_info(&self, file_id: String) -> Result<B2File, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ReadFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2GetFileInfo)
            .query(&[("fileId", file_id)])
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_get_upload_part_url](https://www.backblaze.com/apidocs/b2-get-upload-part-url)
    pub async fn get_upload_part_url(
        &self,
        file_id: String,
    ) -> Result<B2GetUploadPartUrlResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2GetUploadPartUrl)
            .query(&[("fileId", file_id)])
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_get_upload_url](https://www.backblaze.com/apidocs/b2-get-upload-url)
    pub async fn get_upload_url(
        &self,
        bucket_id: String,
    ) -> Result<B2GetUploadUrlResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2GetUploadUrl)
            .query(&[("bucketId", bucket_id)])
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_hide_file](https://www.backblaze.com/apidocs/b2-hide-file)
    pub async fn hide_file(&self, bucket_id: String, file_name: String) -> Result<B2File, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2HideFile)
            .json(&json!({ "bucketId": bucket_id, "fileName": file_name }))
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_buckets](https://www.backblaze.com/apidocs/b2-list-buckets)
    pub async fn list_buckets(
        &self,
        request_body: B2ListBucketsBody,
    ) -> Result<B2ListBucketsResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ListBuckets])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2ListBuckets)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_file_names](https://www.backblaze.com/apidocs/b2-list-file-names)
    pub async fn list_file_names(
        &self,
        request_body: B2ListFileNamesQueryParameters,
    ) -> Result<B2ListFilesResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ListFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2ListFileNames)
            .query(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_file_versions](https://www.backblaze.com/apidocs/b2-list-file-versions)
    pub async fn list_file_versions(
        &self,
        request_body: B2ListFileVersionsQueryParameters,
    ) -> Result<B2ListFileVersionsResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ListFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2ListFileVersions)
            .query(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_keys](https://www.backblaze.com/apidocs/b2-list-keys)
    pub async fn list_keys(
        &self,
        request_body: B2ListKeysParameters,
    ) -> Result<B2ListKeysResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ListKeys])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2ListKeys)
            .query(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_parts](https://www.backblaze.com/apidocs/b2-list-parts)
    pub async fn list_parts(
        &self,
        request_body: B2ListPartsQueryParameters,
    ) -> Result<B2ListPartsResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2ListParts)
            .query(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_list_unfinished_large_files](https://www.backblaze.com/apidocs/b2-list-unfinished-large-files)
    pub async fn list_unfinished_large_files(
        &self,
        request_body: B2ListUnfinishedLargeFilesQueryParameters,
    ) -> Result<B2ListUnfinishedLargeFilesResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::ListFiles])?;

        let response = self
            .create_request_with_token(Method::GET, B2Endpoint::B2ListUnfinishedLargeFiles)
            .query(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_set_bucket_notification_rules](https://www.backblaze.com/apidocs/b2-set-bucket-notification-rules)
    pub async fn set_bucket_notification_rules(
        &self,
        request_body: B2BucketNotificationRulesResponseBody,
    ) -> Result<B2BucketNotificationRulesResponseBody, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteBucketNotifications])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2SetBucketNotificationRules)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_start_large_file](https://www.backblaze.com/apidocs/b2-start-large-file)
    pub async fn start_large_file(
        &self,
        request_body: B2StartLargeFileUploadBody,
    ) -> Result<B2File, B2Error> {
        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2StartLargeFile)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_update_bucket](https://www.backblaze.com/apidocs/b2-update-bucket)
    pub async fn update_bucket(
        &self,
        request_body: B2UpdateBucketBody,
    ) -> Result<B2Bucket, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteBuckets])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2UpdateBucket)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_update_file_legal_hold](https://www.backblaze.com/apidocs/b2-update-file-legal-hold)
    pub async fn update_file_legal_hold(
        &self,
        request_body: B2UpdateFileLegalHoldBodyResponse,
    ) -> Result<B2UpdateFileLegalHoldBodyResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFileLegalHolds])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2UpdateFileLegalHold)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_update_file_retention](https://www.backblaze.com/apidocs/b2-update-file-retention)
    pub async fn update_file_retention(
        &self,
        request_body: B2UpdateFileRetentionBody,
    ) -> Result<B2UpdateFileRetentionResponse, B2Error> {
        self.has_capabilities(&[B2KeyCapability::WriteFileRetentions])?;

        let response = self
            .create_request_with_token(Method::POST, B2Endpoint::B2UpdateFileRetention)
            .json(&request_body)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// [b2_upload_file](https://www.backblaze.com/apidocs/b2-upload-file)
    pub async fn upload_file<S: AsRef<str>, F: Into<reqwest::Body>>(
        &self,
        file: F,
        upload_url: S,
        request_headers: B2UploadFileHeaders,
        file_info: Option<HashMap<S, impl AsRef<str>>>,
    ) -> Result<B2File, B2Error> {
        let file_info = match file_info {
            Some(map) => map,
            None => HashMap::new(),
        };

        let file_info: HashMap<_, _> = file_info
            .iter()
            .map(|(key, value)| {
                let key_ref = key.as_ref();
                (
                    format!("X-Bz-Info-{key_ref}"),
                    utf8_percent_encode(value.as_ref(), ENCODE_SET).to_string(),
                )
            })
            .collect();

        let mut request_headers = request_headers;

        request_headers.file_name =
            utf8_percent_encode(&request_headers.file_name, ENCODE_SET).to_string();

        let response = self
            .client
            .request(Method::POST, upload_url.as_ref())
            .headers(request_headers.into_header_map()?)
            .headers(hash_map_to_headers(file_info))
            .body(file)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    /// []()
    pub async fn upload_part<F: Into<reqwest::Body>>(
        &self,
        request_headers: B2UploadPartHeaders,
        part: F,
        upload_url: String,
    ) -> Result<B2FilePart, B2Error> {
        let response = self
            .client
            .request(Method::POST, upload_url)
            .headers(request_headers.into_header_map()?)
            .body(part)
            .send()
            .await;

        B2SimpleClient::handle_response(response).await
    }

    pub fn get_authorization_token(&self) -> &str {
        &self.auth_data.authorization_token
    }

    pub fn has_capability(&self, capability: &B2KeyCapability) -> bool {
        self.auth_data
            .api_info
            .storage_api
            .capabilities
            .contains(capability)
    }

    pub fn has_capabilities(&self, capabilities: &[B2KeyCapability]) -> Result<(), B2Error> {
        for capability in capabilities {
            if !self.has_capability(capability) {
                return Err(B2Error::MissingCapability(capability.clone()));
            }
        }

        Ok(())
    }

    #[inline]
    fn create_request_url(&self, api_name: B2Endpoint) -> String {
        format!(
            "{}/b2api/v3/{}",
            self.auth_data.api_info.storage_api.api_url,
            api_name.to_string()
        )
    }

    #[inline]
    fn create_request_with_token(&self, method: Method, api_name: B2Endpoint) -> RequestBuilder {
        let url = self.create_request_url(api_name);

        self.client
            .request(method, url)
            .header("Authorization", self.get_authorization_token())
    }

    #[inline]
    async fn response_option_handling(
        response: Result<Response, reqwest::Error>,
    ) -> Result<Response, B2Error> {
        let response = match response {
            Ok(resp) => resp,
            Err(error) => {
                return Err(B2Error::RequestSendError(error));
            }
        };

        let response_code = response.status().as_u16();

        if response_code >= 400 {
            let response = match response.bytes().await {
                Ok(text) => text,
                Err(_) => {
                    return Err(B2Error::RequestError(B2RequestError {
                        status: NonZeroU16::new(response_code).expect("Response code cannot be 0"),
                        code: String::from(""),
                        message: Some(String::from("B2Client failed to collect")),
                    }))
                }
            };

            let error_json: B2RequestError = match serde_json::from_slice(&response) {
                Ok(json) => json,
                Err(_) => B2RequestError {
                    status: NonZeroU16::new(response_code).expect("Response code cannot be 0"),
                    code: String::from(""),
                    message: Some(String::from(format!(
                        "B2Client failed to parse response as json, returned string: {}",
                        String::from_utf8_lossy(&response)
                    ))),
                },
            };

            return Err(B2Error::RequestError(error_json));
        };

        Ok(response)
    }

    #[inline]
    async fn handle_response<T: DeserializeOwned>(
        response: Result<Response, reqwest::Error>,
    ) -> Result<T, B2Error> {
        let response = match B2SimpleClient::response_option_handling(response).await {
            Ok(resp) => resp,
            Err(error) => return Err(error),
        };

        let text = response
            .text()
            .await
            .map_err(|err| B2Error::RequestSendError(err))?;

        match serde_json::from_str::<T>(&text) {
            Ok(json) => Ok(json),
            Err(error) => Err(B2Error::JsonParseError(error)),
        }
    }

    #[inline]
    async fn handle_file_response(
        response: Result<Response, reqwest::Error>,
    ) -> Result<B2DownloadFileContent, B2Error> {
        let response = match response {
            Ok(resp) => resp,
            Err(error) => return Err(B2Error::RequestSendError(error)),
        };

        let mut headers = header_map_to_hashmap(response.headers());
        let file_name = headers.remove("x-bz-file-name").expect("should exist");
        let file_name = urlencoding::decode(&file_name.replace("+", " "))
            .expect("valid")
            .to_string();

        let sha1 = headers.remove("x-bz-content-sha1").expect("should exist");

        let mut file_details = B2FileDownloadDetails {
            file_id: headers.remove("x-bz-file-id").expect("should exist"),
            file_name,
            content_length: headers
                .remove("content-length")
                .expect("should exist")
                .parse()
                .expect("valid number"),
            content_type: headers.remove("content-type").expect("should exist"),
            content_sha1: if sha1 != "none" { Some(sha1) } else { None },
            upload_timestamp: headers
                .remove("x-bz-upload-timestamp")
                .expect("should exist")
                .parse()
                .expect("valid number"),
            file_info: None,
        };

        let mut temp_file_info: HashMap<String, String> = HashMap::new();
        let keys: Vec<String> = headers.keys().map(|e| e.clone()).collect();

        for key in keys {
            if key.starts_with("x-bz-info-") {
                let value = headers.remove(&key).expect("key exists");
                let value = urlencoding::decode(&value.replace("+", " "))
                    .expect("valid")
                    .to_string();
                temp_file_info.insert(key.replace("x-bz-info-", ""), value);
            }
        }

        if temp_file_info.len() > 0 {
            file_details.file_info = Some(temp_file_info)
        }

        let body = response.bytes_stream();

        Ok(B2DownloadFileContent {
            file: B2FileStream::new(body, file_details.content_length as usize),
            file_details,
            remaining_headers: headers,
        })
    }
}

#[inline]
fn hash_map_to_headers<S: AsRef<str>>(map: HashMap<S, impl AsRef<str>>) -> HeaderMap {
    map.iter()
        .map(|(name, value)| {
            (
                HeaderName::from_str(name.as_ref()),
                HeaderValue::from_str(value.as_ref()),
            )
        })
        .filter_map(|(key, value)| match (key, value) {
            (Ok(key), Ok(value)) if !value.is_empty() => Some((key, value)),
            _ => None,
        })
        .collect()
}

#[inline]
fn header_map_to_hashmap(map: &HeaderMap) -> HashMap<String, String> {
    let mut header_hashmap = HashMap::new();

    for (k, v) in map {
        let k = k.as_str().to_owned();
        let v = String::from_utf8_lossy(v.as_bytes()).into_owned();
        header_hashmap.insert(k, v);
    }

    header_hashmap
}
