//! Score-related functionality for evaluating traces and observations
//!
//! This module contains types and utilities for scoring.
//! The client methods live here as a `#[bon] impl` block on [`LangfuseClient`].

// Re-export common types that might be useful
pub use langfuse_client_base::models::{CreateScoreValue, ScoreBody, ScoreDataType};

use bon::bon;
use serde_json::Value;

use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

#[bon]
impl LangfuseClient {
    // ===== SCORING =====

    /// Create a score
    #[builder]
    pub async fn score(
        &self,
        #[builder(into)] trace_id: String,
        #[builder(into)] name: String,
        #[builder(into)] observation_id: Option<String>,
        value: Option<f64>,
        #[builder(into)] string_value: Option<String>,
        #[builder(into)] comment: Option<String>,
        #[builder(into)] queue_id: Option<String>,
        metadata: Option<Value>,
    ) -> Result<String> {
        // Validate that either value or string_value is set
        if value.is_none() && string_value.is_none() {
            return Err(crate::error::Error::Validation(
                "Score must have either a numeric value or string value".to_string(),
            ));
        }

        use langfuse_client_base::models::{
            CreateScoreValue, IngestionEvent, ScoreBody, ScoreDataType, ScoreEvent1,
        };

        let score_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let score_value = if let Some(v) = value {
            Box::new(CreateScoreValue::DoubleNumber(v))
        } else if let Some(s) = string_value {
            Box::new(CreateScoreValue::String(s))
        } else {
            return Err(crate::error::Error::Validation(
                "Score must have either a numeric value or string value".to_string(),
            ));
        };

        let score_body = ScoreBody {
            id: Some(Some(score_id.clone())),
            trace_id: Some(Some(trace_id)),
            name,
            queue_id: queue_id.map(Some),
            value: score_value,
            observation_id: observation_id.map(Some),
            comment: comment.map(Some),
            data_type: if value.is_some() {
                Some(ScoreDataType::Numeric)
            } else {
                Some(ScoreDataType::Categorical)
            },
            config_id: None,
            session_id: None,
            dataset_run_id: None,
            environment: None,
            metadata: metadata.map(Some),
        };

        let event = ScoreEvent1 {
            body: Box::new(score_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::score_event_1::Type::ScoreCreate,
        };

        self.ingest_events(vec![IngestionEvent::ScoreEvent1(Box::new(event))])
            .await
            .map(|_| score_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create score: {e}")))
    }

    /// Create a binary score (0 or 1)
    pub async fn binary_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        value: bool,
    ) -> Result<String> {
        self.score()
            .trace_id(trace_id.into())
            .name(name.into())
            .value(if value { 1.0 } else { 0.0 })
            .call()
            .await
    }

    /// Create a rating score (e.g., 1-5 stars)
    ///
    /// # Validation
    /// - `max_rating` must be greater than 0
    /// - `rating` must be less than or equal to `max_rating`
    pub async fn rating_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        rating: u8,
        max_rating: u8,
    ) -> Result<String> {
        // Validate inputs
        if max_rating == 0 {
            return Err(Error::Validation(
                "max_rating must be greater than 0".to_string(),
            ));
        }
        if rating > max_rating {
            return Err(Error::Validation(format!(
                "rating ({rating}) must be less than or equal to max_rating ({max_rating})"
            )));
        }

        let normalized = rating as f64 / max_rating as f64;
        let final_metadata = serde_json::json!({
            "rating": rating,
            "max_rating": max_rating
        });

        self.score()
            .trace_id(trace_id.into())
            .name(name.into())
            .value(normalized)
            .metadata(final_metadata)
            .call()
            .await
    }

    /// Create a categorical score
    pub async fn categorical_score(
        &self,
        trace_id: impl Into<String>,
        name: impl Into<String>,
        category: impl Into<String>,
    ) -> Result<String> {
        self.score()
            .trace_id(trace_id.into())
            .name(name.into())
            .string_value(category.into())
            .call()
            .await
    }
}
