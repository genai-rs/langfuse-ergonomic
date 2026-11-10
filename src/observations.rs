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

/// Convert a JSON object of model parameters into the ingestion API's map type.
pub(crate) fn model_parameters_map(
    value: &Value,
) -> Option<std::collections::HashMap<String, langfuse_client_base::models::MapValue>> {
    use langfuse_client_base::models::MapValue;

    let Value::Object(map) = value else {
        tracing::warn!("model_parameters must be a JSON object; ignoring {value}");
        return None;
    };
    let converted = map
        .iter()
        .filter(|(_, v)| !v.is_null())
        .map(|(key, v)| {
            let mapped = match v {
                Value::String(s) => MapValue::String(s.clone()),
                Value::Bool(b) => MapValue::Boolean(*b),
                Value::Number(n) => match n.as_i64() {
                    Some(i) => MapValue::Integer(i),
                    #[allow(clippy::cast_possible_truncation)]
                    None => MapValue::FloatNumber(n.as_f64().unwrap_or_default() as f32),
                },
                Value::Array(items) if items.iter().all(Value::is_string) => MapValue::Array(
                    items
                        .iter()
                        .filter_map(|i| i.as_str().map(str::to_owned))
                        .collect(),
                ),
                other => MapValue::String(other.to_string()),
            };
            (key.clone(), mapped)
        })
        .collect::<std::collections::HashMap<_, _>>();
    (!converted.is_empty()).then_some(converted)
}

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
            ingestion_event_create_span_event::Type as SpanEventType, CreateSpanBody,
            IngestionEvent, IngestionEventCreateSpanEvent,
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

        let event = IngestionEventCreateSpanEvent::builder()
            .body(span_body)
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(SpanEventType::SpanCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateSpanEvent(event)])
            .await
            .map(|_| self.effective_observation_id(&observation_id))
            .map_err(|e| crate::error::Error::Api(format!("Failed to create span: {e}")))
    }

    /// Create a generation observation
    ///
    /// Token counts are sent as `usageDetails`: `prompt_tokens` as `input`,
    /// `completion_tokens` as `output`, and `total_tokens` as `total`.
    /// Only supplied counts are sent, including zeros. An explicit total is
    /// preserved; when omitted, Langfuse derives it from the supplied usage.
    /// If no counts are supplied, `usageDetails` is omitted.
    ///
    /// `model_parameters` must be a JSON object; its values are sent as strings, numbers,
    /// booleans or string arrays (other values are sent as JSON strings).
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
            ingestion_event_create_generation_event::Type as GenerationEventType,
            CreateGenerationBody, IngestionEvent, IngestionEventCreateGenerationEvent,
            UsageDetails,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = start_time.unwrap_or_else(Utc::now);

        let level = level.map(|l| parse_observation_level(&l));
        let end_time = end_time.map(|t| t.fixed_offset());

        let usage_details: std::collections::HashMap<String, i64> = [
            ("input", prompt_tokens),
            ("output", completion_tokens),
            ("total", total_tokens),
        ]
        .into_iter()
        .filter_map(|(key, count)| count.map(|count| (key.to_owned(), i64::from(count))))
        .collect();
        let usage_details =
            (!usage_details.is_empty()).then_some(UsageDetails::Object(usage_details));

        let model_parameters = _model_parameters.as_ref().and_then(model_parameters_map);

        let generation_body = CreateGenerationBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id))
            .start_time(Some(timestamp.fixed_offset()))
            .maybe_name(name.map(Some))
            .maybe_end_time(end_time.map(Some))
            .maybe_model(model.map(Some))
            .maybe_model_parameters(model_parameters.map(Some))
            .maybe_usage_details(usage_details)
            .maybe_input(input.map(Some))
            .maybe_output(output.map(Some))
            .maybe_metadata(metadata.map(Some))
            .maybe_level(level)
            .maybe_status_message(status_message.map(Some))
            .maybe_parent_observation_id(parent_observation_id.map(Some))
            .build();

        let event = IngestionEventCreateGenerationEvent::builder()
            .body(generation_body)
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(GenerationEventType::GenerationCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateGenerationEvent(event)])
            .await
            .map(|_| self.effective_observation_id(&observation_id))
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
            ingestion_event_create_event_event::Type as EventEventType, CreateEventBody,
            IngestionEvent, IngestionEventCreateEventEvent,
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

        let event = IngestionEventCreateEventEvent::builder()
            .body(event_body)
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(EventEventType::EventCreate)
            .build();

        self.ingest_events(vec![IngestionEvent::CreateEventEvent(event)])
            .await
            .map(|_| self.effective_observation_id(&observation_id))
            .map_err(|e| crate::error::Error::Api(format!("Failed to create event: {e}")))
    }

    // ===== OBSERVATION UPDATES AND RETRIEVAL =====

    /// Get a specific observation (Langfuse v3 only)
    ///
    /// Calls the legacy `GET /api/public/observations/{id}`, which Langfuse v4 removed
    /// (Langfuse Cloud: 2026-11-16). With [`crate::ApiVersion::V4`] this returns
    /// [`Error::Configuration`] without a request; use [`LangfuseClient::observations`] or
    /// [`LangfuseClient::trace_observations`] instead.
    #[deprecated(
        note = "Langfuse v3 only; fails with ApiVersion::V4. Use `observations()` instead."
    )]
    pub async fn get_observation(
        &self,
        observation_id: impl Into<String>,
    ) -> Result<langfuse_client_base::models::ObservationsViewSingle> {
        use langfuse_client_base::apis::legacy_observations_v1_api;

        self.require_v3("get_observation", "use observations()")?;
        let observation_id = observation_id.into();

        legacy_observations_v1_api::legacy_observations_v1_get()
            .configuration(self.configuration())
            .observation_id(observation_id.as_str())
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get observation: {e}")))
    }

    /// Get multiple observations (Langfuse v3 only)
    ///
    /// Calls the legacy `GET /api/public/observations`, which Langfuse v4 removed
    /// (Langfuse Cloud: 2026-11-16). With [`crate::ApiVersion::V4`] this returns
    /// [`Error::Configuration`] without a request; use [`LangfuseClient::observations`]
    /// instead.
    #[deprecated(
        note = "Langfuse v3 only; fails with ApiVersion::V4. Use `observations()` instead."
    )]
    pub fn get_observations(&self) -> LangfuseClientGetObservationsBuilder<'_> {
        self.get_observations_v3()
    }

    #[builder(
        builder_type(name = LangfuseClientGetObservationsBuilder, vis = "pub"),
        start_fn(name = get_observations_v3, vis = "pub(crate)")
    )]
    async fn get_observations_impl(
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

        self.require_v3("get_observations", "use observations()")?;

        // Note: The API has more parameters but they're not all exposed in v0.2
        // Using the actual signature from the base client
        let trace_id_ref = trace_id.as_deref();
        let parent_ref = parent_observation_id.as_deref();
        let type_ref = observation_type.as_deref();
        let user_id_ref = user_id.as_deref();
        let name_ref = name.as_deref();

        legacy_observations_v1_api::legacy_observations_v1_get_many()
            .configuration(self.configuration())
            .maybe_page(page.map(i64::from))
            .maybe_limit(limit.map(i64::from))
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
        use langfuse_client_base::models::{
            IngestionEvent, IngestionEventUpdateSpanEvent, UpdateSpanBody,
        };
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

        let event = IngestionEventUpdateSpanEvent {
            body: event_body,
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type:
                langfuse_client_base::models::ingestion_event_update_span_event::Type::SpanUpdate,
        };

        self.ingest_events(vec![IngestionEvent::UpdateSpanEvent(event)])
            .await
            .map_err(|e| Error::Api(format!("Failed to update span: {e}")))?;

        Ok(self.effective_observation_id(&id))
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
            IngestionEvent, IngestionEventUpdateGenerationEvent, UpdateGenerationBody,
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

        let event = IngestionEventUpdateGenerationEvent {
            body: event_body,
            id: Uuid::new_v4().to_string(),
            timestamp: ChronoUtc::now().to_rfc3339(),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_update_generation_event::Type::GenerationUpdate,
        };

        self.ingest_events(vec![IngestionEvent::UpdateGenerationEvent(event)])
            .await
            .map_err(|e| Error::Api(format!("Failed to update generation: {e}")))?;

        Ok(self.effective_observation_id(&id))
    }

    // Note: UpdateEventBody exists in v0.2 but doesn't have a corresponding IngestionEvent variant
    // This functionality will need to wait for a later version
}

/// Every field group of `GET /api/public/v2/observations`, for the `fields` parameter.
pub const ALL_OBSERVATION_FIELDS: &str =
    "core,basic,time,io,metadata,model,usage,prompt,metrics,trace_context";

/// Page size [`LangfuseClient::trace_observations`] requests.
const TRACE_OBSERVATIONS_PAGE_SIZE: u32 = 100;

/// One page of `GET /api/public/v2/observations`.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationsPage {
    /// The observations on this page. A page may hold fewer items than the limit, or none,
    /// while a next page exists.
    pub data: Vec<langfuse_client_base::models::ObservationV2>,
    /// Cursor for the next page; `None` on the last page. Pass it to
    /// [`LangfuseClientObservationsBuilder::cursor`].
    pub next_cursor: Option<String>,
}

impl ObservationsPage {
    /// Whether another page follows.
    pub fn has_next_page(&self) -> bool {
        self.next_cursor.is_some()
    }
}

impl From<langfuse_client_base::models::ObservationsV2Response> for ObservationsPage {
    fn from(response: langfuse_client_base::models::ObservationsV2Response) -> Self {
        Self {
            data: response.data,
            next_cursor: response.meta.cursor.flatten().filter(|c| !c.is_empty()),
        }
    }
}

#[bon]
impl LangfuseClient {
    // ===== OBSERVATIONS API V2 (LANGFUSE V4 READS) =====

    /// List observations with `GET /api/public/v2/observations` (cursor pagination).
    ///
    /// This is the Langfuse v4 read API for traces, observations and sessions. A trace is its
    /// root observation (`.is_root_observation(true)`). Without `fields`, Langfuse returns the
    /// `core` and `basic` field groups; pass [`ALL_OBSERVATION_FIELDS`] for everything.
    ///
    /// With [`crate::ApiVersion::V4`], `trace_id` and `parent_observation_id` go through the
    /// same id mapping as writes (see [`crate::otel`]), so the ids you wrote with work here.
    ///
    /// ```no_run
    /// # use langfuse_ergonomic::ClientBuilder;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClientBuilder::from_env()?.build()?;
    /// let mut cursor = None;
    /// loop {
    ///     let page = client
    ///         .observations()
    ///         .is_root_observation(true)
    ///         .from_start_time(chrono::Utc::now() - chrono::Duration::days(1))
    ///         .limit(50)
    ///         .maybe_cursor(cursor)
    ///         .call()
    ///         .await?;
    ///     for observation in &page.data {
    ///         println!("{} {:?}", observation.id, observation.name);
    ///     }
    ///     match page.next_cursor {
    ///         Some(next) => cursor = Some(next),
    ///         None => break,
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub async fn observations(
        &self,
        /// Maximum number of items per page.
        limit: Option<u32>,
        /// Cursor from [`ObservationsPage::next_cursor`].
        #[builder(into)]
        cursor: Option<String>,
        #[builder(into)] trace_id: Option<String>,
        #[builder(into)] parent_observation_id: Option<String>,
        /// Only root observations (`true`) or only children (`false`).
        is_root_observation: Option<bool>,
        /// Inclusive lower bound on the start time.
        from_start_time: Option<DateTime<Utc>>,
        /// Exclusive upper bound on the start time.
        to_start_time: Option<DateTime<Utc>>,
        #[builder(into)] user_id: Option<String>,
        #[builder(into)] session_id: Option<String>,
        /// Observation type, for example `GENERATION`.
        #[builder(into)]
        observation_type: Option<String>,
        #[builder(into)] name: Option<String>,
        level: Option<ObservationLevel>,
        /// Environments to include.
        environment: Option<Vec<String>>,
        #[builder(into)] version: Option<String>,
        /// Comma-separated field groups (see [`ALL_OBSERVATION_FIELDS`]).
        #[builder(into)]
        fields: Option<String>,
        /// Comma-separated metadata keys to return untruncated.
        #[builder(into)]
        expand_metadata: Option<String>,
        /// Parse input and output as JSON.
        parse_io_as_json: Option<bool>,
        /// Structured JSON filter; takes precedence over the individual filters.
        #[builder(into)]
        filter: Option<String>,
    ) -> Result<ObservationsPage> {
        use langfuse_client_base::apis::observations_api;

        let trace_id = trace_id.map(|id| self.effective_trace_id(&id));
        let parent_observation_id =
            parent_observation_id.map(|id| self.effective_observation_id(&id));

        observations_api::observations_get_many()
            .configuration(self.configuration())
            .maybe_fields(fields.as_deref())
            .maybe_expand_metadata(expand_metadata.as_deref())
            .maybe_limit(limit.map(i64::from))
            .maybe_cursor(cursor.as_deref())
            .maybe_parse_io_as_json(parse_io_as_json)
            .maybe_name(name.as_deref())
            .maybe_user_id(user_id.as_deref())
            .maybe_session_id(session_id.as_deref())
            .maybe_type(observation_type.as_deref())
            .maybe_trace_id(trace_id.as_deref())
            .maybe_level(level)
            .maybe_parent_observation_id(parent_observation_id.as_deref())
            .maybe_is_root_observation(is_root_observation)
            .maybe_environment(environment)
            .maybe_from_start_time(from_start_time.map(|t| t.fixed_offset()))
            .maybe_to_start_time(to_start_time.map(|t| t.fixed_offset()))
            .maybe_version(version.as_deref())
            .maybe_filter(filter.as_deref())
            .call()
            .await
            .map(ObservationsPage::from)
            .map_err(crate::error::map_api_error)
    }

    /// Fetch every observation of a trace, following the cursor across pages.
    ///
    /// Requests all field groups ([`ALL_OBSERVATION_FIELDS`]). The trace's root observation
    /// carries the trace-level input and output. This replaces [`LangfuseClient::get_trace`]
    /// for Langfuse v4.
    pub async fn trace_observations(
        &self,
        trace_id: impl Into<String>,
    ) -> Result<Vec<langfuse_client_base::models::ObservationV2>> {
        let trace_id = trace_id.into();
        let mut observations = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let page = self
                .observations()
                .trace_id(trace_id.as_str())
                .fields(ALL_OBSERVATION_FIELDS)
                .limit(TRACE_OBSERVATIONS_PAGE_SIZE)
                .maybe_cursor(cursor.clone())
                .call()
                .await?;
            observations.extend(page.data);
            match page.next_cursor {
                Some(next) if cursor.as_deref() != Some(next.as_str()) => cursor = Some(next),
                Some(_) => {
                    return Err(Error::Api(
                        "observations API returned the same cursor twice".to_string(),
                    ))
                }
                None => return Ok(observations),
            }
        }
    }
}
