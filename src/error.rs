//! Error types for the ergonomic Langfuse client

use std::fmt;
use std::time::Duration;
use thiserror::Error;

/// Result type for Langfuse operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by the Langfuse ergonomic client
#[derive(Error, Debug)]
pub enum Error {
    /// The Langfuse API returned an error
    #[error("API error: {0}")]
    Api(String),

    /// Invalid or missing client configuration
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Invalid input rejected before sending a request
    #[error("Validation error: {0}")]
    Validation(String),

    /// JSON serialization or deserialization failed
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Network-level failure while sending a request
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Failure in the HTTP middleware stack
    #[error("Middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

    /// Authentication failure
    #[error("Authentication failed: {message}")]
    Auth {
        /// Description of the authentication failure
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
        /// HTTP status code
        status: u16,
        /// Error message returned by the server
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Client error (4xx status codes other than auth/rate limit)
    #[error("Client error (status {status}): {message}")]
    Client {
        /// HTTP status code
        status: u16,
        /// Error message returned by the server
        message: String,
        /// Request ID for debugging
        request_id: Option<String>,
    },

    /// Partial failure in batch operations (207 Multi-Status)
    #[error("Partial batch failure: {success_count} succeeded, {failure_count} failed")]
    PartialFailure {
        /// Number of events processed successfully
        success_count: usize,
        /// Number of events that failed
        failure_count: usize,
        /// Individual error details for failed items
        errors: Vec<EventError>,
        /// Successfully processed event IDs
        success_ids: Vec<String>,
    },

    /// Batch size exceeded
    #[error("Batch size exceeded: {size} bytes (max: {max_size} bytes)")]
    BatchSizeExceeded {
        /// Size of the rejected batch in bytes
        size: usize,
        /// Maximum allowed batch size in bytes
        max_size: usize,
    },

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
            write!(f, " (code: {code})")?;
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
            Error::Middleware(_) => true,
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
    /// Number of failed events
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

/// Parse a `Retry-After` header, given either as seconds or as an HTTP-date.
pub(crate) fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            // Try parsing as seconds (integer)
            s.parse::<u64>().ok().or_else(|| {
                // Try parsing as HTTP-date (RFC2616)
                chrono::DateTime::parse_from_rfc2822(s).ok().and_then(|dt| {
                    let now = chrono::Utc::now();
                    let retry_time = dt.with_timezone(&chrono::Utc);
                    if retry_time > now {
                        Some((retry_time - now).num_seconds() as u64)
                    } else {
                        None
                    }
                })
            })
        })
        .map(Duration::from_secs)
}

/// Read the `x-request-id` header, if present.
pub(crate) fn parse_request_id(headers: &reqwest::header::HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
}

/// Map a non-success HTTP response to an [`Error`], keeping `Retry-After` and `x-request-id`.
pub(crate) async fn error_from_response(response: reqwest::Response) -> Error {
    let status = response.status().as_u16();
    let request_id = parse_request_id(response.headers());
    let retry_after = parse_retry_after(response.headers());
    let body = response.text().await.unwrap_or_default();
    let message = if body.is_empty() {
        format!("status {status}")
    } else {
        body
    };
    match status {
        401 | 403 => Error::Auth {
            message,
            request_id,
        },
        429 => Error::RateLimit {
            retry_after,
            request_id,
        },
        500..=599 => Error::Server {
            status,
            message,
            request_id,
        },
        _ => Error::Client {
            status,
            message,
            request_id,
        },
    }
}

/// Helper to map API errors to appropriate error types based on status code
pub fn map_api_error<T>(err: langfuse_client_base::apis::Error<T>) -> Error {
    use langfuse_client_base::apis::Error as ApiError;

    match err {
        ApiError::Reqwest(e) => Error::Network(e),
        ApiError::ReqwestMiddleware(e) => Error::Middleware(e),
        ApiError::Serde(e) => Error::Serialization(e),
        ApiError::Io(e) => Error::Api(format!("IO error: {e}")),
        ApiError::ResponseError(response) => {
            let status = response.status.as_u16();
            let request_id = parse_request_id(&response.headers);
            let message = if response.content.is_empty() {
                format!("status {status}")
            } else {
                response.content
            };

            match status {
                401 | 403 => Error::Auth {
                    message,
                    request_id,
                },
                429 => Error::RateLimit {
                    retry_after: parse_retry_after(&response.headers),
                    request_id,
                },
                400..=499 => Error::Client {
                    status,
                    message,
                    request_id,
                },
                500..=599 => Error::Server {
                    status,
                    message,
                    request_id,
                },
                _ => Error::Api(message),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
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

    fn response_error(
        status: u16,
        headers: &[(&'static str, &'static str)],
    ) -> langfuse_client_base::apis::Error<()> {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (name, value) in headers {
            header_map.insert(*name, reqwest::header::HeaderValue::from_static(value));
        }
        langfuse_client_base::apis::Error::ResponseError(Box::new(
            langfuse_client_base::apis::ResponseContent {
                status: reqwest::StatusCode::from_u16(status).unwrap(),
                content: "body".to_string(),
                entity: None,
                headers: header_map,
            },
        ))
    }

    #[test]
    fn test_map_api_error_reads_response_headers() {
        let error = map_api_error(response_error(
            429,
            &[("retry-after", "7"), ("x-request-id", "req-429")],
        ));
        assert!(matches!(error, Error::RateLimit { .. }));
        assert_eq!(error.retry_after(), Some(Duration::from_secs(7)));
        assert_eq!(error.request_id(), Some("req-429"));

        let error = map_api_error(response_error(503, &[("x-request-id", "req-503")]));
        assert!(matches!(error, Error::Server { status: 503, .. }));
        assert_eq!(error.request_id(), Some("req-503"));

        let error = map_api_error(response_error(401, &[]));
        assert!(matches!(error, Error::Auth { .. }));
        assert_eq!(error.request_id(), None);
    }

    #[test]
    fn test_parse_retry_after_http_date() {
        let mut headers = reqwest::header::HeaderMap::new();
        let future = (chrono::Utc::now() + chrono::Duration::seconds(120)).to_rfc2822();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(&future).unwrap(),
        );
        let delay = parse_retry_after(&headers).unwrap();
        assert!(
            (Duration::from_secs(115)..=Duration::from_secs(120)).contains(&delay),
            "unexpected delay {delay:?}"
        );

        let past = (chrono::Utc::now() - chrono::Duration::seconds(120)).to_rfc2822();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            reqwest::header::HeaderValue::from_str(&past).unwrap(),
        );
        assert_eq!(parse_retry_after(&headers), None);
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

        let display = format!("{error}");
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

        let display = format!("{error}");
        assert!(display.contains("minimal-id"));
        assert!(display.contains("Minimal error"));
        assert!(!display.contains("retryable"));
    }
}
