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
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Default connection timeout
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Main client for interacting with the Langfuse API
#[derive(Clone)]
pub struct LangfuseClient {
    pub(crate) public_key: String,
    pub(crate) secret_key: String,
    pub(crate) base_url: String,
    pub(crate) configuration: Configuration,
    pub(crate) api_version: ApiVersion,
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

    /// Get the underlying API configuration
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Validate that the client credentials are valid
    pub async fn validate(&self) -> Result<bool> {
        use crate::error::Error;

        // Make a lightweight request to the health endpoint
        let url = format!("{}/api/public/health", self.base_url);
        let response = self
            .configuration
            .client
            .get(&url)
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(Error::Middleware)?;

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
        http_client: Option<reqwest_middleware::ClientWithMiddleware>,
        api_version: ApiVersion,
    ) -> Result<Self> {
        let client = match http_client {
            Some(client) => client,
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
        };

        let default_user_agent = format!("{SDK_NAME}/{SDK_VERSION} (Rust)");
        let final_user_agent = user_agent.unwrap_or(default_user_agent);

        let configuration = Configuration {
            base_path: base_url.clone(),
            basic_auth: Some((public_key.clone(), Some(secret_key.clone()))),
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
        })
    }
}

/// Build the HTTP client, surfacing failures instead of silently dropping the configured
/// timeouts and connection settings.
fn finish_http_client(
    builder: reqwest::ClientBuilder,
) -> Result<reqwest_middleware::ClientWithMiddleware> {
    let reqwest_client = builder
        .build()
        .map_err(|e| Error::Configuration(format!("failed to build HTTP client: {e}")))?;

    // Wrap with middleware support
    Ok(reqwest_middleware::ClientBuilder::new(reqwest_client).build())
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
    http_client: Option<reqwest_middleware::ClientWithMiddleware>,
    api_version: Option<ApiVersion>,
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

    /// Set a custom HTTP client with middleware.
    ///
    /// This allows you to provide a pre-configured `ClientWithMiddleware` with
    /// custom settings like retry policies, connection pooling, logging, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use reqwest_middleware::ClientBuilder;
    /// use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};
    ///
    /// let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
    /// let client = ClientBuilder::new(reqwest::Client::new())
    ///     .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    ///     .build();
    ///
    /// let langfuse_client = langfuse_ergonomic::ClientBuilder::from_env()?
    ///     .http_client(client)
    ///     .build()?;
    /// ```
    #[must_use]
    pub fn http_client(mut self, client: reqwest_middleware::ClientWithMiddleware) -> Self {
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
        )
    }
}

#[cfg(all(test, any(feature = "rustls", feature = "native-tls")))]
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
