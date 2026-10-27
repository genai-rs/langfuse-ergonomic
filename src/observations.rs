//! Observation-related functionality (spans, generations, and events)
//!
//! This module contains types and utilities for observations.
//! The client methods live here as a `#[bon] impl` block on [`LangfuseClient`].

// Re-export common types that might be useful
pub use langfuse_client_base::models::{
    CreateEventBody, CreateGenerationBody, CreateSpanBody, ObservationLevel,
};

use bon::bon;
use chrono::{DateTime, Utc};
use serde_json::Value;

use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};
use crate::traces::parse_observation_level;

#[bon]
impl LangfuseClient {
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
            create_span_event_1::Type as SpanEventType, CreateSpanBody, CreateSpanEvent1,
            IngestionEvent,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time.unwrap_or_else(Utc::now);
        let level = level.map(|l| parse_observation_level(&l));
        let end_time = end_time.map(|t| t.fixed_offset());

        let span_body = CreateSpanBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.fixed_offset()))
            .maybe_end_time(end_time.map(Some))
            .maybe_name(name.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_metadata(metadata.map(Some))
            .build();

        let event = CreateSpanEvent1::builder()
            .body(Box::new(span_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(SpanEventType::SpanCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateSpanEvent1(Box::new(event))])
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create span: {e}")))
    }

    /// Create a generation observation
    ///
    /// Token counts are sent as `usageDetails`: `prompt_tokens` as `input`,
    /// `completion_tokens` as `output`, and `total_tokens` as `total`.
    /// Only supplied counts are sent, including zeros. An explicit total is
    /// preserved; when omitted, Langfuse derives it from the supplied usage.
    /// If no counts are supplied, `usageDetails` is omitted.
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
        prompt_tokens: Option<i32>,
        completion_tokens: Option<i32>,
        total_tokens: Option<i32>,
    ) -> Result<String> {
        use langfuse_client_base::models::{
            create_generation_event_1::Type as GenerationEventType, CreateGenerationBody,
            CreateGenerationEvent1, IngestionEvent, UsageDetails,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time.unwrap_or_else(Utc::now);

        let level = level.map(|l| parse_observation_level(&l));
        let end_time = end_time.map(|t| t.fixed_offset());

        let usage_details: std::collections::HashMap<String, i32> = [
            ("input", prompt_tokens),
            ("output", completion_tokens),
            ("total", total_tokens),
        ]
        .into_iter()
        .filter_map(|(key, count)| count.map(|count| (key.to_owned(), count)))
        .collect();
        let usage_details =
            (!usage_details.is_empty()).then(|| Box::new(UsageDetails::Object(usage_details)));

        let generation_body = CreateGenerationBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.fixed_offset()))
            .maybe_name(name.map(Some))
            .maybe_end_time(end_time.map(Some))
            .maybe_model(model.map(Some))
            .maybe_usage_details(usage_details)
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_metadata(metadata.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .build();

        let event = CreateGenerationEvent1::builder()
            .body(Box::new(generation_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(GenerationEventType::GenerationCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateGenerationEvent1(Box::new(
            event,
        ))])
        .await
        .map(|_| observation_id)
        .map_err(|e| crate::error::Error::Api(format!("Failed to create generation: {e}")))
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
            create_event_event_1::Type as EventEventType, CreateEventBody, CreateEventEvent1,
            IngestionEvent,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time.unwrap_or_else(Utc::now);

        let level = level.map(|l| parse_observation_level(&l));

        let event_body = CreateEventBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.fixed_offset()))
            .maybe_name(name.map(Some))
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .maybe_metadata(metadata.map(Some))
            .build();

        let event = CreateEventEvent1::builder()
            .body(Box::new(event_body))
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(EventEventType::EventCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateEventEvent1(Box::new(event))])
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create event: {e}")))
    }

    // ===== OBSERVATION UPDATES AND RETRIEVAL =====

    /// Get a specific observation
    pub async fn get_observation(
        &self,
        observation_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::ObservationsViewSingle> {
        use langfuse_client_base::apis::legacy_observations_v1_api;

        let observation_id = observation_id.into();

        legacy_observations_v1_api::legacy_observations_v1_get()
            .configuration(self.configuration())
            .observation_id(observation_id.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observation: {e}")))
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
    ) -> Result<crate::ObservationsViews> {
        use langfuse_client_base::apis::legacy_observations_v1_api;

        // Note: The API has more parameters but they're not all exposed in v0.2
        // Using the actual signature from the base client
        let trace_id_ref = trace_id.as_deref();
        let parent_ref = parent_observation_id.as_deref();
        let type_ref = observation_type.as_deref();
        let user_id_ref = user_id.as_deref();
        let name_ref = name.as_deref();

        legacy_observations_v1_api::legacy_observations_v1_get_many()
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
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observations: {e}")))
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
        use langfuse_client_base::models::{IngestionEvent, UpdateSpanBody, UpdateSpanEvent1};
        use uuid::Uuid;

        let event_body = UpdateSpanBody {
            id: id.clone(),
            trace_id: Some(Some(trace_id)),
            name: Some(name),
            start_time: Some(start_time.map(|dt| dt.fixed_offset())),
            end_time: Some(end_time.map(|dt| dt.fixed_offset())),
            metadata: Some(metadata),
            input: Some(input),
            output: Some(output),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: Some(status_message),
            version: Some(version),
            parent_observation_id: Some(parent_observation_id),
            environment: None,
        };

        let event = UpdateSpanEvent1 {
            body: Box::new(event_body),
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type: langfuse_client_base::models::update_span_event_1::Type::SpanUpdate,
        };

        self.ingest_events(vec![IngestionEvent::UpdateSpanEvent1(Box::new(event))])
            .await
            .map_err(|e| Error::Api(format!("Failed to update span: {e}")))?;

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
            IngestionEvent, UpdateGenerationBody, UpdateGenerationEvent1,
        };
        use uuid::Uuid;

        // Note: In v0.2, model_parameters and usage have different types
        // We'll leave them out for now as they require special handling
        let event_body = UpdateGenerationBody {
            id: id.clone(),
            trace_id: Some(Some(trace_id)),
            name: Some(name),
            start_time: Some(start_time.map(|dt| dt.fixed_offset())),
            end_time: Some(end_time.map(|dt| dt.fixed_offset())),
            completion_start_time: Some(completion_start_time.map(|dt| dt.fixed_offset())),
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

        let event = UpdateGenerationEvent1 {
            body: Box::new(event_body),
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type: langfuse_client_base::models::update_generation_event_1::Type::GenerationUpdate,
        };

        self.ingest_events(vec![IngestionEvent::UpdateGenerationEvent1(Box::new(
            event,
        ))])
        .await
        .map_err(|e| Error::Api(format!("Failed to update generation: {e}")))?;

        Ok(id)
    }

    // Note: UpdateEventBody exists in v0.2 but doesn't have a corresponding IngestionEvent variant
    // This functionality will need to wait for a later version
}
