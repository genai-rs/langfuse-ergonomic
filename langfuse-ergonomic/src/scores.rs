//! Score-related functionality for evaluating traces and observations

use serde_json::Value;

use crate::client::LangfuseClient;
use crate::error::Result;

/// Builder for creating scores
pub struct ScoreBuilder<'a> {
    client: &'a LangfuseClient,
    trace_id: String,
    observation_id: Option<String>,
    name: String,
    value: Option<f64>,
    string_value: Option<String>,
    comment: Option<String>,
    metadata: Option<Value>,
}

impl LangfuseClient {
    /// Start building a score
    pub fn score(&self, trace_id: impl Into<String>, name: impl Into<String>) -> ScoreBuilder {
        ScoreBuilder {
            client: self,
            trace_id: trace_id.into(),
            observation_id: None,
            name: name.into(),
            value: None,
            string_value: None,
            comment: None,
            metadata: None,
        }
    }
}

impl<'a> ScoreBuilder<'a> {
    /// Set the observation ID (optional - if not set, score applies to the trace)
    pub fn observation_id(mut self, observation_id: impl Into<String>) -> Self {
        self.observation_id = Some(observation_id.into());
        self
    }
    
    /// Set a numeric score value
    pub fn value(mut self, value: f64) -> Self {
        self.value = Some(value);
        self.string_value = None; // Clear string value if numeric is set
        self
    }
    
    /// Set a categorical/string score value
    pub fn string_value(mut self, value: impl Into<String>) -> Self {
        self.string_value = Some(value.into());
        self.value = None; // Clear numeric value if string is set
        self
    }
    
    /// Add a comment explaining the score
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }
    
    /// Set metadata for the score
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
    
    /// Execute the score creation
    pub async fn send(self) -> Result<()> {
        // Validate that either value or string_value is set
        if self.value.is_none() && self.string_value.is_none() {
            return Err(crate::error::Error::Validation(
                "Score must have either a numeric value or string value".to_string()
            ));
        }
        
        // TODO: Implement actual API call when base client is generated
        // For now, we'll just return Ok
        Ok(())
    }
}

/// Common score configurations
impl LangfuseClient {
    /// Create a binary score (0 or 1)
    pub fn binary_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        value: bool,
    ) -> ScoreBuilder {
        self.score(trace_id, name)
            .value(if value { 1.0 } else { 0.0 })
    }
    
    /// Create a rating score (e.g., 1-5 stars)
    pub fn rating_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        rating: u8,
        max_rating: u8,
    ) -> ScoreBuilder {
        let normalized = rating as f64 / max_rating as f64;
        self.score(trace_id, name)
            .value(normalized)
            .metadata(serde_json::json!({
                "rating": rating,
                "max_rating": max_rating
            }))
    }
    
    /// Create a categorical score
    pub fn categorical_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
    ) -> ScoreBuilder {
        self.score(trace_id, name)
            .string_value(category)
    }
}