//! Error types for the ergonomic Langfuse client

use std::fmt;
use std::time::Duration;
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

    /// Authentication failure
    #[error("Authentication failed: {message}")]
    Auth {
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Rate limit exceeded
    #[error("Rate limit exceeded (retry after {retry_after:?})")]
    RateLimit {
        /// How long to wait before retrying
        retry_after: Option<Duration>,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Server error (5xx status codes)
    #[error("Server error (status {status}): {message}")]
    Server {
        status: u16,
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Client error (4xx status codes other than auth/rate limit)
    #[error("Client error (status {status}): {message}")]
    Client {
        status: u16,
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Partial failure in batch operations (207 Multi-Status)
    #[error("Partial batch failure: {success_count} succeeded, {failure_count} failed")]
    PartialFailure {
        success_count: usize,
        failure_count: usize,
        /// Individual error details for failed items
        errors: Vec<EventError>,
        /// Successfully processed event IDs
        success_ids: Vec<String>,
    },

    /// Batch size exceeded
    #[error("Batch size exceeded: {size} bytes (max: {max_size} bytes)")]
    BatchSizeExceeded { size: usize, max_size: usize },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Error details for individual events in a batch
#[derive(Debug, Clone)]
pub struct EventError {
    /// The event ID that failed
    pub event_id: String,
    /// The error message
    pub message: String,
    /// Optional error code
    pub code: Option<String>,
    /// Whether this error is retryable
    pub retryable: bool,
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event {} failed: {}", self.event_id, self.message)?;
        if let Some(ref code) = self.code {
            write!(f, " (code: {})", code)?;
        }
        if self.retryable {
            write!(f, " [retryable]")?;
        }
        Ok(())
    }
}

impl Error {
    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Network(_) => true,
            Error::RateLimit { .. } => true,
            Error::Server { .. } => true,
            Error::PartialFailure { .. } => true,
            Error::Auth { .. } => false,
            Error::Client { .. } => false,
            Error::Validation(_) => false,
            Error::Serialization(_) => false,
            Error::Configuration(_) => false,
            Error::Api(_) => false,
            Error::BatchSizeExceeded { .. } => false,
        }
    }

    /// Get the retry delay if applicable
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::RateLimit { retry_after, .. } => *retry_after,
            Error::Server { .. } => Some(Duration::from_secs(5)), // Default retry for server errors
            Error::Network(_) => Some(Duration::from_secs(1)),    // Quick retry for network errors
            _ => None,
        }
    }

    /// Get the request ID if available
    pub fn request_id(&self) -> Option<&str> {
        match self {
            Error::Auth { request_id, .. }
            | Error::RateLimit { request_id, .. }
            | Error::Server { request_id, .. }
            | Error::Client { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }
}

/// Response from batch ingestion operations
#[derive(Debug)]
pub struct IngestionResponse {
    /// Successfully processed event IDs
    pub success_ids: Vec<String>,
    /// Failed events with error details
    pub failures: Vec<EventError>,
    /// Overall success/failure counts
    pub success_count: usize,
    pub failure_count: usize,
}

impl IngestionResponse {
    /// Check if all events were processed successfully
    pub fn is_success(&self) -> bool {
        self.failure_count == 0
    }

    /// Check if this was a partial failure
    pub fn is_partial_failure(&self) -> bool {
        self.success_count > 0 && self.failure_count > 0
    }

    /// Convert to an error if there were any failures
    pub fn to_error(&self) -> Option<Error> {
        if self.failure_count > 0 {
            Some(Error::PartialFailure {
                success_count: self.success_count,
                failure_count: self.failure_count,
                errors: self.failures.clone(),
                success_ids: self.success_ids.clone(),
            })
        } else {
            None
        }
    }
}
