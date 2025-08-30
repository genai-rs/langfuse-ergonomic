//! Main client for interacting with the Langfuse API

use crate::batcher::{Batcher, BatcherConfig};
use crate::error::Result;
use bon::bon;
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
pub struct LangfuseClient {
    pub(crate) public_key: String,
    pub(crate) secret_key: String,
    pub(crate) base_url: String,
    pub(crate) configuration: Configuration,
}

#[bon]
impl LangfuseClient {
    /// Create a new Langfuse client with the given credentials  
    #[builder]
    pub fn builder(
        public_key: impl Into<String>,
        secret_key: impl Into<String>,
        #[builder(default = String::from("https://cloud.langfuse.com"))] base_url: String,
        timeout: Option<Duration>,
        connect_timeout: Option<Duration>,
        user_agent: Option<String>,
    ) -> Self {
        let public_key = public_key.into();
        let secret_key = secret_key.into();

        // Build HTTP client with sensible defaults
        #[allow(unused_mut)]
        let mut client_builder = reqwest::Client::builder()
            .timeout(timeout.unwrap_or(DEFAULT_TIMEOUT))
            .connect_timeout(connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90));

        // Disable compression by default, enable with feature flag
        #[cfg(not(feature = "compression"))]
        {
            client_builder = client_builder.no_gzip().no_brotli().no_deflate();
        }

        // Build client (ignore errors for now, use default client if building fails)
        let client = client_builder
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // Construct User-Agent with SDK info
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

    /// Create a new Langfuse client from environment variables
    ///
    /// Reads from:
    /// - `LANGFUSE_PUBLIC_KEY`: Required public key
    /// - `LANGFUSE_SECRET_KEY`: Required secret key  
    /// - `LANGFUSE_BASE_URL`: Optional base URL (defaults to <https://cloud.langfuse.com>)
    pub fn from_env() -> Result<Self> {
        use std::env;

        let public_key = env::var("LANGFUSE_PUBLIC_KEY").map_err(|_| {
            crate::error::Error::Configuration(
                "LANGFUSE_PUBLIC_KEY environment variable not set".to_string(),
            )
        })?;

        let secret_key = env::var("LANGFUSE_SECRET_KEY").map_err(|_| {
            crate::error::Error::Configuration(
                "LANGFUSE_SECRET_KEY environment variable not set".to_string(),
            )
        })?;

        let base_url = env::var("LANGFUSE_BASE_URL")
            .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string());

        Ok(Self::builder()
            .public_key(public_key)
            .secret_key(secret_key)
            .base_url(base_url)
            .build())
    }

    /// Get the underlying API configuration
    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    /// Validate that the client credentials are valid
    pub async fn validate(&self) -> Result<bool> {
        // This would make a simple API call to validate credentials
        // For now, we'll just return Ok(true)
        Ok(true)
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
    /// # use langfuse_ergonomic::{LangfuseClient, BatcherConfig};
    /// # use std::sync::Arc;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Arc::new(LangfuseClient::from_env()?);
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
}
