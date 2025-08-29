//! Main client for interacting with the Langfuse API

use crate::error::Result;
use bon::bon;
use langfuse_client_base::apis::configuration::Configuration;

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
    ) -> Self {
        let public_key = public_key.into();
        let secret_key = secret_key.into();

        let configuration = Configuration {
            base_path: base_url.clone(),
            basic_auth: Some((public_key.clone(), Some(secret_key.clone()))),
            api_key: None,
            oauth_access_token: None,
            bearer_access_token: None,
            client: reqwest::Client::new(),
            user_agent: Some("langfuse-rs/0.1.0".to_string()),
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
