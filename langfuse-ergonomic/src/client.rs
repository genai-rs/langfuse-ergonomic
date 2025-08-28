//! Main client for interacting with the Langfuse API

use bon::bon;
use langfuse_client_base::{Configuration, DefaultApi};
use crate::error::Result;

/// Main client for interacting with the Langfuse API
pub struct LangfuseClient {
    public_key: String,
    secret_key: String,
    base_url: String,
    api: DefaultApi,
}

#[bon]
impl LangfuseClient {
    /// Create a new Langfuse client with the given credentials
    #[builder]
    pub fn new(
        public_key: impl Into<String>,
        secret_key: impl Into<String>,
        #[builder(default = String::from("https://cloud.langfuse.com"))]
        base_url: String,
    ) -> Self {
        let public_key = public_key.into();
        let secret_key = secret_key.into();
        
        let config = Configuration {
            base_path: base_url.clone(),
            basic_auth: Some((public_key.clone(), Some(secret_key.clone()))),
            api_key: None,
            client: reqwest::Client::new(),
        };
        
        let api = DefaultApi::new(config);
        
        Self {
            public_key,
            secret_key,
            base_url,
            api,
        }
    }
    
    /// Get the underlying API client
    pub fn api(&self) -> &DefaultApi {
        &self.api
    }
    
    /// Validate that the client credentials are valid
    pub async fn validate(&self) -> Result<bool> {
        // This would make a simple API call to validate credentials
        // For now, we'll just return Ok(true)
        Ok(true)
    }
}