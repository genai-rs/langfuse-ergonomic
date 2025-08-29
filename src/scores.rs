//! Score-related functionality for evaluating traces and observations

use serde_json::Value;
use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::Result;

/// Builder for creating scores
pub struct ScoreBuilder<'a> {
    #[allow(dead_code)]
    client: &'a LangfuseClient,
    #[allow(dead_code)]
    trace_id: String,
    observation_id: Option<String>,
    #[allow(dead_code)]
    name: String,
    value: Option<f64>,
    string_value: Option<String>,
    comment: Option<String>,
    metadata: Option<Value>,
}

impl LangfuseClient {
    /// Start building a score
    pub fn score(&self, trace_id: impl Into<String>, name: impl Into<String>) -> ScoreBuilder<'_> {
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
    pub async fn send(self) -> Result<String> {
        // Validate that either value or string_value is set
        if self.value.is_none() && self.string_value.is_none() {
            return Err(crate::error::Error::Validation(
                "Score must have either a numeric value or string value".to_string(),
            ));
        }

        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            CreateScoreValue, IngestionBatchRequest, IngestionEvent, IngestionEventOneOf1,
            ScoreBody, ScoreDataType,
        };

        let score_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let value = if let Some(v) = self.value {
            Box::new(CreateScoreValue::Number(v))
        } else if let Some(s) = self.string_value {
            Box::new(CreateScoreValue::String(s))
        } else {
            return Err(crate::error::Error::Validation(
                "Score must have either a numeric value or string value".to_string(),
            ));
        };

        let score_body = ScoreBody {
            id: Some(Some(score_id.clone())),
            trace_id: Some(Some(self.trace_id.clone())),
            name: self.name.clone(),
            value,
            observation_id: self.observation_id.map(Some),
            comment: self.comment.map(Some),
            data_type: if self.value.is_some() {
                Some(ScoreDataType::Numeric)
            } else {
                Some(ScoreDataType::Categorical)
            },
            config_id: None,
            session_id: None,
            dataset_run_id: None,
            environment: None,
            metadata: self.metadata.map(Some),
        };

        let event = IngestionEventOneOf1 {
            body: Box::new(score_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_1::Type::ScoreCreate,
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf1(Box::new(event))],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.client.configuration(), batch_request)
            .await
            .map(|_| score_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create score: {}", e)))
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
    ) -> ScoreBuilder<'_> {
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
    ) -> ScoreBuilder<'_> {
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
    ) -> ScoreBuilder<'_> {
        self.score(trace_id, name).string_value(category)
    }
}
