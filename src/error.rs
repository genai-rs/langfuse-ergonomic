//! Error types for the ergonomic Langfuse client

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("API error: {0}")]
    Api(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
