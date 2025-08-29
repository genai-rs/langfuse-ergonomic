//! Trace-related functionality with builder patterns

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::Result;

/// Builder for creating traces
pub struct TraceBuilder<'a> {
    client: &'a LangfuseClient,
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
    output: Option<Value>,
    metadata: Option<Value>,
    tags: Vec<String>,
    user_id: Option<String>,
    session_id: Option<String>,
    timestamp: Option<DateTime<Utc>>,
    release: Option<String>,
    version: Option<String>,
    public: Option<bool>,
}

/// Response from trace creation
pub struct TraceResponse {
    pub id: String,
}

impl LangfuseClient {
    /// Start building a new trace
    pub fn trace(&self) -> TraceBuilder<'_> {
        TraceBuilder {
            client: self,
            id: None,
            name: None,
            input: None,
            output: None,
            metadata: None,
            tags: Vec::new(),
            user_id: None,
            session_id: None,
            timestamp: None,
            release: None,
            version: None,
            public: None,
        }
    }
}

impl<'a> TraceBuilder<'a> {
    /// Set the trace ID (if not provided, one will be generated)
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the trace name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the input data
    pub fn input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the output data
    pub fn output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add a single tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set multiple tags at once
    pub fn tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags.extend(tags.into_iter().map(|s| s.into()));
        self
    }

    /// Set the user ID
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Set the session ID
    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set a custom timestamp (defaults to now)
    pub fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the release
    pub fn release(mut self, release: impl Into<String>) -> Self {
        self.release = Some(release.into());
        self
    }

    /// Set the version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Make trace publicly accessible
    pub fn public(mut self, public: bool) -> Self {
        self.public = Some(public);
        self
    }

    /// Execute the trace creation
    pub async fn send(self) -> Result<TraceResponse> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            IngestionBatchRequest, IngestionEvent, IngestionEventOneOf, TraceBody,
        };

        let trace_id = self.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = self
            .timestamp
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let trace_body = TraceBody {
            id: Some(Some(trace_id.clone())),
            timestamp: Some(Some(timestamp.clone())),
            name: self.name.map(Some),
            user_id: self.user_id.map(Some),
            input: self.input.map(Some),
            output: self.output.map(Some),
            session_id: self.session_id.map(Some),
            release: self.release.map(Some),
            version: self.version.map(Some),
            metadata: self.metadata.map(Some),
            tags: if self.tags.is_empty() {
                None
            } else {
                Some(Some(self.tags))
            },
            environment: None,
            public: self.public.map(Some),
        };

        let event = IngestionEventOneOf {
            body: Box::new(trace_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf(Box::new(event))],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.client.configuration(), batch_request)
            .await
            .map(|_| TraceResponse { id: trace_id })
            .map_err(|e| crate::error::Error::Api(format!("Failed to create trace: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_trace_builder() {
        let client = LangfuseClient::builder()
            .public_key("test-public-key")
            .secret_key("test-secret-key")
            .build();

        let trace = client
            .trace()
            .name("test-trace")
            .input(json!({"prompt": "Hello"}))
            .output(json!({"response": "Hi"}))
            .tag("test")
            .tag("example")
            .user_id("user-123");

        // In a real test, we'd mock the API call
        assert_eq!(trace.name, Some("test-trace".to_string()));
        assert_eq!(trace.tags, vec!["test", "example"]);
        assert_eq!(trace.user_id, Some("user-123".to_string()));
    }
}
