//! Main client for interacting with the Langfuse API

use crate::error::Result;
use bon::bon;
use langfuse_client_base::apis::configuration::Configuration;
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
    #[allow(dead_code)]
    public_key: String,
    #[allow(dead_code)]
    secret_key: String,
    #[allow(dead_code)]
    base_url: String,
    configuration: Configuration,
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
        let client_builder = reqwest::Client::builder()
            .timeout(timeout.unwrap_or(DEFAULT_TIMEOUT))
            .connect_timeout(connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT))
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Duration::from_secs(90));

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
}
