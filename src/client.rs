//! Main client for interacting with the Langfuse API

use crate::api_version::ApiVersion;
use crate::batcher::{Batcher, BatcherConfig};
use crate::error::{Error, Result};
use langfuse_client_base::apis::configuration::Configuration;
use std::sync::Arc;
use std::time::Duration;

/// SDK version for User-Agent header
const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");
const SDK_NAME: &str = env!("CARGO_PKG_NAME");

/// Default timeout for API requests
#[cfg(not(target_family = "wasm"))]
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Default connection timeout
#[cfg(not(target_family = "wasm"))]
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Main client for interacting with the Langfuse API
#[derive(Clone)]
pub struct LangfuseClient {
    pub(crate) public_key: String,
    pub(crate) secret_key: String,
    pub(crate) base_url: String,
    pub(crate) configuration: Configuration,
    pub(crate) api_version: ApiVersion,
    /// Langfuse v4 write state (span cache, trace attributes), shared by clones and batchers.
    pub(crate) v4_state: Arc<std::sync::Mutex<crate::otel::V4State>>,
    /// Applied to inputs, outputs and metadata before they are sent (see [`crate::mask`]).
    pub(crate) mask: Option<crate::mask::Mask>,
    /// Prompts fetched with [`LangfuseClient::prompt`], shared by clones.
    pub(crate) prompt_cache: Arc<crate::prompts::PromptCache>,
}

impl LangfuseClient {
    /// The Langfuse API generation this client writes to (see [`ApiVersion`]).
    pub fn api_version(&self) -> ApiVersion {
        self.api_version
    }

    /// Return a copy of this client that uses `api_version`.
    #[must_use]
    pub fn with_api_version(mut self, api_version: ApiVersion) -> Self {
        self.api_version = api_version;
        self
    }

    /// Fail with a clear error when a Langfuse v3-only method is called in V4 mode.
    pub(crate) fn require_v3(&self, method: &str, alternative: &str) -> Result<()> {
        match self.api_version {
            ApiVersion::V3 => Ok(()),
            ApiVersion::V4 => Err(Error::Configuration(format!(
                "{method}() uses a Langfuse v3 endpoint that Langfuse v4 removed \
                 (Langfuse Cloud: 2026-11-16); {alternative}, or select ApiVersion::V3 \
                 for a Langfuse v3 server"
            ))),
        }
    }

    /// Trace attributes given on an observation builder (`user_id`, `session_id`, `tags`).
    ///
    /// V4 records them in the write state, which stamps them on every span of the trace. V3
    /// returns a trace upsert event to send with the observation.
    pub(crate) fn trace_attribute_events(
        &self,
        trace_id: &str,
        user_id: Option<String>,
        session_id: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Vec<langfuse_client_base::models::IngestionEvent> {
        use langfuse_client_base::models::{
            ingestion_event_trace_event::Type, IngestionEvent, IngestionEventTraceEvent, TraceBody,
        };

        if user_id.is_none() && session_id.is_none() && tags.is_none() {
            return Vec::new();
        }
        match self.api_version {
            ApiVersion::V4 => {
                self.v4_state().remember_trace_attributes(
                    trace_id,
                    user_id.as_deref(),
                    session_id.as_deref(),
                    tags.as_deref(),
                );
                Vec::new()
            }
            ApiVersion::V3 => {
                let body = TraceBody {
                    id: Some(Some(trace_id.to_owned())),
                    user_id: user_id.map(Some),
                    session_id: session_id.map(Some),
                    tags: tags.map(Some),
                    ..TraceBody::default()
                };
                vec![IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
                    uuid::Uuid::new_v4().to_string(),
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                    body,
                    Type::TraceCreate,
                ))]
            }
        }
    }

    /// The trace id Langfuse stores for `id` with this client's API version.
    ///
    /// V3 stores ids unchanged; V4 maps them to OpenTelemetry trace ids
    /// (see [`crate::otel::trace_id`]).
    pub fn effective_trace_id(&self, id: &str) -> String {
        match self.api_version {
            ApiVersion::V3 => id.to_owned(),
            ApiVersion::V4 => crate::otel::trace_id(id),
        }
    }

    /// The observation id Langfuse stores for `id` with this client's API version.
    ///
    /// V3 stores ids unchanged; V4 maps them to OpenTelemetry span ids
    /// (see [`crate::otel::observation_id`]).
    pub fn effective_observation_id(&self, id: &str) -> String {
        match self.api_version {
            ApiVersion::V3 => id.to_owned(),
            ApiVersion::V4 => crate::otel::observation_id(id),
        }
    }

    /// Apply the mask function, if one is set, to an event about to be sent.
    pub(crate) fn mask_event(&self, event: &mut langfuse_client_base::models::IngestionEvent) {
        if let Some(mask) = &self.mask {
            mask.apply_to_event(event);
        }
    }

    /// The mask function set with [`ClientBuilder::mask`], if any.
    pub fn mask(&self) -> Option<&crate::mask::Mask> {
        self.mask.as_ref()
    }

    /// The base URL of the Langfuse web UI (the base URL without a trailing `/api/public`).
    pub fn web_base_url(&self) -> String {
        let url = self.base_url.trim_end_matches('/');
        let url = url
            .strip_suffix("/api/public")
            .or_else(|| url.strip_suffix("/api"))
            .unwrap_or(url);
        url.to_owned()
    }

    /// Get the underlying API configuration
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Validate that the client credentials are valid
    pub async fn validate(&self) -> Result<bool> {
        use crate::error::Error;

        // Make a lightweight request to the health endpoint
        let url = format!("{}/api/public/health", self.base_url);
        let request = self
            .configuration
            .client
            .request(timvw_rust_runtime::Method::GET, &url)
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .send();
        let response = crate::time::timeout(Duration::from_secs(5), request)
            .await
            .ok_or_else(|| Error::Api("health check timed out after 5s".to_string()))??;

        // Check if we got a successful response
        match response.status() {
            status if status.is_success() => Ok(true),
            status if status == 401 || status == 403 => Err(Error::Auth {
                message: "Invalid credentials".to_string(),
                request_id: response
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
            }),
            status => Err(Error::Client {
                status: status.as_u16(),
                message: format!("Validation failed with status {status}"),
                request_id: response
                    .headers()
                    .get("x-request-id")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
            }),
        }
    }

    /// Create a batcher for efficient batch ingestion
    ///
    /// The batcher automatically handles:
    /// - Batching events up to size/count limits
    /// - Automatic flushing on intervals
    /// - 207 Multi-Status response parsing
    /// - Retrying only failed events
    /// - Exponential backoff for retryable errors
    ///
    /// # Example
    /// ```no_run
    /// # use langfuse_ergonomic::{ClientBuilder, LangfuseClient, BatcherConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(ClientBuilder::from_env()?.build()?);
    /// let batcher = client.create_batcher(None).await;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_batcher(self: Arc<Self>, config: Option<BatcherConfig>) -> Batcher {
        let client = (*self).clone();

        let config = config.unwrap_or_default();

        Batcher::builder()
            .client(client)
            .max_events(config.max_events)
            .max_bytes(config.max_bytes)
            .flush_interval(config.flush_interval)
            .max_retries(config.max_retries)
            .initial_retry_delay(config.initial_retry_delay)
            .max_retry_delay(config.max_retry_delay)
            .retry_jitter(config.retry_jitter)
            .max_queue_size(config.max_queue_size)
            .backpressure_policy(config.backpressure_policy)
            .fail_fast(config.fail_fast)
            .build()
            .await
    }

    /// Start building a [`Batcher`] anchored to this client.
    pub fn batcher(&self) -> crate::batcher::BatcherBuilderWithClient {
        crate::batcher::Batcher::builder().client(self.clone())
    }

    fn build_internal(
        public_key: String,
        secret_key: String,
        base_url: String,
        timeout: Option<Duration>,
        connect_timeout: Option<Duration>,
        user_agent: Option<String>,
        http_client: Option<timvw_rust_runtime::Client>,
        api_version: ApiVersion,
        v4_state: crate::otel::V4State,
        mask: Option<crate::mask::Mask>,
    ) -> Result<Self> {
        let client = match http_client {
            Some(client) => client,
            #[cfg(not(target_family = "wasm"))]
            None => {
                #[allow(unused_mut)]
                let mut client_builder = reqwest::Client::builder()
                    .timeout(timeout.unwrap_or(DEFAULT_TIMEOUT))
                    .connect_timeout(connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT))
                    .pool_max_idle_per_host(10)
                    .pool_idle_timeout(Duration::from_secs(90));

                #[cfg(not(feature = "compression"))]
                {
                    client_builder = client_builder.no_gzip().no_brotli().no_deflate();
                }

                finish_http_client(client_builder)?
            }
            // On WebAssembly reqwest sends with the host's `fetch`, which owns connections, timeouts and
            // compression: `timeout` and `connect_timeout` do not apply there.
            #[cfg(target_family = "wasm")]
            None => {
                let _ = (timeout, connect_timeout);
                finish_http_client(reqwest::Client::builder())?
            }
        };

        let default_user_agent = format!("{SDK_NAME}/{SDK_VERSION} (Rust)");
        let final_user_agent = user_agent.unwrap_or(default_user_agent);

        let configuration = Configuration {
            base_path: base_url.clone(),
            basic_auth: Some((public_key.clone(), Some(secret_key.clone().into()))),
            api_key: None,
            oauth_access_token: None,
            bearer_access_token: None,
            client,
            user_agent: Some(final_user_agent),
        };

        Ok(Self {
            public_key,
            secret_key,
            base_url,
            configuration,
            api_version,
            v4_state: Arc::new(std::sync::Mutex::new(v4_state)),
            mask,
            prompt_cache: Arc::new(crate::prompts::PromptCache::default()),
        })
    }
}

/// Build the HTTP client, surfacing failures instead of silently dropping the configured
/// timeouts and connection settings.
fn finish_http_client(builder: reqwest::ClientBuilder) -> Result<timvw_rust_runtime::Client> {
    let reqwest_client = builder
        .build()
        .map_err(|e| Error::Configuration(format!("failed to build HTTP client: {e}")))?;

    // No retries at this level: the batcher retries ingestion itself (with backoff and jitter),
    // and retrying here as well would multiply the attempts. Add a RetryLayer with `http_client`.
    Ok(timvw_rust_runtime::Client::from_service(
        timvw_rust_runtime::ReqwestService::new(reqwest_client),
    ))
}

/// Builder for [`LangfuseClient`], mirroring the style of `opentelemetry-langfuse`.
#[derive(Default, Debug, Clone)]
pub struct ClientBuilder {
    public_key: Option<String>,
    secret_key: Option<String>,
    base_url: Option<String>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    user_agent: Option<String>,
    http_client: Option<timvw_rust_runtime::Client>,
    api_version: Option<ApiVersion>,
    v4_state_capacity: Option<usize>,
    v4_state_max_bytes: Option<usize>,
    mask: Option<crate::mask::Mask>,
}

impl ClientBuilder {
    /// Start a new builder without credentials. Use [`ClientBuilder::public_key`] and
    /// [`ClientBuilder::secret_key`] to provide them before calling [`ClientBuilder::build`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder pre-populated from environment variables.
    ///
    /// Reads `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY` (required), `LANGFUSE_BASE_URL`
    /// (optional) and `LANGFUSE_API_VERSION` (optional, `v3` or `v4`, default `v4`).
    ///
    /// Not available on WebAssembly (`target_family = "wasm"`), which has no process environment: configure the
    /// builder explicitly there ([`ClientBuilder::new`], [`ClientBuilder::public_key`], ...), for example from a
    /// Cloudflare Worker's `env` bindings.
    #[cfg(not(target_family = "wasm"))]
    #[cfg_attr(docsrs, doc(cfg(not(target_family = "wasm"))))]
    pub fn from_env() -> Result<Self> {
        use std::env;

        let public_key = env::var("LANGFUSE_PUBLIC_KEY").map_err(|_| {
            Error::Configuration("LANGFUSE_PUBLIC_KEY environment variable not set".to_string())
        })?;

        let secret_key = env::var("LANGFUSE_SECRET_KEY").map_err(|_| {
            Error::Configuration("LANGFUSE_SECRET_KEY environment variable not set".to_string())
        })?;

        let base_url = env::var("LANGFUSE_BASE_URL").ok();

        let api_version = match env::var(ApiVersion::ENV_VAR) {
            Ok(value) if !value.trim().is_empty() => Some(value.parse::<ApiVersion>()?),
            _ => None,
        };

        Ok(Self {
            public_key: Some(public_key),
            secret_key: Some(secret_key),
            base_url,
            api_version,
            ..Self::default()
        })
    }

    /// Set the public key used for authentication.
    #[must_use]
    pub fn public_key(mut self, value: impl Into<String>) -> Self {
        self.public_key = Some(value.into());
        self
    }

    /// Set the secret key used for authentication.
    #[must_use]
    pub fn secret_key(mut self, value: impl Into<String>) -> Self {
        self.secret_key = Some(value.into());
        self
    }

    /// Override the Langfuse base URL (defaults to `https://cloud.langfuse.com`).
    #[must_use]
    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = Some(value.into());
        self
    }

    /// Select the Langfuse API generation (defaults to [`ApiVersion::V4`]).
    ///
    /// Use [`ApiVersion::V3`] for self-hosted Langfuse v3 servers. See [`ApiVersion`] for what
    /// changes between the two.
    #[must_use]
    pub fn api_version(mut self, value: ApiVersion) -> Self {
        self.api_version = Some(value);
        self
    }

    /// How many spans, traces and batch events the Langfuse v4 write state remembers
    /// (least recently used first out; default [`crate::otel::STATE_CAPACITY`]).
    ///
    /// The state lets updates re-send complete spans and propagates trace attributes; see
    /// [`crate::otel`]. Updates of evicted spans fail with [`Error::Validation`].
    #[must_use]
    pub fn v4_state_capacity(mut self, value: usize) -> Self {
        self.v4_state_capacity = Some(value);
        self
    }

    /// Memory budget, in bytes of serialized JSON, for the full spans the Langfuse v4 write
    /// state keeps (default [`crate::otel::STATE_MAX_BYTES`]). Ended spans are evicted first.
    #[must_use]
    pub fn v4_state_max_bytes(mut self, value: usize) -> Self {
        self.v4_state_max_bytes = Some(value);
        self
    }

    /// Set a function that masks (redacts) every input, output and metadata value before it is
    /// sent: by the builders, the [`Batcher`] and [`observe`](mod@crate::observe). See [`crate::mask`].
    ///
    /// A panic in the function is caught (natively) and the value replaced by
    /// [`crate::mask::MASK_FAILED`].
    #[must_use]
    pub fn mask(
        mut self,
        mask: impl Fn(serde_json::Value) -> serde_json::Value + Send + Sync + 'static,
    ) -> Self {
        self.mask = Some(crate::mask::Mask::new(mask));
        self
    }

    /// Override the request timeout (defaults to 60 seconds).
    #[must_use]
    pub fn timeout(mut self, value: Duration) -> Self {
        self.timeout = Some(value);
        self
    }

    /// Override the connection timeout (defaults to 10 seconds).
    #[must_use]
    pub fn connect_timeout(mut self, value: Duration) -> Self {
        self.connect_timeout = Some(value);
        self
    }

    /// Override the user agent string.
    #[must_use]
    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.user_agent = Some(value.into());
        self
    }

    /// Set a custom HTTP client.
    ///
    /// The client is a [`timvw_rust_runtime::Client`]: a tower service (reqwest by default)
    /// that takes tower layers for retries, timeouts, logging, rate limits and middleware of
    /// your own. `timeout` and `connect_timeout` do not apply to it; configure them on the
    /// `reqwest::Client` or add a `TimeoutLayer`. See the
    /// [runtime's guide](https://docs.rs/timvw-rust-runtime).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use timvw_rust_runtime::{retry::RetryPolicy, Client, ReqwestService};
    /// use tower::retry::RetryLayer;
    ///
    /// let client = Client::from_service(ReqwestService::new(reqwest::Client::new()))
    ///     .layer(RetryLayer::new(RetryPolicy::new(3)));
    ///
    /// let langfuse_client = langfuse_ergonomic::ClientBuilder::from_env()?
    ///     .http_client(client)
    ///     .build()?;
    /// ```
    #[must_use]
    pub fn http_client(mut self, client: timvw_rust_runtime::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Build a [`LangfuseClient`] using the configured options.
    pub fn build(self) -> Result<LangfuseClient> {
        let public_key = self
            .public_key
            .ok_or_else(|| Error::Configuration("Langfuse public key is required".to_string()))?;
        let secret_key = self
            .secret_key
            .ok_or_else(|| Error::Configuration("Langfuse secret key is required".to_string()))?;
        let base_url = self
            .base_url
            .unwrap_or_else(|| "https://cloud.langfuse.com".to_string());

        LangfuseClient::build_internal(
            public_key,
            secret_key,
            base_url,
            self.timeout,
            self.connect_timeout,
            self.user_agent,
            self.http_client,
            self.api_version.unwrap_or_default(),
            crate::otel::V4State::new(
                self.v4_state_capacity
                    .unwrap_or(crate::otel::STATE_CAPACITY),
                self.v4_state_max_bytes
                    .unwrap_or(crate::otel::STATE_MAX_BYTES),
            ),
            self.mask,
        )
    }
}

#[cfg(all(
    test,
    not(target_family = "wasm"),
    any(feature = "rustls", feature = "native-tls")
))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn http_client_build_failure_is_an_error() {
        // An unusable preconfigured TLS backend makes `reqwest::ClientBuilder::build` fail.
        let builder = reqwest::Client::builder().use_preconfigured_tls(());
        let err = finish_http_client(builder).unwrap_err();
        assert!(
            matches!(&err, Error::Configuration(msg) if msg.starts_with("failed to build HTTP client")),
            "unexpected error: {err:?}"
        );
    }
}
