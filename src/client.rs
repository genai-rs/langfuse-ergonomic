//! Main client for interacting with the Langfuse API

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
}

impl LangfuseClient {
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
            .map_err(Error::Network)?;

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
                message: format!("Validation failed with status {}", status),
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
        // Clone the Arc to avoid moving self
        let client = LangfuseClient {
            public_key: self.public_key.clone(),
            secret_key: self.secret_key.clone(),
            base_url: self.base_url.clone(),
            configuration: Configuration {
                base_path: self.configuration.base_path.clone(),
                basic_auth: self.configuration.basic_auth.clone(),
                api_key: self.configuration.api_key.clone(),
                oauth_access_token: self.configuration.oauth_access_token.clone(),
                bearer_access_token: self.configuration.bearer_access_token.clone(),
                client: self.configuration.client.clone(),
                user_agent: self.configuration.user_agent.clone(),
            },
        };

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
    ) -> Self {
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

        let client = client_builder
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let default_user_agent = format!("{}/{} (Rust)", SDK_NAME, SDK_VERSION);
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

        Self {
            public_key,
            secret_key,
            base_url,
            configuration,
        }
    }
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
}

impl ClientBuilder {
    /// Start a new builder without credentials. Use [`ClientBuilder::public_key`] and
    /// [`ClientBuilder::secret_key`] to provide them before calling [`ClientBuilder::build`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder pre-populated from environment variables.
    pub fn from_env() -> Result<Self> {
        use std::env;

        let public_key = env::var("LANGFUSE_PUBLIC_KEY").map_err(|_| {
            Error::Configuration("LANGFUSE_PUBLIC_KEY environment variable not set".to_string())
        })?;

        let secret_key = env::var("LANGFUSE_SECRET_KEY").map_err(|_| {
            Error::Configuration("LANGFUSE_SECRET_KEY environment variable not set".to_string())
        })?;

        let base_url = env::var("LANGFUSE_BASE_URL").ok();

        Ok(Self {
            public_key: Some(public_key),
            secret_key: Some(secret_key),
            base_url,
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

        Ok(LangfuseClient::build_internal(
            public_key,
            secret_key,
            base_url,
            self.timeout,
            self.connect_timeout,
            self.user_agent,
        ))
    }
}
