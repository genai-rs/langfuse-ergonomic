//! Observation-related functionality (spans, generations, and events)
//!
//! This module contains types and utilities for observations.
//! The client methods live here as a `#[bon] impl` block on [`LangfuseClient`].

// Re-export common types that might be useful
pub use langfuse_client_base::models::{
    CreateEventBody, CreateGenerationBody, CreateSpanBody, ObservationLevel, ObservationType,
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

/// Every field an observation create event can carry, for [`observation_event`].
#[derive(Debug, Clone)]
pub(crate) struct ObservationFields {
    pub kind: ObservationType,
    pub id: String,
    pub trace_id: String,
    pub parent_observation_id: Option<String>,
    pub name: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub completion_start_time: Option<DateTime<Utc>>,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub metadata: Option<Value>,
    pub level: Option<ObservationLevel>,
    pub status_message: Option<String>,
    pub version: Option<String>,
    pub environment: Option<String>,
    pub model: Option<String>,
    pub model_parameters: Option<Value>,
    pub usage_details: Option<std::collections::HashMap<String, i64>>,
    pub cost_details: Option<std::collections::HashMap<String, f64>>,
    pub prompt_name: Option<String>,
    pub prompt_version: Option<i64>,
}

impl ObservationFields {
    pub(crate) fn new(kind: ObservationType, id: String, trace_id: String) -> Self {
        Self {
            kind,
            id,
            trace_id,
            parent_observation_id: None,
            name: None,
            start_time: crate::time::now_utc(),
            end_time: None,
            completion_start_time: None,
            input: None,
            output: None,
            metadata: None,
            level: None,
            status_message: None,
            version: None,
            environment: None,
            model: None,
            model_parameters: None,
            usage_details: None,
            cost_details: None,
            prompt_name: None,
            prompt_version: None,
        }
    }
}

/// Build the create event of one observation, in the shape both API versions understand:
/// `span-create`, `generation-create` and `event-create` for those three types, and
/// `observation-create` (with the type) for agent, tool, chain, retriever, evaluator, embedding
/// and guardrail. Usage and cost details are sent in full on generations; other types send the
/// `input`/`output`/`total` counts and costs as legacy `usage`.
pub(crate) fn observation_event(
    f: ObservationFields,
) -> langfuse_client_base::models::IngestionEvent {
    use langfuse_client_base::models::{
        ingestion_event_create_event_event, ingestion_event_create_generation_event,
        ingestion_event_create_observation_event, ingestion_event_create_span_event,
        IngestionEvent, IngestionEventCreateEventEvent, IngestionEventCreateGenerationEvent,
        IngestionEventCreateObservationEvent, IngestionEventCreateSpanEvent, ObservationBody,
        Usage, UsageDetails,
    };

    let envelope_id = Uuid::new_v4().to_string();
    let timestamp = f
        .start_time
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let start = Some(Some(f.start_time.fixed_offset()));
    let end = f.end_time.map(|t| Some(t.fixed_offset()));
    let completion_start = f.completion_start_time.map(|t| Some(t.fixed_offset()));
    let some = |v: Option<String>| v.map(Some);
    let model_parameters = f
        .model_parameters
        .as_ref()
        .and_then(model_parameters_map)
        .map(Some);

    match f.kind {
        ObservationType::Span => {
            IngestionEvent::CreateSpanEvent(IngestionEventCreateSpanEvent::new(
                envelope_id,
                timestamp,
                CreateSpanBody {
                    id: Some(Some(f.id)),
                    trace_id: Some(Some(f.trace_id)),
                    name: some(f.name),
                    start_time: start,
                    end_time: end,
                    metadata: f.metadata.map(Some),
                    input: f.input.map(Some),
                    output: f.output.map(Some),
                    level: f.level,
                    status_message: some(f.status_message),
                    parent_observation_id: some(f.parent_observation_id),
                    version: some(f.version),
                    environment: some(f.environment),
                },
                ingestion_event_create_span_event::Type::SpanCreate,
            ))
        }
        ObservationType::Generation => {
            IngestionEvent::CreateGenerationEvent(IngestionEventCreateGenerationEvent::new(
                envelope_id,
                timestamp,
                CreateGenerationBody {
                    id: Some(Some(f.id)),
                    trace_id: Some(Some(f.trace_id)),
                    name: some(f.name),
                    start_time: start,
                    end_time: end,
                    completion_start_time: completion_start,
                    metadata: f.metadata.map(Some),
                    input: f.input.map(Some),
                    output: f.output.map(Some),
                    level: f.level,
                    status_message: some(f.status_message),
                    parent_observation_id: some(f.parent_observation_id),
                    version: some(f.version),
                    environment: some(f.environment),
                    model: some(f.model),
                    model_parameters,
                    usage: None,
                    usage_details: f
                        .usage_details
                        .filter(|u| !u.is_empty())
                        .map(UsageDetails::Object),
                    cost_details: f.cost_details.filter(|c| !c.is_empty()).map(Some),
                    prompt_name: some(f.prompt_name),
                    prompt_version: f.prompt_version.map(Some),
                },
                ingestion_event_create_generation_event::Type::GenerationCreate,
            ))
        }
        ObservationType::Event => {
            IngestionEvent::CreateEventEvent(IngestionEventCreateEventEvent::new(
                envelope_id,
                timestamp,
                CreateEventBody {
                    id: Some(Some(f.id)),
                    trace_id: Some(Some(f.trace_id)),
                    name: some(f.name),
                    start_time: start,
                    metadata: f.metadata.map(Some),
                    input: f.input.map(Some),
                    output: f.output.map(Some),
                    level: f.level,
                    status_message: some(f.status_message),
                    parent_observation_id: some(f.parent_observation_id),
                    version: some(f.version),
                    environment: some(f.environment),
                },
                ingestion_event_create_event_event::Type::EventCreate,
            ))
        }
        kind => {
            let usage = f.usage_details.as_ref().map(|u| {
                let count = |key: &str| u.get(key).copied().unwrap_or_default();
                let cost = |key: &str| f.cost_details.as_ref().and_then(|c| c.get(key).copied());
                Usage {
                    input: count("input"),
                    output: count("output"),
                    total: u
                        .get("total")
                        .copied()
                        .unwrap_or_else(|| count("input") + count("output")),
                    unit: None,
                    input_cost: cost("input").map(Some),
                    output_cost: cost("output").map(Some),
                    total_cost: cost("total").map(Some),
                }
            });
            IngestionEvent::CreateObservationEvent(IngestionEventCreateObservationEvent::new(
                envelope_id,
                timestamp,
                ObservationBody {
                    id: Some(Some(f.id)),
                    trace_id: Some(Some(f.trace_id)),
                    r#type: kind,
                    name: some(f.name),
                    start_time: start,
                    end_time: end,
                    completion_start_time: completion_start,
                    model: some(f.model),
                    model_parameters,
                    input: f.input.map(Some),
                    version: some(f.version),
                    metadata: f.metadata.map(Some),
                    output: f.output.map(Some),
                    usage,
                    level: f.level,
                    status_message: some(f.status_message),
                    parent_observation_id: some(f.parent_observation_id),
                    environment: some(f.environment),
                },
                ingestion_event_create_observation_event::Type::ObservationCreate,
            ))
        }
    }
}

/// Builder of [`LangfuseClient::observation`] with the type already set, returned by
/// [`LangfuseClient::agent`], [`LangfuseClient::tool`] and the other typed shortcuts.
pub type LangfuseClientTypedObservationBuilder<'a> =
    LangfuseClientObservationBuilder<'a, langfuse_client_observation_builder::SetAsType>;

macro_rules! typed_observation {
    ($($(#[$doc:meta])* $method:ident => $kind:ident,)*) => {
        impl LangfuseClient {
            $(
                $(#[$doc])*
                ///
                /// A shortcut for [`LangfuseClient::observation`] with `as_type` set; see there
                /// for the fields and how the type is sent with each API version.
                pub fn $method(&self) -> LangfuseClientTypedObservationBuilder<'_> {
                    self.observation().as_type(ObservationType::$kind)
                }
            )*
        }
    };
}

typed_observation! {
    /// Create an agent observation (an LLM-driven agent deciding on steps and tool calls).
    agent => Agent,
    /// Create a tool observation (a function or API call made by an agent or a chain).
    tool => Tool,
    /// Create a chain observation (a fixed sequence of steps).
    chain => Chain,
    /// Create a retriever observation (a lookup in a vector store, search index or database).
    retriever => Retriever,
    /// Create an evaluator observation (an LLM-as-a-judge or other check of an output).
    evaluator => Evaluator,
    /// Create an embedding observation (a call to an embedding model).
    embedding => Embedding,
    /// Create a guardrail observation (a check that blocks or rewrites unsafe content).
    guardrail => Guardrail,
}

#[bon]
impl LangfuseClient {
    // ===== OBSERVATIONS (SPANS, GENERATIONS, EVENTS) =====

    /// Create an observation of any type ([`ObservationType`]): span, generation, event, agent,
    /// tool, chain, retriever, evaluator, embedding or guardrail.
    ///
    /// The typed shortcuts [`LangfuseClient::agent`], [`LangfuseClient::tool`], ... set the
    /// type for you. Spans, generations and events are sent like [`LangfuseClient::span`],
    /// [`LangfuseClient::generation`] and [`LangfuseClient::event`]; the other types are sent as
    /// `observation-create` events with their type (V3) or as spans with
    /// `langfuse.observation.type` set to it (V4). The generation fields (`model`,
    /// `usage_details`, `cost_details`, `completion_start_time`, `prompt_name`/`prompt_version`)
    /// are sent in full on generations; other types keep `model`, `model_parameters`,
    /// `completion_start_time` and the `input`/`output`/`total` usage and costs.
    ///
    /// Returns the observation id (the mapped OpenTelemetry span id with
    /// [`crate::ApiVersion::V4`]).
    ///
    /// ```no_run
    /// # use langfuse_ergonomic::ClientBuilder;
    /// # use serde_json::json;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClientBuilder::from_env()?.build()?;
    /// let tool_id = client
    ///     .tool()
    ///     .trace_id("trace-1")
    ///     .name("weather-lookup")
    ///     .input(json!({"city": "Ghent"}))
    ///     .output(json!({"celsius": 12}))
    ///     .call()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub async fn observation(
        &self,
        /// The observation type.
        as_type: ObservationType,
        #[builder(into)] trace_id: String,
        /// Trace user id. V4 stamps it on every span of the trace; V3 upserts the trace.
        #[builder(into)]
        user_id: Option<String>,
        /// Trace session id. V4 stamps it on every span of the trace; V3 upserts the trace.
        #[builder(into)]
        session_id: Option<String>,
        /// Trace tags. V4 stamps them on every span of the trace; V3 upserts the trace.
        tags: Option<Vec<String>>,
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
        #[builder(into)] version: Option<String>,
        #[builder(into)] model: Option<String>,
        /// A JSON object of model parameters.
        model_parameters: Option<Value>,
        /// When the first token arrived (time to first token).
        completion_start_time: Option<DateTime<Utc>>,
        /// Token counts by usage type (`input`, `output`, `total`, `cache_read_input_tokens`, ...).
        usage_details: Option<std::collections::HashMap<String, i64>>,
        /// Costs in USD by usage type (`input`, `output`, `total`, ...).
        cost_details: Option<std::collections::HashMap<String, f64>>,
        /// The Langfuse prompt this generation used (see [`crate::prompts::FetchedPrompt`]).
        #[builder(into)]
        prompt_name: Option<String>,
        /// The version of `prompt_name`.
        prompt_version: Option<i64>,
    ) -> Result<String> {
        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut fields = ObservationFields::new(as_type, observation_id.clone(), trace_id.clone());
        if let Some(start_time) = start_time {
            fields.start_time = start_time;
        }
        fields.parent_observation_id = parent_observation_id;
        fields.name = name;
        fields.end_time = end_time;
        fields.completion_start_time = completion_start_time;
        fields.input = input;
        fields.output = output;
        fields.metadata = metadata;
        fields.level = level.map(|l| parse_observation_level(&l));
        fields.status_message = status_message;
        fields.version = version;
        fields.model = model;
        fields.model_parameters = model_parameters;
        fields.usage_details = usage_details;
        fields.cost_details = cost_details;
        fields.prompt_name = prompt_name;
        fields.prompt_version = prompt_version;

        let mut events = self.trace_attribute_events(&trace_id, user_id, session_id, tags);
        events.push(observation_event(fields));
        self.ingest_events(events)
            .await
            .map(|_| self.effective_observation_id(&observation_id))
            .map_err(|e| Error::Api(format!("Failed to create {as_type} observation: {e}")))
    }

    /// Create a span observation
    ///
    /// With [`crate::ApiVersion::V4`] the returned id is the mapped OpenTelemetry span id (see
    /// [`crate::otel`]); passing your own id again resolves to the same observation. A
    /// parentless observation becomes a child of the trace's root span, or the root itself if
    /// this client has not written the trace yet. Writing it once, complete with `end_time`
    /// and output, is the preferred V4 pattern; the export only means Langfuse queued it.
    #[builder]
    pub async fn span(
        &self,
        #[builder(into)] trace_id: String,
        /// Trace user id. V4 stamps it on every span of the trace; V3 upserts the trace (which,
        /// before `trace()`, fixes the trace timestamp: v3 does not change it afterwards).
        #[builder(into)]
        user_id: Option<String>,
        /// Trace session id. V4 stamps it on every span of the trace; V3 upserts the trace.
        #[builder(into)]
        session_id: Option<String>,
        /// Trace tags. V4 stamps them on every span of the trace; V3 upserts the trace.
        tags: Option<Vec<String>>,
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
        let trace_id_for_context = trace_id;
        let timestamp = start_time.unwrap_or_else(Utc::now);
        let level = level.map(|l| parse_observation_level(&l));
        let end_time = end_time.map(|t| t.fixed_offset());

        let span_body = CreateSpanBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id_for_context.clone()))
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

        let mut events =
            self.trace_attribute_events(&trace_id_for_context, user_id, session_id, tags);
        events.push(IngestionEvent::CreateSpanEvent(event));
        self.ingest_events(events)
            .await
            .map(|_| self.effective_observation_id(&observation_id))
            .map_err(|e| crate::error::Error::Api(format!("Failed to create span: {e}")))
    }

    /// Create a generation observation
    ///
    /// Token counts are sent as `usageDetails`: the `usage_details` map, with `prompt_tokens`
    /// as `input`, `completion_tokens` as `output`, and `total_tokens` as `total` on top.
    /// Only supplied counts are sent, including zeros. An explicit total is
    /// preserved; when omitted, Langfuse derives it from the supplied usage.
    /// If no counts are supplied, `usageDetails` is omitted. `cost_details` are sent as
    /// `costDetails` (Langfuse infers costs from the model otherwise), `completion_start_time`
    /// records the time to first token, and `prompt_name`/`prompt_version` link the generation
    /// to a Langfuse prompt.
    ///
    /// With [`crate::ApiVersion::V4`] the returned id is the mapped OpenTelemetry span id (see
    /// [`crate::otel`]); passing your own id again resolves to the same observation. A
    /// parentless observation becomes a child of the trace's root span, or the root itself if
    /// this client has not written the trace yet. Writing it once, complete with `end_time`
    /// and output, is the preferred V4 pattern; the export only means Langfuse queued it.
    ///
    /// `model_parameters` must be a JSON object; its values are sent as strings, numbers,
    /// booleans or string arrays (other values are sent as JSON strings).
    #[builder]
    pub async fn generation(
        &self,
        #[builder(into)] trace_id: String,
        /// Trace user id. V4 stamps it on every span of the trace; V3 upserts the trace (which,
        /// before `trace()`, fixes the trace timestamp: v3 does not change it afterwards).
        #[builder(into)]
        user_id: Option<String>,
        /// Trace session id. V4 stamps it on every span of the trace; V3 upserts the trace.
        #[builder(into)]
        session_id: Option<String>,
        /// Trace tags. V4 stamps them on every span of the trace; V3 upserts the trace.
        tags: Option<Vec<String>>,
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
        /// Token counts by usage type (`input`, `output`, `total`, `cache_read_input_tokens`,
        /// ...), sent as `usageDetails`. `prompt_tokens`, `completion_tokens` and
        /// `total_tokens`, when given, set `input`, `output` and `total` on top of it.
        usage_details: Option<std::collections::HashMap<String, i64>>,
        /// Costs in USD by usage type (`input`, `output`, `total`, ...), sent as `costDetails`.
        cost_details: Option<std::collections::HashMap<String, f64>>,
        /// When the first token arrived (time to first token).
        completion_start_time: Option<DateTime<Utc>>,
        /// The Langfuse prompt this generation used (see [`crate::prompts::FetchedPrompt`]).
        #[builder(into)]
        prompt_name: Option<String>,
        /// The version of `prompt_name`.
        prompt_version: Option<i64>,
        #[builder(into)] version: Option<String>,
    ) -> Result<String> {
        use langfuse_client_base::models::{
            ingestion_event_create_generation_event::Type as GenerationEventType,
            CreateGenerationBody, IngestionEvent, IngestionEventCreateGenerationEvent,
            UsageDetails,
        };

        let observation_id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let trace_id_for_context = trace_id;
        let timestamp = start_time.unwrap_or_else(Utc::now);

        let level = level.map(|l| parse_observation_level(&l));
        let end_time = end_time.map(|t| t.fixed_offset());

        let mut usage_details = usage_details.unwrap_or_default();
        usage_details.extend(
            [
                ("input", prompt_tokens),
                ("output", completion_tokens),
                ("total", total_tokens),
            ]
            .into_iter()
            .filter_map(|(key, count)| count.map(|count| (key.to_owned(), i64::from(count)))),
        );
        let usage_details =
            (!usage_details.is_empty()).then_some(UsageDetails::Object(usage_details));

        let model_parameters = _model_parameters.as_ref().and_then(model_parameters_map);

        let generation_body = CreateGenerationBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id_for_context.clone()))
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
            .maybe_completion_start_time(completion_start_time.map(|t| Some(t.fixed_offset())))
            .maybe_cost_details(cost_details.filter(|c| !c.is_empty()).map(Some))
            .maybe_prompt_name(prompt_name.map(Some))
            .maybe_prompt_version(prompt_version.map(Some))
            .maybe_version(version.map(Some))
            .build();

        let event = IngestionEventCreateGenerationEvent::builder()
            .body(generation_body)
            .id(Uuid::new_v4().to_string())
            .timestamp(timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .r#type(GenerationEventType::GenerationCreate)
            .build();

        let mut events =
            self.trace_attribute_events(&trace_id_for_context, user_id, session_id, tags);
        events.push(IngestionEvent::CreateGenerationEvent(event));
        self.ingest_events(events)
            .await
            .map(|_| self.effective_observation_id(&observation_id))
            .map_err(|e| crate::error::Error::Api(format!("Failed to create generation: {e}")))
    }

    /// Create an event observation
    ///
    /// With [`crate::ApiVersion::V4`] the returned id is the mapped OpenTelemetry span id (see
    /// [`crate::otel`]); passing your own id again resolves to the same observation. A
    /// parentless observation becomes a child of the trace's root span, or the root itself if
    /// this client has not written the trace yet. Writing it once, complete with `end_time`
    /// and output, is the preferred V4 pattern; the export only means Langfuse queued it.
    #[builder]
    pub async fn event(
        &self,
        #[builder(into)] trace_id: String,
        /// Trace user id. V4 stamps it on every span of the trace; V3 upserts the trace (which,
        /// before `trace()`, fixes the trace timestamp: v3 does not change it afterwards).
        #[builder(into)]
        user_id: Option<String>,
        /// Trace session id. V4 stamps it on every span of the trace; V3 upserts the trace.
        #[builder(into)]
        session_id: Option<String>,
        /// Trace tags. V4 stamps them on every span of the trace; V3 upserts the trace.
        tags: Option<Vec<String>>,
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
        let trace_id_for_context = trace_id;
        let timestamp = start_time.unwrap_or_else(Utc::now);

        let level = level.map(|l| parse_observation_level(&l));

        let event_body = CreateEventBody::builder()
            .id(Some(observation_id.clone()))
            .trace_id(Some(trace_id_for_context.clone()))
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

        let mut events =
            self.trace_attribute_events(&trace_id_for_context, user_id, session_id, tags);
        events.push(IngestionEvent::CreateEventEvent(event));
        self.ingest_events(events)
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
    ///
    /// With [`crate::ApiVersion::V4`], Langfuse stores each span as one complete row keyed by
    /// its start time, so this re-sends the full span this client wrote earlier, with the
    /// changes merged in and the original start time (a different `start_time` is ignored).
    /// It fails with [`Error::Validation`] if this client has not written the observation (or
    /// no longer remembers it, see [`crate::otel::STATE_CAPACITY`]). Prefer creating the
    /// observation once with its `end_time` and output. Returns the mapped id.
    ///
    /// Langfuse keeps the row with the latest processing time (`event_ts`, set when its worker
    /// processes the export), and two separate exports of the same span are not ordered: if
    /// the create and the update are sent as two requests (for example `span()` followed by
    /// this method), the older payload can win. Write the span once, complete, or send create
    /// and update through one [`crate::Batcher`] flush, which exports only the merged span.
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
            .map_err(|e| match e {
                Error::Validation(_) => e,
                other => Error::Api(format!("Failed to update span: {other}")),
            })?;

        Ok(self.effective_observation_id(&id))
    }

    /// Update an existing generation
    ///
    /// With [`crate::ApiVersion::V4`], Langfuse stores each span as one complete row keyed by
    /// its start time, so this re-sends the full span this client wrote earlier, with the
    /// changes merged in and the original start time (a different `start_time` is ignored).
    /// It fails with [`Error::Validation`] if this client has not written the observation (or
    /// no longer remembers it, see [`crate::otel::STATE_CAPACITY`]). Prefer creating the
    /// observation once with its `end_time` and output. Returns the mapped id.
    ///
    /// Langfuse keeps the row with the latest processing time (`event_ts`, set when its worker
    /// processes the export), and two separate exports of the same span are not ordered: if
    /// the create and the update are sent as two requests (for example `span()` followed by
    /// this method), the older payload can win. Write the span once, complete, or send create
    /// and update through one [`crate::Batcher`] flush, which exports only the merged span.
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
        /// A JSON object of model parameters.
        model_parameters: Option<Value>,
        /// Token counts by usage type, sent as `usageDetails`.
        usage_details: Option<std::collections::HashMap<String, i64>>,
        /// Costs in USD by usage type, sent as `costDetails`.
        cost_details: Option<std::collections::HashMap<String, f64>>,
        /// The Langfuse prompt this generation used.
        #[builder(into)]
        prompt_name: Option<String>,
        /// The version of `prompt_name`.
        prompt_version: Option<i64>,
    ) -> Result<String> {
        use chrono::Utc as ChronoUtc;
        use langfuse_client_base::models::{
            IngestionEvent, IngestionEventUpdateGenerationEvent, UpdateGenerationBody, UsageDetails,
        };
        use uuid::Uuid;

        let event_body = UpdateGenerationBody {
            id: id.clone(),
            trace_id: Some(Some(trace_id)),
            name: Some(name),
            start_time: Some(start_time.map(|dt| dt.fixed_offset())),
            end_time: Some(end_time.map(|dt| dt.fixed_offset())),
            completion_start_time: Some(completion_start_time.map(|dt| dt.fixed_offset())),
            model: Some(model),
            model_parameters: model_parameters
                .as_ref()
                .and_then(model_parameters_map)
                .map(Some),
            input: Some(input),
            output: Some(output),
            usage: None,
            metadata: Some(metadata),
            level: level.map(|l| parse_observation_level(&l)),
            status_message: Some(status_message),
            version: Some(version),
            parent_observation_id: Some(parent_observation_id),
            environment: None,
            cost_details: cost_details.filter(|c| !c.is_empty()).map(Some),
            prompt_name: prompt_name.map(Some),
            prompt_version: prompt_version.map(Some),
            usage_details: usage_details
                .filter(|u| !u.is_empty())
                .map(UsageDetails::Object),
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
            .map_err(|e| match e {
                Error::Validation(_) => e,
                other => Error::Api(format!("Failed to update generation: {other}")),
            })?;

        Ok(self.effective_observation_id(&id))
    }

    // Note: UpdateEventBody exists in v0.2 but doesn't have a corresponding IngestionEvent variant
    // This functionality will need to wait for a later version
}

/// Every field group of `GET /api/public/v2/observations`, for the `fields` parameter.
pub const ALL_OBSERVATION_FIELDS: &str =
    "core,basic,time,io,metadata,model,usage,prompt,metrics,trace_context";

/// Largest `limit` the observations API accepts; [`LangfuseClient::trace_observations`]
/// requests pages of this size.
pub const MAX_OBSERVATIONS_PAGE_SIZE: u32 = 1000;

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
    /// `limit` must be at most [`MAX_OBSERVATIONS_PAGE_SIZE`] (checked before the request).
    /// The API does not de-duplicate rows. Usage counts that are not integers are rounded,
    /// because [`ObservationV2::usage_details`](langfuse_client_base::models::ObservationV2)
    /// holds integers.
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
        /// Maximum number of items per page (1 to [`MAX_OBSERVATIONS_PAGE_SIZE`]).
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
        /// Structured JSON filter; takes precedence over the individual filters.
        #[builder(into)]
        filter: Option<String>,
    ) -> Result<ObservationsPage> {
        if let Some(limit) = limit {
            if limit == 0 || limit > MAX_OBSERVATIONS_PAGE_SIZE {
                return Err(Error::Validation(format!(
                    "observations limit must be between 1 and {MAX_OBSERVATIONS_PAGE_SIZE}, got {limit}"
                )));
            }
        }
        let trace_id = trace_id.map(|id| self.effective_trace_id(&id));
        let parent_observation_id =
            parent_observation_id.map(|id| self.effective_observation_id(&id));

        let mut query: Vec<(&str, String)> = Vec::new();
        let mut push = |key: &'static str, value: Option<String>| {
            if let Some(value) = value {
                query.push((key, value));
            }
        };
        push("fields", fields);
        push("expandMetadata", expand_metadata);
        push("limit", limit.map(|l| l.to_string()));
        push("cursor", cursor);
        push("name", name);
        push("userId", user_id);
        push("sessionId", session_id);
        push("type", observation_type);
        push("traceId", trace_id);
        // `Unknown` is the base crate's catch-all; the server rejects `level=unknown`.
        push(
            "level",
            level
                .filter(|l| *l != ObservationLevel::Unknown)
                .map(|l| l.to_string()),
        );
        push("parentObservationId", parent_observation_id);
        push(
            "isRootObservation",
            is_root_observation.map(|b| b.to_string()),
        );
        for environment in environment.into_iter().flatten() {
            push("environment", Some(environment));
        }
        push(
            "fromStartTime",
            from_start_time.map(|t| t.fixed_offset().to_rfc3339()),
        );
        push(
            "toStartTime",
            to_start_time.map(|t| t.fixed_offset().to_rfc3339()),
        );
        push("version", version);
        push("filter", filter);

        // A raw request instead of the generated client, so that non-integer usage counts
        // (which the server allows) can be rounded instead of failing deserialization.
        let mut request = self
            .configuration
            .client
            .request(
                timvw_rust_runtime::Method::GET,
                &format!(
                    "{}/api/public/v2/observations",
                    self.configuration.base_path
                ),
            )
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .query(&query);
        if let Some(user_agent) = &self.configuration.user_agent {
            request = request.header(timvw_rust_runtime::header::USER_AGENT, user_agent);
        }
        let response = request.send().await?;
        if !response.status().is_success() {
            return Err(crate::error::error_from_response(response).await);
        }
        let text = response.text().await?;
        let mut body: Value = serde_json::from_str(&text)?;
        round_usage_details(&mut body);
        let response: langfuse_client_base::models::ObservationsV2Response =
            serde_json::from_value(body)?;
        Ok(ObservationsPage::from(response))
    }

    /// Fetch every observation of a trace, following the cursor across pages.
    ///
    /// Requests all field groups ([`ALL_OBSERVATION_FIELDS`]) in pages of
    /// [`MAX_OBSERVATIONS_PAGE_SIZE`]. The trace's root observation carries the trace-level
    /// input and output. The API does not de-duplicate rows, so as a safety net this keeps one
    /// observation per id (the one with the latest `updated_at`), in first-seen order. This
    /// replaces [`LangfuseClient::get_trace`] for Langfuse v4.
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
                .limit(MAX_OBSERVATIONS_PAGE_SIZE)
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
                None => return Ok(dedupe_observations(observations)),
            }
        }
    }
}

/// Round non-integer usage counts in an observations response, since `usage_details` is a
/// map of integers.
fn round_usage_details(body: &mut Value) {
    let Some(data) = body.get_mut("data").and_then(Value::as_array_mut) else {
        return;
    };
    for observation in data {
        if let Some(Value::Object(usage)) = observation.get_mut("usageDetails") {
            for value in usage.values_mut() {
                if let Value::Number(n) = value {
                    if !n.is_i64() {
                        if let Some(f) = n.as_f64() {
                            #[allow(clippy::cast_possible_truncation)]
                            let rounded = f.round() as i64;
                            *value = Value::from(rounded);
                        }
                    }
                }
            }
        }
    }
}

/// Keep one observation per id (the latest `updated_at`), in first-seen order.
fn dedupe_observations(
    observations: Vec<langfuse_client_base::models::ObservationV2>,
) -> Vec<langfuse_client_base::models::ObservationV2> {
    let mut order: Vec<String> = Vec::new();
    let mut latest: std::collections::HashMap<String, langfuse_client_base::models::ObservationV2> =
        std::collections::HashMap::new();
    for observation in observations {
        match latest.get(&observation.id) {
            None => {
                order.push(observation.id.clone());
                latest.insert(observation.id.clone(), observation);
            }
            Some(existing) => {
                let updated =
                    |o: &langfuse_client_base::models::ObservationV2| o.updated_at.flatten();
                if updated(&observation) >= updated(existing) {
                    latest.insert(observation.id.clone(), observation);
                }
            }
        }
    }
    order
        .into_iter()
        .filter_map(|id| latest.remove(&id))
        .collect()
}
