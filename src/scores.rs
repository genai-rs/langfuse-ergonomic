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
            CreateScoreValue, IngestionEvent, IngestionEventScoreEvent, ScoreBody, ScoreDataType,
        };

        let score_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let score_value = if let Some(v) = value {
            CreateScoreValue::DoubleNumber(v)
        } else if let Some(s) = string_value {
            CreateScoreValue::String(s)
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

        let event = IngestionEventScoreEvent {
            body: score_body,
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_score_event::Type::ScoreCreate,
        };

        self.ingest_events(vec![IngestionEvent::ScoreEvent(event)])
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

/// The value of a score: numeric, boolean or categorical.
///
/// Used by [`crate::observe::Observation::score`] and the experiment evaluators
/// ([`crate::experiments::Evaluation`]).
#[derive(Debug, Clone, PartialEq)]
pub enum ScoreValue {
    /// A number (data type `NUMERIC`).
    Numeric(f64),
    /// True or false (data type `BOOLEAN`, sent as 1 or 0).
    Boolean(bool),
    /// A category (data type `CATEGORICAL`).
    Categorical(String),
}

impl ScoreValue {
    /// The value as a number: numeric scores as they are, booleans as 1 or 0, categories `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Numeric(v) => Some(*v),
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::Categorical(_) => None,
        }
    }
}

impl From<f64> for ScoreValue {
    fn from(value: f64) -> Self {
        Self::Numeric(value)
    }
}

impl From<f32> for ScoreValue {
    fn from(value: f32) -> Self {
        Self::Numeric(f64::from(value))
    }
}

impl From<i32> for ScoreValue {
    fn from(value: i32) -> Self {
        Self::Numeric(f64::from(value))
    }
}

impl From<bool> for ScoreValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<String> for ScoreValue {
    fn from(value: String) -> Self {
        Self::Categorical(value)
    }
}

impl From<&str> for ScoreValue {
    fn from(value: &str) -> Self {
        Self::Categorical(value.to_owned())
    }
}

/// A score to send, for [`score_event`].
#[derive(Debug, Clone, Default)]
pub(crate) struct ScoreTarget {
    pub trace_id: Option<String>,
    pub observation_id: Option<String>,
    pub session_id: Option<String>,
    pub dataset_run_id: Option<String>,
}

/// Build a `score-create` ingestion event.
pub(crate) fn score_event(
    target: ScoreTarget,
    name: String,
    value: &ScoreValue,
    comment: Option<String>,
    metadata: Option<Value>,
) -> langfuse_client_base::models::IngestionEvent {
    use langfuse_client_base::models::{
        ingestion_event_score_event::Type, IngestionEvent, IngestionEventScoreEvent,
    };
    let (value, data_type) = match value {
        ScoreValue::Numeric(v) => (CreateScoreValue::DoubleNumber(*v), ScoreDataType::Numeric),
        ScoreValue::Boolean(b) => (
            CreateScoreValue::DoubleNumber(if *b { 1.0 } else { 0.0 }),
            ScoreDataType::Boolean,
        ),
        ScoreValue::Categorical(s) => (
            CreateScoreValue::String(s.clone()),
            ScoreDataType::Categorical,
        ),
    };
    let body = ScoreBody {
        id: Some(Some(Uuid::new_v4().to_string())),
        trace_id: target.trace_id.map(Some),
        session_id: target.session_id.map(Some),
        observation_id: target.observation_id.map(Some),
        dataset_run_id: target.dataset_run_id.map(Some),
        name,
        environment: None,
        queue_id: None,
        value,
        comment: comment.map(Some),
        metadata: metadata.map(Some),
        data_type: Some(data_type),
        config_id: None,
    };
    IngestionEvent::ScoreEvent(IngestionEventScoreEvent {
        body,
        id: Uuid::new_v4().to_string(),
        timestamp: crate::time::now_utc().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: Type::ScoreCreate,
    })
}
