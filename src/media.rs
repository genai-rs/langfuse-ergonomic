//! Media: upload images, audio and files to Langfuse and reference them from inputs, outputs and
//! metadata.
//!
//! [`LangfuseClient::upload_media`] asks Langfuse for an upload URL (`POST /api/public/media`),
//! uploads the bytes to it (`PUT`, with their SHA-256), and reports the upload
//! (`PATCH /api/public/media/{id}`). It returns a [`MediaReference`], whose
//! [`MediaReference::reference_string`] is the token Langfuse renders as the media wherever it
//! appears in an input, output or metadata value:
//! `@@@langfuseMedia:type=image/png|id=<media id>|source=bytes@@@`.
//!
//! Langfuse deduplicates by content: uploading the same bytes again for the same field returns
//! the existing media id without a second upload.
//!
//! [`LangfuseClient::get_media`] reads a media record (with a short-lived download URL) and
//! [`LangfuseClient::download_media`] fetches its bytes.

use base64::Engine as _;
use bon::bon;
use sha2::Digest as _;
use timvw_rust_runtime::Upload;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

pub use langfuse_client_base::models::{GetMediaResponse, MediaContentType};

/// Which part of a trace or observation the media belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaField {
    /// The input (default).
    #[default]
    Input,
    /// The output.
    Output,
    /// The metadata.
    Metadata,
}

impl MediaField {
    fn as_str(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Metadata => "metadata",
        }
    }
}

/// A reference to uploaded media, to embed in inputs, outputs or metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaReference {
    /// The Langfuse media id.
    pub media_id: String,
    /// The media type, for example `image/png`.
    pub content_type: String,
    /// Where the bytes came from: `bytes`, `base64_data_uri` or `file`.
    pub source: String,
}

impl MediaReference {
    /// The token Langfuse renders as the media:
    /// `@@@langfuseMedia:type=<content type>|id=<media id>|source=<source>@@@`.
    pub fn reference_string(&self) -> String {
        format!(
            "@@@langfuseMedia:type={}|id={}|source={}@@@",
            self.content_type, self.media_id, self.source
        )
    }

    /// Parse a reference string (see [`MediaReference::reference_string`]).
    pub fn parse(reference: &str) -> Option<Self> {
        let inner = reference
            .trim()
            .strip_prefix("@@@langfuseMedia:")?
            .strip_suffix("@@@")?;
        let mut media_id = None;
        let mut content_type = None;
        let mut source = None;
        for part in inner.split('|') {
            let (key, value) = part.split_once('=')?;
            match key {
                "id" => media_id = Some(value.to_owned()),
                "type" => content_type = Some(value.to_owned()),
                "source" => source = Some(value.to_owned()),
                _ => {}
            }
        }
        Some(Self {
            media_id: media_id?,
            content_type: content_type?,
            source: source?,
        })
    }

    /// Every reference string in a JSON value (strings anywhere inside it).
    pub fn find_all(value: &serde_json::Value) -> Vec<Self> {
        let mut found = Vec::new();
        collect_references(value, &mut found);
        found
    }
}

fn collect_references(value: &serde_json::Value, found: &mut Vec<MediaReference>) {
    use serde_json::Value;
    match value {
        Value::String(s) => {
            let mut rest = s.as_str();
            while let Some(start) = rest.find("@@@langfuseMedia:") {
                let after = &rest[start + 3..];
                let Some(end) = after.find("@@@") else { break };
                if let Some(reference) = MediaReference::parse(&rest[start..start + 3 + end + 3]) {
                    found.push(reference);
                }
                rest = &after[end + 3..];
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_references(v, found)),
        Value::Object(map) => map.values().for_each(|v| collect_references(v, found)),
        _ => {}
    }
}

impl std::fmt::Display for MediaReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reference_string())
    }
}

fn content_type_of(value: &str) -> Result<MediaContentType> {
    let content_type: MediaContentType =
        serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|_| {
            Error::Validation(format!("Langfuse does not accept media of type {value}"))
        })?;
    if content_type.to_string() == "unknown" {
        return Err(Error::Validation(format!(
            "Langfuse does not accept media of type {value}"
        )));
    }
    Ok(content_type)
}

#[bon]
impl LangfuseClient {
    /// Upload media for a trace (and optionally one of its observations) or a dataset item, and
    /// return its [`MediaReference`].
    ///
    /// `upload` is a [`timvw_rust_runtime::Upload`] (bytes, a stream, or natively a file); its
    /// media type comes from `content_type`, else from the upload (`Upload::mime`, or a file's
    /// extension). The content is read into memory to compute its SHA-256, which Langfuse
    /// requires. Give `trace_id` (with `observation_id` for an observation's media), or
    /// `dataset_id` with `dataset_item_id`. With [`crate::ApiVersion::V4`] the trace and
    /// observation ids are mapped like the builders' ids.
    ///
    /// ```no_run
    /// # use langfuse_ergonomic::ClientBuilder;
    /// # use langfuse_ergonomic::media::MediaField;
    /// # use timvw_rust_runtime::Upload;
    /// # async fn example(png: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClientBuilder::from_env()?.build()?;
    /// let reference = client
    ///     .upload_media()
    ///     .upload(Upload::bytes(png).mime("image/png"))
    ///     .trace_id("trace-1")
    ///     .field(MediaField::Input)
    ///     .call()
    ///     .await?;
    /// // Embed the reference string where the image belongs, e.g. in the trace input.
    /// let input = serde_json::json!({ "question": "What is in this image?", "image": reference.reference_string() });
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub async fn upload_media(
        &self,
        upload: Upload,
        /// The media type (`image/png`, `audio/wav`, `application/pdf`, ...).
        #[builder(into)]
        content_type: Option<String>,
        #[builder(into)] trace_id: Option<String>,
        #[builder(into)] observation_id: Option<String>,
        #[builder(into)] dataset_id: Option<String>,
        #[builder(into)] dataset_item_id: Option<String>,
        /// Where the media is used (default input).
        #[builder(default)]
        field: MediaField,
        /// The `source` written into the reference string (default `bytes`).
        #[builder(into, default = "bytes".to_owned())]
        source: String,
    ) -> Result<MediaReference> {
        use langfuse_client_base::apis::media_api;
        use langfuse_client_base::models::{GetMediaUploadUrlRequest, PatchMediaBody};

        let content_type = content_type
            .or_else(|| upload.media_type().map(str::to_owned))
            .ok_or_else(|| {
                Error::Validation("media upload needs a content type (`content_type`)".into())
            })?;
        let media_type = content_type_of(&content_type)?;
        if trace_id.is_none() && dataset_item_id.is_none() {
            return Err(Error::Validation(
                "media upload needs a trace_id or a dataset_id and dataset_item_id".into(),
            ));
        }

        let bytes = timvw_rust_runtime::Body::from(upload)
            .collect_bytes()
            .await
            .map_err(|e| Error::Api(format!("Failed to read the media to upload: {e}")))?;
        let sha256 = base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(&bytes));

        let request = GetMediaUploadUrlRequest {
            trace_id: trace_id.map(|id| Some(self.effective_trace_id(&id))),
            observation_id: observation_id.map(|id| Some(self.effective_observation_id(&id))),
            dataset_id: dataset_id.map(Some),
            dataset_item_id: dataset_item_id.map(Some),
            content_type: media_type,
            content_length: i64::try_from(bytes.len()).unwrap_or(i64::MAX),
            sha256_hash: sha256.clone(),
            field: field.as_str().to_owned(),
        };
        let response = media_api::media_get_upload_url()
            .configuration(self.configuration())
            .get_media_upload_url_request(&request)
            .call()
            .await
            .map_err(crate::error::map_api_error)?;
        let reference = MediaReference {
            media_id: response.media_id.clone(),
            content_type: content_type.clone(),
            source,
        };
        // No URL: Langfuse already has these bytes.
        let Some(upload_url) = response.upload_url.flatten() else {
            return Ok(reference);
        };

        let started = crate::time::Instant::now();
        let result = self
            .configuration
            .client
            .request(timvw_rust_runtime::Method::PUT, &upload_url)
            .header(
                timvw_rust_runtime::header::CONTENT_TYPE,
                content_type.as_str(),
            )
            .header("x-amz-checksum-sha256", sha256.as_str())
            .body(bytes)
            .send()
            .await;
        let elapsed = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
        let (status, error) = match &result {
            Ok(response) if response.status().is_success() => (response.status().as_u16(), None),
            Ok(response) => (
                response.status().as_u16(),
                Some(format!("upload failed with status {}", response.status())),
            ),
            Err(error) => (0, Some(error.to_string())),
        };
        let patch = PatchMediaBody {
            uploaded_at: crate::time::now_utc().fixed_offset(),
            upload_http_status: i64::from(status),
            upload_http_error: error.clone().map(Some),
            upload_time_ms: Some(Some(elapsed)),
        };
        media_api::media_patch()
            .configuration(self.configuration())
            .media_id(&reference.media_id)
            .patch_media_body(&patch)
            .call()
            .await
            .map_err(crate::error::map_api_error)?;
        match error {
            None => Ok(reference),
            Some(error) => Err(Error::Api(format!("Failed to upload media: {error}"))),
        }
    }

    /// Get a media record, with a short-lived download URL.
    pub async fn get_media(&self, media_id: &str) -> Result<GetMediaResponse> {
        langfuse_client_base::apis::media_api::media_get()
            .configuration(self.configuration())
            .media_id(media_id)
            .call()
            .await
            .map_err(crate::error::map_api_error)
    }

    /// Get a media record and download its bytes.
    pub async fn download_media(&self, media_id: &str) -> Result<(GetMediaResponse, Vec<u8>)> {
        let media = self.get_media(media_id).await?;
        let response = self
            .configuration
            .client
            .request(timvw_rust_runtime::Method::GET, &media.url)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(crate::error::error_from_response(response).await);
        }
        let bytes = response.bytes().await?;
        Ok((media, bytes.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use serde_json::json;

    #[test]
    fn reference_strings_round_trip() {
        let reference = MediaReference {
            media_id: "m1".into(),
            content_type: "image/png".into(),
            source: "bytes".into(),
        };
        let token = reference.reference_string();
        assert_eq!(
            token,
            "@@@langfuseMedia:type=image/png|id=m1|source=bytes@@@"
        );
        assert_eq!(MediaReference::parse(&token), Some(reference.clone()));
        assert_eq!(MediaReference::parse("@@@langfuseMedia:id=x@@@"), None);
        let found = MediaReference::find_all(&json!({"a": [format!("see {token} and {token}")]}));
        assert_eq!(found, vec![reference.clone(), reference]);
    }

    #[test]
    fn content_types_are_checked() {
        assert!(content_type_of("image/png").is_ok());
        assert!(content_type_of("application/x-nope").is_err());
    }
}
