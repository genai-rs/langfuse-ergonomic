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
    async fn ingest_events(
        &self,
        events: Vec<langfuse_client_base::models::IngestionEvent>,
    ) -> Result<langfuse_client_base::models::IngestionResponse> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::IngestionBatchRequest;

        let batch_request = IngestionBatchRequest::builder().batch(events).build();

        ingestion_api::ingestion_batch()
            .configuration(self.configuration())
            .ingestion_batch_request(batch_request)
            .call()
            .await
            .map_err(crate::error::map_api_error)
    }

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
        use langfuse_client_base::models::{
            ingestion_event_one_of::Type as TraceEventType, IngestionEvent, IngestionEventOneOf,
            TraceBody,
        };

        let trace_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = timestamp
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let tags_option = if tags.is_empty() { None } else { Some(tags) };

        let trace_body = TraceBody::builder()
            .id(Some(trace_id.clone()))
            .timestamp(Some(timestamp.clone()))
            .maybe_name(name.map(Some))
            .maybe_user_id(user_id.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_session_id(session_id.map(Some))
            .maybe_release(release.map(Some))
            .maybe_version(version.map(Some))
            .maybe_metadata(metadata.map(Some))
            .maybe_tags(tags_option.map(Some))
            .maybe_public(public.map(Some))
            .build();

        let event = IngestionEventOneOf::builder()
            .body(Box::new(trace_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.clone())
            .r#type(TraceEventType::TraceCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf(Box::new(event))])
            .await
            .map(|_| TraceResponse {
                id: trace_id,
                base_url: self.configuration().base_path.clone(),
            })
    }

    /// Get a trace by ID
    pub async fn get_trace(
        &self,
        trace_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::TraceWithFullDetails> {
        use langfuse_client_base::apis::trace_api;

        let trace_id = trace_id.into();

        trace_api::trace_get()
            .configuration(self.configuration())
            .trace_id(trace_id.as_str())
            .call()
            .await
            .map_err(crate::error::map_api_error)
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
    ) -> Result<langfuse_client_base::models::Traces> {
        use langfuse_client_base::apis::trace_api;

        let user_id_ref = user_id.as_deref();
        let name_ref = name.as_deref();
        let session_id_ref = session_id.as_deref();
        let version_ref = version.as_deref();
        let release_ref = release.as_deref();
        let order_by_ref = order_by.as_deref();
        let tags_vec = tags.map(|t| vec![t]);

        trace_api::trace_list()
            .configuration(self.configuration())
            .maybe_page(page)
            .maybe_limit(limit)
            .maybe_user_id(user_id_ref)
            .maybe_name(name_ref)
            .maybe_session_id(session_id_ref)
            .maybe_version(version_ref)
            .maybe_release(release_ref)
            .maybe_order_by(order_by_ref)
            .maybe_from_timestamp(from_timestamp)
            .maybe_to_timestamp(to_timestamp)
            .maybe_tags(tags_vec)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list traces: {}", e)))
    }

    /// Delete a trace
    pub async fn delete_trace(&self, trace_id: impl Into<String>) -> Result<()> {
        use langfuse_client_base::apis::trace_api;

        let trace_id = trace_id.into();

        trace_api::trace_delete()
            .configuration(self.configuration())
            .trace_id(trace_id.as_str())
            .call()
            .await
            .map(|_| ())
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to delete trace '{}': {}", trace_id, e))
            })
    }

    /// Delete multiple traces
    pub async fn delete_multiple_traces(&self, trace_ids: Vec<String>) -> Result<()> {
        use langfuse_client_base::apis::trace_api;
        use langfuse_client_base::models::TraceDeleteMultipleRequest;

        let trace_count = trace_ids.len();
        let request = TraceDeleteMultipleRequest::builder()
            .trace_ids(trace_ids)
            .build();

        trace_api::trace_delete_multiple()
            .configuration(self.configuration())
            .trace_delete_multiple_request(request)
            .call()
            .await
            .map(|_| ())
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to delete {} traces: {}", trace_count, e))
            })
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
        use langfuse_client_base::models::{
            ingestion_event_one_of_2::Type as SpanEventType, CreateSpanBody, IngestionEvent,
            IngestionEventOneOf2,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let level = level.map(|l| parse_observation_level(&l));
        let end_time_str = end_time.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));

        let span_body = CreateSpanBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.clone()))
            .maybe_end_time(end_time_str.map(Some))
            .maybe_name(name.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_metadata(metadata.map(Some))
            .build();

        let event = IngestionEventOneOf2::builder()
            .body(Box::new(span_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.clone())
            .r#type(SpanEventType::SpanCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf2(Box::new(event))])
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
        use langfuse_client_base::models::{
            ingestion_event_one_of_4::Type as GenerationEventType, CreateGenerationBody,
            IngestionEvent, IngestionEventOneOf4,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let level = level.map(|l| parse_observation_level(&l));
        let end_time_str = end_time.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));

        let generation_body = CreateGenerationBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.clone()))
            .maybe_name(name.map(Some))
            .maybe_end_time(end_time_str.map(Some))
            .maybe_model(model.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_metadata(metadata.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .build();

        let event = IngestionEventOneOf4::builder()
            .body(Box::new(generation_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.clone())
            .r#type(GenerationEventType::GenerationCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf4(Box::new(event))])
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
        use langfuse_client_base::models::{
            ingestion_event_one_of_6::Type as EventEventType, CreateEventBody, IngestionEvent,
            IngestionEventOneOf6,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let level = level.map(|l| parse_observation_level(&l));

        let event_body = CreateEventBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.clone()))
            .maybe_name(name.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .maybe_metadata(metadata.map(Some))
            .build();

        let event = IngestionEventOneOf6::builder()
            .body(Box::new(event_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.clone())
            .r#type(EventEventType::EventCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf6(Box::new(event))])
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create event: {}", e)))
    }

    // ===== OBSERVATION UPDATES AND RETRIEVAL =====

    /// Get a specific observation
    pub async fn get_observation(
        &self,
        observation_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::ObservationsView> {
        use langfuse_client_base::apis::observations_api;

        let observation_id = observation_id.into();

        observations_api::observations_get()
            .configuration(self.configuration())
            .observation_id(observation_id.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observation: {}", e)))
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
    ) -> Result<langfuse_client_base::models::ObservationsViews> {
        use langfuse_client_base::apis::observations_api;

        // Note: The API has more parameters but they're not all exposed in v0.2
        // Using the actual signature from the base client
        let trace_id_ref = trace_id.as_deref();
        let parent_ref = parent_observation_id.as_deref();
        let type_ref = observation_type.as_deref();
        let user_id_ref = user_id.as_deref();
        let name_ref = name.as_deref();

        observations_api::observations_get_many()
            .configuration(self.configuration())
            .maybe_page(page)
            .maybe_limit(limit)
            .maybe_trace_id(trace_id_ref)
            .maybe_parent_observation_id(parent_ref)
            .maybe_type(type_ref)
            .maybe_user_id(user_id_ref)
            .maybe_name(name_ref)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observations: {}", e)))
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
        use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf3, UpdateSpanBody};
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

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf3(Box::new(event))])
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
            IngestionEvent, IngestionEventOneOf5, UpdateGenerationBody,
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

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf5(Box::new(event))])
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
            CreateScoreValue, IngestionEvent, IngestionEventOneOf1, ScoreBody, ScoreDataType,
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

        let event = IngestionEventOneOf1 {
            body: Box::new(score_body),
            id: Uuid::new_v4().to_string(),
            timestamp: timestamp.clone(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of_1::Type::ScoreCreate,
        };

        self.ingest_events(vec![IngestionEvent::IngestionEventOneOf1(Box::new(event))])
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
    ) -> Result<langfuse_client_base::models::Dataset> {
        use langfuse_client_base::apis::datasets_api;
        use langfuse_client_base::models::CreateDatasetRequest;

        let request = CreateDatasetRequest {
            name,
            description: description.map(Some),
            metadata: metadata.map(Some),
        };

        datasets_api::datasets_create()
            .configuration(self.configuration())
            .create_dataset_request(request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create dataset: {}", e)))
    }

    /// Get a dataset by name
    pub async fn get_dataset(
        &self,
        dataset_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::Dataset> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        datasets_api::datasets_get()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset: {}", e)))
    }

    /// List datasets with pagination
    #[builder]
    pub async fn list_datasets(
        &self,
        page: Option<i32>,
        limit: Option<i32>,
    ) -> Result<langfuse_client_base::models::PaginatedDatasets> {
        use langfuse_client_base::apis::datasets_api;

        datasets_api::datasets_list()
            .configuration(self.configuration())
            .maybe_page(page)
            .maybe_limit(limit)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list datasets: {}", e)))
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

        datasets_api::datasets_delete_run()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .run_name(run_name.as_str())
            .call()
            .await
            .map(|_| ())
            .map_err(|e| crate::error::Error::Api(format!("Failed to delete dataset run: {}", e)))
    }

    /// Get a dataset run
    pub async fn get_dataset_run(
        &self,
        dataset_name: impl Into<String>,
        run_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::DatasetRunWithItems> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();
        let run_name = run_name.into();

        datasets_api::datasets_get_run()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .run_name(run_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset run: {}", e)))
    }

    /// Get all runs for a dataset
    pub async fn get_dataset_runs(
        &self,
        dataset_name: impl Into<String>,
    ) -> Result<langfuse_client_base::models::PaginatedDatasetRuns> {
        use langfuse_client_base::apis::datasets_api;

        let dataset_name = dataset_name.into();

        datasets_api::datasets_get_runs()
            .configuration(self.configuration())
            .dataset_name(dataset_name.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset runs: {}", e)))
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
    ) -> Result<langfuse_client_base::models::DatasetItem> {
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

        dataset_items_api::dataset_items_create()
            .configuration(self.configuration())
            .create_dataset_item_request(item_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create dataset item: {}", e)))
    }

    /// Get a specific dataset item
    pub async fn get_dataset_item(
        &self,
        item_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::DatasetItem> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        dataset_items_api::dataset_items_get()
            .configuration(self.configuration())
            .id(item_id.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get dataset item: {}", e)))
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
    ) -> Result<langfuse_client_base::models::PaginatedDatasetItems> {
        use langfuse_client_base::apis::dataset_items_api;

        let dataset_name_ref = dataset_name.as_deref();
        let source_trace_ref = source_trace_id.as_deref();
        let source_observation_ref = source_observation_id.as_deref();

        dataset_items_api::dataset_items_list()
            .configuration(self.configuration())
            .maybe_dataset_name(dataset_name_ref)
            .maybe_source_trace_id(source_trace_ref)
            .maybe_source_observation_id(source_observation_ref)
            .maybe_page(page)
            .maybe_limit(limit)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list dataset items: {}", e)))
    }

    /// Delete a dataset item
    pub async fn delete_dataset_item(&self, item_id: impl Into<String>) -> Result<()> {
        use langfuse_client_base::apis::dataset_items_api;

        let item_id = item_id.into();

        dataset_items_api::dataset_items_delete()
            .configuration(self.configuration())
            .id(item_id.as_str())
            .call()
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
    ) -> Result<langfuse_client_base::models::Prompt> {
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

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create prompt: {}", e)))
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
    ) -> Result<langfuse_client_base::models::Prompt> {
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

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create chat prompt: {}", e)))
    }

    /// Update labels for a specific prompt version
    #[builder]
    pub async fn update_prompt_version(
        &self,
        #[builder(into)] name: String,
        version: i32,
        labels: Vec<String>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompt_version_api;
        use langfuse_client_base::models::PromptVersionUpdateRequest;

        let update_request = PromptVersionUpdateRequest { new_labels: labels };

        prompt_version_api::prompt_version_update()
            .configuration(self.configuration())
            .name(name.as_str())
            .version(version)
            .prompt_version_update_request(update_request)
            .call()
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to update prompt version: {}", e))
            })
    }

    /// Get a prompt by name and version
    pub async fn get_prompt(
        &self,
        prompt_name: impl Into<String>,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompts_api;

        let prompt_name = prompt_name.into();

        prompts_api::prompts_get()
            .configuration(self.configuration())
            .prompt_name(prompt_name.as_str())
            .maybe_version(version)
            .maybe_label(label)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get prompt: {}", e)))
    }

    /// List prompts with filters
    #[builder]
    pub async fn list_prompts(
        &self,
        #[builder(into)] name: Option<String>,
        #[builder(into)] tag: Option<String>,
        #[builder(into)] label: Option<String>,
        page: Option<i32>,
        limit: Option<String>,
    ) -> Result<langfuse_client_base::models::PromptMetaListResponse> {
        use langfuse_client_base::apis::prompts_api;

        let name_ref = name.as_deref();
        let tag_ref = tag.as_deref();
        let label_ref = label.as_deref();
        let limit_num = limit.and_then(|value| value.parse::<i32>().ok());

        prompts_api::prompts_list()
            .configuration(self.configuration())
            .maybe_name(name_ref)
            .maybe_tag(tag_ref)
            .maybe_label(label_ref)
            .maybe_page(page)
            .maybe_limit(limit_num)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list prompts: {}", e)))
    }
}
