//! Error types for the ergonomic Langfuse client

use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Result type for Langfuse operations
pub type Result<T> = std::result::Result<T, Error>;

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

    /// Backpressure triggered when queue is full
    #[error("Backpressure: {reason} (policy: {policy:?})")]
    Backpressure {
        /// The backpressure policy that was triggered
        policy: crate::BackpressurePolicy,
        /// Reason for the backpressure
        reason: String,
    },
}

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
            Error::Backpressure { .. } => false,
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

/// Helper to map API errors to appropriate error types based on status code
pub fn map_api_error<E: std::fmt::Display>(err: E) -> Error {
    let error_str = err.to_string();

    // Try to extract status code from error message
    if error_str.contains("401")
        || error_str.contains("Unauthorized")
        || error_str.contains("403")
        || error_str.contains("Forbidden")
    {
        Error::Auth {
            message: error_str,
            request_id: None,
        }
    } else if error_str.contains("429") || error_str.contains("Too Many Requests") {
        Error::RateLimit {
            retry_after: None,
            request_id: None,
        }
    } else if error_str.contains("500")
        || error_str.contains("Internal Server Error")
        || error_str.contains("502")
        || error_str.contains("Bad Gateway")
        || error_str.contains("503")
        || error_str.contains("Service Unavailable")
        || error_str.contains("504")
        || error_str.contains("Gateway Timeout")
    {
        Error::Server {
            status: 500,
            message: error_str,
            request_id: None,
        }
    } else if error_str.contains("400")
        || error_str.contains("Bad Request")
        || error_str.contains("404")
        || error_str.contains("Not Found")
        || error_str.contains("422")
        || error_str.contains("Unprocessable Entity")
    {
        Error::Client {
            status: 400,
            message: error_str,
            request_id: None,
        }
    } else {
        Error::Api(error_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_is_retryable() {
        // Rate limit errors should be retryable
        let rate_limit_error = Error::RateLimit {
            retry_after: Some(Duration::from_secs(5)),
            request_id: None,
        };
        assert!(rate_limit_error.is_retryable());

        // Server errors should be retryable
        let server_error = Error::Server {
            status: 500,
            message: "Internal server error".to_string(),
            request_id: None,
        };
        assert!(server_error.is_retryable());

        // Auth errors should not be retryable
        let auth_error = Error::Auth {
            message: "Invalid credentials".to_string(),
            request_id: None,
        };
        assert!(!auth_error.is_retryable());

        // Client errors should not be retryable
        let client_error = Error::Client {
            status: 400,
            message: "Bad request".to_string(),
            request_id: None,
        };
        assert!(!client_error.is_retryable());

        // Validation errors should not be retryable
        let validation_error = Error::Validation("Invalid input".to_string());
        assert!(!validation_error.is_retryable());
    }

    #[test]
    fn test_error_retry_after() {
        // Rate limit error with retry_after
        let rate_limit_error = Error::RateLimit {
            retry_after: Some(Duration::from_secs(10)),
            request_id: None,
        };
        assert_eq!(
            rate_limit_error.retry_after(),
            Some(Duration::from_secs(10))
        );

        // Server error should have default retry delay
        let server_error = Error::Server {
            status: 503,
            message: "Service unavailable".to_string(),
            request_id: None,
        };
        assert_eq!(server_error.retry_after(), Some(Duration::from_secs(5)));

        // Auth error should have no retry delay
        let auth_error = Error::Auth {
            message: "Unauthorized".to_string(),
            request_id: None,
        };
        assert_eq!(auth_error.retry_after(), None);
    }

    #[test]
    fn test_ingestion_response_success() {
        let response = IngestionResponse {
            success_ids: vec!["id1".to_string(), "id2".to_string()],
            failures: vec![],
            success_count: 2,
            failure_count: 0,
        };

        assert!(response.is_success());
        assert!(!response.is_partial_failure());
        assert!(response.to_error().is_none());
    }

    #[test]
    fn test_ingestion_response_partial_failure() {
        let response = IngestionResponse {
            success_ids: vec!["id1".to_string()],
            failures: vec![EventError {
                event_id: "id2".to_string(),
                message: "Validation failed".to_string(),
                code: Some("VALIDATION_ERROR".to_string()),
                retryable: false,
            }],
            success_count: 1,
            failure_count: 1,
        };

        assert!(!response.is_success());
        assert!(response.is_partial_failure());

        let error = response.to_error().unwrap();
        match error {
            Error::PartialFailure {
                success_count,
                failure_count,
                ..
            } => {
                assert_eq!(success_count, 1);
                assert_eq!(failure_count, 1);
            }
            _ => panic!("Expected PartialFailure error"),
        }
    }

    #[test]
    fn test_ingestion_response_total_failure() {
        let response = IngestionResponse {
            success_ids: vec![],
            failures: vec![
                EventError {
                    event_id: "id1".to_string(),
                    message: "Auth failed".to_string(),
                    code: Some("AUTH_ERROR".to_string()),
                    retryable: false,
                },
                EventError {
                    event_id: "id2".to_string(),
                    message: "Rate limited".to_string(),
                    code: Some("RATE_LIMIT".to_string()),
                    retryable: true,
                },
            ],
            success_count: 0,
            failure_count: 2,
        };

        assert!(!response.is_success());
        assert!(!response.is_partial_failure()); // No successes
        assert!(response.to_error().is_some());
    }

    #[test]
    fn test_event_error_display() {
        let error = EventError {
            event_id: "test-id".to_string(),
            message: "Something went wrong".to_string(),
            code: Some("TEST_ERROR".to_string()),
            retryable: true,
        };

        let display = format!("{}", error);
        assert!(display.contains("test-id"));
        assert!(display.contains("Something went wrong"));
        assert!(display.contains("TEST_ERROR"));
        assert!(display.contains("retryable"));
    }

    #[test]
    fn test_event_error_display_minimal() {
        let error = EventError {
            event_id: "minimal-id".to_string(),
            message: "Minimal error".to_string(),
            code: None,
            retryable: false,
        };

        let display = format!("{}", error);
        assert!(display.contains("minimal-id"));
        assert!(display.contains("Minimal error"));
        assert!(!display.contains("retryable"));
    }
}
