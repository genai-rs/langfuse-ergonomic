//! Trace-related functionality with builder patterns

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use langfuse_client_base::{CreateTraceRequest, TraceResponse};
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
}

impl LangfuseClient {
    /// Start building a new trace
    pub fn trace(&self) -> TraceBuilder {
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
    
    /// Execute the trace creation
    pub async fn send(self) -> Result<TraceResponse> {
        let request = CreateTraceRequest {
            id: self.id.or_else(|| Some(Uuid::new_v4().to_string())),
            name: self.name,
            input: self.input,
            output: self.output,
            metadata: self.metadata,
            tags: if self.tags.is_empty() { None } else { Some(self.tags) },
            user_id: self.user_id,
            session_id: self.session_id,
            timestamp: self.timestamp.or_else(|| Some(Utc::now())),
        };
        
        self.client.api()
            .create_trace(request)
            .await
            .map_err(Into::into)
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
        
        let trace = client.trace()
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