//! Trace-related functionality with builder patterns

use bon::bon;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

/// Helper trait for ergonomic tag creation
pub trait IntoTags {
    fn into_tags(self) -> Vec<String>;
}

/// Helper to convert level strings to ObservationLevel
pub fn parse_observation_level(level: &str) -> langfuse_client_base::models::ObservationLevel {
    use langfuse_client_base::models::ObservationLevel;

    match level.to_uppercase().as_str() {
        "DEBUG" => ObservationLevel::Debug,
        "INFO" | "DEFAULT" => ObservationLevel::Default, // Map INFO to Default
        "WARN" | "WARNING" => ObservationLevel::Warning,
        "ERROR" => ObservationLevel::Error,
        _ => ObservationLevel::Default, // Fallback to Default for unknown levels
    }
}

impl IntoTags for Vec<String> {
    fn into_tags(self) -> Vec<String> {
        self
    }
}

impl IntoTags for Vec<&str> {
    fn into_tags(self) -> Vec<String> {
        self.into_iter().map(|s| s.to_string()).collect()
    }
}

impl<const N: usize> IntoTags for [&str; N] {
    fn into_tags(self) -> Vec<String> {
        self.into_iter().map(|s| s.to_string()).collect()
    }
}

impl<const N: usize> IntoTags for [String; N] {
    fn into_tags(self) -> Vec<String> {
        self.into_iter().collect()
    }
}

/// Response from trace creation
pub struct TraceResponse {
    pub id: String,
    pub base_url: String,
}

impl TraceResponse {
    /// Get the Langfuse URL for this trace
    pub fn url(&self) -> String {
        // More robust URL construction that handles various base_url formats
        let mut web_url = self.base_url.clone();

        // Remove trailing slashes
        web_url = web_url.trim_end_matches('/').to_string();

        // Replace /api/public or /api at the end with empty string
        if web_url.ends_with("/api/public") {
            web_url = web_url[..web_url.len() - 11].to_string();
        } else if web_url.ends_with("/api") {
            web_url = web_url[..web_url.len() - 4].to_string();
        }

        format!("{}/trace/{}", web_url, self.id)
    }
}

/// Helper functions for generating deterministic IDs
pub struct IdGenerator;

impl IdGenerator {
    /// Generate a deterministic UUID v5 from a seed string
    /// This ensures the same seed always produces the same ID
    pub fn from_seed(seed: &str) -> String {
        // Use UUID v5 with a namespace for deterministic generation
        let namespace = Uuid::NAMESPACE_OID;
        Uuid::new_v5(&namespace, seed.as_bytes()).to_string()
    }

    /// Generate a deterministic ID from multiple components
    /// Useful for creating hierarchical IDs (e.g., trace -> span -> event)
    pub fn from_components(components: &[&str]) -> String {
        let combined = components.join(":");
        Self::from_seed(&combined)
    }

    /// Generate a deterministic ID using a hash-based approach
    /// Alternative to UUID v5 for simpler use cases
    pub fn from_hash(seed: &str) -> String {
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        let hash = hasher.finish();
        format!("{:016x}", hash)
    }
}

#[bon]
impl LangfuseClient {
    /// Create a new trace
    #[builder]
    pub async fn trace(
        &self,
        #[builder(into)] id: Option<String>,
        #[builder(into)] name: Option<String>,
        input: Option<Value>,
        output: Option<Value>,
        metadata: Option<Value>,
        #[builder(default = Vec::new())] tags: Vec<String>,
        #[builder(into)] user_id: Option<String>,
        #[builder(into)] session_id: Option<String>,
        timestamp: Option<DateTime<Utc>>,
        #[builder(into)] release: Option<String>,
        #[builder(into)] version: Option<String>,
        public: Option<bool>,
    ) -> Result<TraceResponse> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            IngestionBatchRequest, IngestionEvent, IngestionEventOneOf, TraceBody,
        };

        let trace_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = timestamp
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let trace_body = TraceBody {
            id: Some(Some(trace_id.clone())),
            timestamp: Some(Some(timestamp.clone())),
            name: name.map(Some),
            user_id: user_id.map(Some),
            input: input.map(Some),
            output: output.map(Some),
            session_id: session_id.map(Some),
            release: release.map(Some),
            version: version.map(Some),
            metadata: metadata.map(Some),
            tags: if tags.is_empty() {
                None
            } else {
                Some(Some(tags))
            },
            environment: None,
            public: public.map(Some),
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

        ingestion_api::ingestion_batch(self.configuration(), batch_request)
            .await
            .map(|_| TraceResponse {
                id: trace_id,
                base_url: self.configuration().base_path.clone(),
            })
            .map_err(crate::error::map_api_error)
    }

    /// Get a trace by ID
    pub async fn get_trace(&self, trace_id: impl Into<String>) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::trace_api;

        let trace_id = trace_id.into();

        let trace = trace_api::trace_get(self.configuration(), &trace_id)
            .await
            .map_err(crate::error::map_api_error)?;

        serde_json::to_value(trace)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize trace: {}", e)))
    }

    /// List traces with optional filters
    #[builder]
    pub async fn list_traces(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
        #[builder(into)] user_id: Option<String>,
        #[builder(into)] name: Option<String>,
        #[builder(into)] session_id: Option<String>,
        #[builder(into)] version: Option<String>,
        #[builder(into)] release: Option<String>,
        #[builder(into)] from_timestamp: Option<String>,
        #[builder(into)] to_timestamp: Option<String>,
        #[builder(into)] order_by: Option<String>,
        #[builder(into)] tags: Option<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::trace_api;

        let traces = trace_api::trace_list(
            self.configuration(),
            page,
            limit,
            user_id.as_deref(),
            name.as_deref(),
            session_id.as_deref(),
            version, // Option<String>
            release, // Option<String>
            order_by.as_deref(),
            None, // tags as Vec<String> - additional parameter
            from_timestamp.as_deref(),
            to_timestamp.as_deref(),
            None, // user_ids as Vec<String> - additional parameter
            tags.as_deref(),
        )
        .await
        .map_err(|e| crate::error::Error::Api(format!("Failed to list traces: {}", e)))?;

        serde_json::to_value(traces)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize traces: {}", e)))
    }

    /// Delete a trace
    pub async fn delete_trace(&self, trace_id: impl Into<String>) -> Result<()> {
        use langfuse_client_base::apis::trace_api;

        let trace_id = trace_id.into();

        trace_api::trace_delete(self.configuration(), &trace_id)
            .await
            .map(|_| ()) // Ignore the response body, just return success
            .map_err(|e| crate::error::Error::Api(format!("Failed to delete trace '{}': {}", trace_id, e)))
    }

    /// Delete multiple traces
    pub async fn delete_multiple_traces(&self, trace_ids: Vec<String>) -> Result<()> {
        use langfuse_client_base::apis::trace_api;
        use langfuse_client_base::models::TraceDeleteMultipleRequest;

        let trace_count = trace_ids.len();
        let request = TraceDeleteMultipleRequest {
            trace_ids, // Remove the Some() wrapper
        };

        trace_api::trace_delete_multiple(self.configuration(), request)
            .await
            .map(|_| ()) // Ignore the response body, just return success
            .map_err(|e| crate::error::Error::Api(format!("Failed to delete {} traces: {}", trace_count, e)))
    }

    // ===== OBSERVATIONS (SPANS, GENERATIONS, EVENTS) =====

    /// Create a span observation
    #[builder]
    pub async fn span(
        &self,
        #[builder(into)] trace_id: String,
        #[builder(into)] id: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
        #[builder(into)] name: Option<String>,
        input: Option<Value>,
        output: Option<Value>,
        metadata: Option<Value>,
        #[builder(into)] level: Option<String>,
        #[builder(into)] status_message: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<String> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            CreateSpanBody, IngestionBatchRequest, IngestionEvent, IngestionEventOneOf2,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let span_body = CreateSpanBody {
            id: Some(Some(observation_id.clone())),
            trace_id: Some(Some(trace_id)),
            name: name.map(Some),
            start_time: Some(Some(timestamp.clone())),
            end_time: end_time
                .map(|t| Some(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))),
            input: input.map(Some),
            output: output.map(Some),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: status_message.map(Some),
            parent_observation_id: parent_observation_id.map(Some),
            version: None,
            metadata: metadata.map(Some),
            environment: None,
        };

        let event = IngestionEventOneOf2 {
            body: Box::new(span_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_2::Type::SpanCreate,
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf2(Box::new(event))],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.configuration(), batch_request)
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create span: {}", e)))
    }

    /// Create a generation observation
    #[builder]
    pub async fn generation(
        &self,
        #[builder(into)] trace_id: String,
        #[builder(into)] id: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
        #[builder(into)] name: Option<String>,
        input: Option<Value>,
        output: Option<Value>,
        metadata: Option<Value>,
        #[builder(into)] level: Option<String>,
        #[builder(into)] status_message: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        #[builder(into)] model: Option<String>,
        _model_parameters: Option<Value>,
        _prompt_tokens: Option<i32>,
        _completion_tokens: Option<i32>,
        _total_tokens: Option<i32>,
    ) -> Result<String> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            CreateGenerationBody, IngestionBatchRequest, IngestionEvent, IngestionEventOneOf4,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let generation_body = CreateGenerationBody {
            id: Some(Some(observation_id.clone())),
            trace_id: Some(Some(trace_id)),
            name: name.map(Some),
            start_time: Some(Some(timestamp.clone())),
            completion_start_time: None,
            end_time: end_time
                .map(|t| Some(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))),
            model: model.map(Some),
            model_parameters: None, // TODO: Convert JSON to HashMap if needed
            input: input.map(Some),
            output: output.map(Some),
            usage: None, // TODO: Add usage tracking if needed
            usage_details: None,
            cost_details: None,
            metadata: metadata.map(Some),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: status_message.map(Some),
            parent_observation_id: parent_observation_id.map(Some),
            version: None,
            prompt_name: None,
            prompt_version: None,
            environment: None,
        };

        let event = IngestionEventOneOf4 {
            body: Box::new(generation_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_4::Type::GenerationCreate,
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf4(Box::new(event))],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.configuration(), batch_request)
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create generation: {}", e)))
    }

    /// Create an event observation
    #[builder]
    pub async fn event(
        &self,
        #[builder(into)] trace_id: String,
        #[builder(into)] id: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
        #[builder(into)] name: Option<String>,
        input: Option<Value>,
        output: Option<Value>,
        metadata: Option<Value>,
        #[builder(into)] level: Option<String>,
        #[builder(into)] status_message: Option<String>,
        start_time: Option<DateTime<Utc>>,
    ) -> Result<String> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            CreateEventBody, IngestionBatchRequest, IngestionEvent, IngestionEventOneOf6,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let event_body = CreateEventBody {
            id: Some(Some(observation_id.clone())),
            trace_id: Some(Some(trace_id)),
            name: name.map(Some),
            start_time: Some(Some(timestamp.clone())),
            input: input.map(Some),
            output: output.map(Some),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: status_message.map(Some),
            parent_observation_id: parent_observation_id.map(Some),
            version: None,
            metadata: metadata.map(Some),
            environment: None,
        };

        let event = IngestionEventOneOf6 {
            body: Box::new(event_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_6::Type::EventCreate,
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf6(Box::new(event))],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.configuration(), batch_request)
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create event: {}", e)))
    }

    // ===== OBSERVATION UPDATES AND RETRIEVAL =====

    /// Get a specific observation
    pub async fn get_observation(
        &self,
        observation_id: impl Into<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::observations_api;

        let observation_id = observation_id.into();

        let observation = observations_api::observations_get(self.configuration(), &observation_id)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observation: {}", e)))?;

        serde_json::to_value(observation).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize observation: {}", e))
        })
    }

    /// Get multiple observations
    #[builder]
    pub async fn get_observations(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
        #[builder(into)] trace_id: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
        #[builder(into)] name: Option<String>,
        #[builder(into)] user_id: Option<String>,
        observation_type: Option<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::observations_api;

        // Note: The API has more parameters but they're not all exposed in v0.2
        // Using the actual signature from the base client
        let observations = observations_api::observations_get_many(
            self.configuration(),
            page,
            limit,
            trace_id.as_deref(),
            parent_observation_id.as_deref(),
            observation_type.as_deref(),
            user_id.as_deref(),
            None, // observation_level
            name.as_deref(),
            None, // tags
            None, // from_start_time_str
            None, // to_start_time_str
            None, // version
        )
        .await
        .map_err(|e| crate::error::Error::Api(format!("Failed to get observations: {}", e)))?;

        serde_json::to_value(observations).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize observations: {}", e))
        })
    }

    /// Update an existing span
    #[builder]
    pub async fn update_span(
        &self,
        #[builder(into)] id: String,
        #[builder(into)] trace_id: String,
        #[builder(into)] name: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        metadata: Option<Value>,
        input: Option<Value>,
        output: Option<Value>,
        level: Option<String>,
        status_message: Option<String>,
        version: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
    ) -> Result<String> {
        use chrono::Utc as ChronoUtc;
        use langfuse_client_base::models::{
            IngestionBatchRequest, IngestionEvent, IngestionEventOneOf3, UpdateSpanBody,
        };
        use uuid::Uuid;

        let event_body = UpdateSpanBody {
            id: id.clone(),
            trace_id: Some(Some(trace_id)),
            name: Some(name),
            start_time: Some(start_time.map(|dt| dt.to_rfc3339())),
            end_time: Some(end_time.map(|dt| dt.to_rfc3339())),
            metadata: Some(metadata),
            input: Some(input),
            output: Some(output),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: Some(status_message),
            version: Some(version),
            parent_observation_id: Some(parent_observation_id),
            environment: None,
        };

        let event = IngestionEventOneOf3 {
            body: Box::new(event_body),
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_3::Type::SpanUpdate,
        };

        let batch = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf3(Box::new(event))],
            metadata: None,
        };

        use langfuse_client_base::apis::ingestion_api;
        ingestion_api::ingestion_batch(self.configuration(), batch)
            .await
            .map_err(|e| Error::Api(format!("Failed to update span: {}", e)))?;

        Ok(id)
    }

    /// Update an existing generation
    #[builder]
    pub async fn update_generation(
        &self,
        #[builder(into)] id: String,
        #[builder(into)] trace_id: String,
        #[builder(into)] name: Option<String>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        completion_start_time: Option<DateTime<Utc>>,
        model: Option<String>,
        input: Option<Value>,
        output: Option<Value>,
        metadata: Option<Value>,
        level: Option<String>,
        status_message: Option<String>,
        version: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
    ) -> Result<String> {
        use chrono::Utc as ChronoUtc;
        use langfuse_client_base::models::{
            IngestionBatchRequest, IngestionEvent, IngestionEventOneOf5, UpdateGenerationBody,
        };
        use uuid::Uuid;

        // Note: In v0.2, model_parameters and usage have different types
        // We'll leave them out for now as they require special handling
        let event_body = UpdateGenerationBody {
            id: id.clone(),
            trace_id: Some(Some(trace_id)),
            name: Some(name),
            start_time: Some(start_time.map(|dt| dt.to_rfc3339())),
            end_time: Some(end_time.map(|dt| dt.to_rfc3339())),
            completion_start_time: Some(completion_start_time.map(|dt| dt.to_rfc3339())),
            model: Some(model),
            model_parameters: None, // Requires HashMap<String, MapValue>
            input: Some(input),
            output: Some(output),
            usage: None, // Requires Box<IngestionUsage>
            metadata: Some(metadata),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: Some(status_message),
            version: Some(version),
            parent_observation_id: Some(parent_observation_id),
            environment: None,
            cost_details: None,
            prompt_name: None,
            prompt_version: None,
            usage_details: None,
        };

        let event = IngestionEventOneOf5 {
            body: Box::new(event_body),
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_5::Type::GenerationUpdate,
        };

        let batch = IngestionBatchRequest {
            batch: vec![IngestionEvent::IngestionEventOneOf5(Box::new(event))],
            metadata: None,
        };

        use langfuse_client_base::apis::ingestion_api;
        ingestion_api::ingestion_batch(self.configuration(), batch)
            .await
            .map_err(|e| Error::Api(format!("Failed to update generation: {}", e)))?;

        Ok(id)
    }

    // Note: UpdateEventBody exists in v0.2 but doesn't have a corresponding IngestionEvent variant
    // This functionality will need to wait for a later version

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
        metadata: Option<Value>,
    ) -> Result<String> {
        // Validate that either value or string_value is set
        if value.is_none() && string_value.is_none() {
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

        let score_value = if let Some(v) = value {
            Box::new(CreateScoreValue::Number(v))
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

        ingestion_api::ingestion_batch(self.configuration(), batch_request)
            .await
            .map(|_| score_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create score: {}", e)))
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
                "rating ({}) must be less than or equal to max_rating ({})",
                rating, max_rating
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

    // ===== DATASET MANAGEMENT =====

    /// Create a dataset
    #[builder]
    pub async fn create_dataset(
        &self,
        #[builder(into)] name: String,
        #[builder(into)] description: Option<String>,
        metadata: Option<Value>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::datasets_api;
        use langfuse_client_base::models::CreateDatasetRequest;

        let request = CreateDatasetRequest {
            name,
            description: description.map(Some),
            metadata: metadata.map(Some),
        };

        let dataset = datasets_api::datasets_create(self.configuration(), request)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create dataset: {}", e)))?;

        serde_json::to_value(dataset)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize dataset: {}", e)))
    }

    /// Get a dataset by name
    pub async fn get_dataset(&self, dataset_name: impl Into<String>) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        let dataset = datasets_api::datasets_get(self.configuration(), &dataset_name)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset: {}", e)))?;

        serde_json::to_value(dataset)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize dataset: {}", e)))
    }

    /// List datasets with pagination
    #[builder]
    pub async fn list_datasets(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::datasets_api;

        let datasets = datasets_api::datasets_list(self.configuration(), page, limit)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list datasets: {}", e)))?;

        serde_json::to_value(datasets)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize datasets: {}", e)))
    }

    /// Delete a dataset run
    pub async fn delete_dataset_run(
        &self,
        dataset_name: impl Into<String>,
        run_name: impl Into<String>,
    ) -> Result<()> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();
        let run_name = run_name.into();

        datasets_api::datasets_delete_run(self.configuration(), &dataset_name, &run_name)
            .await
            .map(|_| ()) // Ignore the response body, just return success
            .map_err(|e| crate::error::Error::Api(format!("Failed to delete dataset run: {}", e)))
    }

    /// Get a dataset run
    pub async fn get_dataset_run(
        &self,
        dataset_name: impl Into<String>,
        run_name: impl Into<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();
        let run_name = run_name.into();

        let run = datasets_api::datasets_get_run(self.configuration(), &dataset_name, &run_name)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset run: {}", e)))?;

        serde_json::to_value(run).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize dataset run: {}", e))
        })
    }

    /// Get all runs for a dataset
    pub async fn get_dataset_runs(
        &self,
        dataset_name: impl Into<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        let runs = datasets_api::datasets_get_runs(self.configuration(), &dataset_name, None, None)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset runs: {}", e)))?;

        serde_json::to_value(runs).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize dataset runs: {}", e))
        })
    }

    // ===== DATASET ITEM OPERATIONS =====

    /// Create a new dataset item
    #[builder]
    pub async fn create_dataset_item(
        &self,
        #[builder(into)] dataset_name: String,
        input: Option<Value>,
        expected_output: Option<Value>,
        metadata: Option<Value>,
        #[builder(into)] source_trace_id: Option<String>,
        #[builder(into)] source_observation_id: Option<String>,
        #[builder(into)] id: Option<String>,
        _status: Option<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::dataset_items_api;
        use langfuse_client_base::models::CreateDatasetItemRequest;

        let item_request = CreateDatasetItemRequest {
            dataset_name,
            input: Some(input),
            expected_output: Some(expected_output),
            metadata: Some(metadata),
            source_trace_id: Some(source_trace_id),
            source_observation_id: Some(source_observation_id),
            id: Some(id),
            status: None, // Status field requires DatasetStatus enum, not available in public API
        };

        let result = dataset_items_api::dataset_items_create(self.configuration(), item_request)
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to create dataset item: {}", e))
            })?;

        serde_json::to_value(result).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize dataset item: {}", e))
        })
    }

    /// Get a specific dataset item
    pub async fn get_dataset_item(&self, item_id: impl Into<String>) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        let item = dataset_items_api::dataset_items_get(self.configuration(), &item_id)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset item: {}", e)))?;

        serde_json::to_value(item).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize dataset item: {}", e))
        })
    }

    /// List dataset items
    #[builder]
    pub async fn list_dataset_items(
        &self,
        #[builder(into)] dataset_name: Option<String>,
        #[builder(into)] source_trace_id: Option<String>,
        #[builder(into)] source_observation_id: Option<String>,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::dataset_items_api;

        let items = dataset_items_api::dataset_items_list(
            self.configuration(),
            dataset_name.as_deref(),
            source_trace_id.as_deref(),
            source_observation_id.as_deref(),
            page,
            limit,
        )
        .await
        .map_err(|e| crate::error::Error::Api(format!("Failed to list dataset items: {}", e)))?;

        serde_json::to_value(items).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize dataset items: {}", e))
        })
    }

    /// Delete a dataset item
    pub async fn delete_dataset_item(&self, item_id: impl Into<String>) -> Result<()> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        dataset_items_api::dataset_items_delete(self.configuration(), &item_id)
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to delete dataset item: {}", e))
            })?;

        Ok(())
    }

    // Note: dataset_run_items_api doesn't exist in v0.2
    // We'll implement this when the API is available

    // ===== PROMPT MANAGEMENT =====

    /// Create a new prompt or a new version of an existing prompt
    #[builder]
    pub async fn create_prompt(
        &self,
        #[builder(into)] name: String,
        #[builder(into)] prompt: String,
        _is_active: Option<bool>,
        config: Option<Value>,
        labels: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::prompts_api;
        use langfuse_client_base::models::CreatePromptRequest;

        // Create a text prompt request using the OneOf1 variant
        use langfuse_client_base::models::CreatePromptRequestOneOf1;

        let prompt_request =
            CreatePromptRequest::CreatePromptRequestOneOf1(Box::new(CreatePromptRequestOneOf1 {
                name: name.clone(),
                prompt,
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..Default::default()
            }));

        let result = prompts_api::prompts_create(self.configuration(), prompt_request)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create prompt: {}", e)))?;

        serde_json::to_value(result)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize prompt: {}", e)))
    }

    /// Create a chat prompt with messages
    #[builder]
    pub async fn create_chat_prompt(
        &self,
        #[builder(into)] name: String,
        messages: Vec<serde_json::Value>, // Array of chat messages as JSON
        config: Option<Value>,
        labels: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::prompts_api;
        use langfuse_client_base::models::{
            ChatMessageWithPlaceholders, CreatePromptRequest, CreatePromptRequestOneOf,
        };

        // Convert JSON messages to ChatMessageWithPlaceholders
        // Since ChatMessageWithPlaceholders is an enum, we need to deserialize properly
        let chat_messages: Vec<ChatMessageWithPlaceholders> = messages
            .into_iter()
            .map(|msg| {
                // Try to deserialize the JSON into ChatMessageWithPlaceholders
                serde_json::from_value(msg).unwrap_or_else(|_| {
                    // Create a default message if parsing fails
                    ChatMessageWithPlaceholders::default()
                })
            })
            .collect();

        let prompt_request =
            CreatePromptRequest::CreatePromptRequestOneOf(Box::new(CreatePromptRequestOneOf {
                name: name.clone(),
                prompt: chat_messages,
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..Default::default()
            }));

        let result = prompts_api::prompts_create(self.configuration(), prompt_request)
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to create chat prompt: {}", e))
            })?;

        serde_json::to_value(result)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize prompt: {}", e)))
    }

    /// Update labels for a specific prompt version
    #[builder]
    pub async fn update_prompt_version(
        &self,
        #[builder(into)] name: String,
        version: i32,
        labels: Vec<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::prompt_version_api;
        use langfuse_client_base::models::PromptVersionUpdateRequest;

        let update_request = PromptVersionUpdateRequest { new_labels: labels };

        let result = prompt_version_api::prompt_version_update(
            self.configuration(),
            &name,
            version,
            update_request,
        )
        .await
        .map_err(|e| crate::error::Error::Api(format!("Failed to update prompt version: {}", e)))?;

        serde_json::to_value(result).map_err(|e| {
            crate::error::Error::Api(format!("Failed to serialize prompt version: {}", e))
        })
    }

    /// Get a prompt by name and version
    pub async fn get_prompt(
        &self,
        prompt_name: impl Into<String>,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::prompts_api;

        let prompt_name = prompt_name.into();

        let prompt = prompts_api::prompts_get(self.configuration(), &prompt_name, version, label)
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get prompt: {}", e)))?;

        serde_json::to_value(prompt)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize prompt: {}", e)))
    }

    /// List prompts with filters
    #[builder]
    pub async fn list_prompts(
        &self,
        #[builder(into)] name: Option<String>,
        #[builder(into)] tag: Option<String>,
        #[builder(into)] label: Option<String>,
        version: Option<i32>,
        page: Option<i32>,
        limit: Option<String>,
    ) -> Result<serde_json::Value> {
        use langfuse_client_base::apis::prompts_api;

        let prompts = prompts_api::prompts_list(
            self.configuration(),
            name.as_deref(),
            tag.as_deref(),
            label.as_deref(),
            version,
            page,
            limit,
            None, // Additional parameter
        )
        .await
        .map_err(|e| crate::error::Error::Api(format!("Failed to list prompts: {}", e)))?;

        serde_json::to_value(prompts)
            .map_err(|e| crate::error::Error::Api(format!("Failed to serialize prompts: {}", e)))
    }
}
