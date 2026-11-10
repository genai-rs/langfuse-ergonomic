//! Langfuse v4 write path: OpenTelemetry (OTLP/HTTP JSON) encoding and id mapping.
//!
//! With [`ApiVersion::V4`](crate::ApiVersion::V4) the client keeps building the same
//! ingestion events as before, and this module translates them:
//!
//! - trace, span, generation and event (create and update) events become OTLP spans sent to
//!   `POST /api/public/otel/v1/traces` with the `x-langfuse-ingestion-version: 4` header;
//! - score events become requests to `POST /api/public/scores`.
//!
//! # Encoding
//!
//! Spans are sent as OTLP/HTTP **JSON** (`content-type: application/json`), which Langfuse
//! accepts next to protobuf. JSON only needs `serde_json`, which this crate already uses, so
//! the v4 path adds no dependencies. Langfuse's own Rust AI gateway exports to the same endpoint
//! with JSON.
//!
//! # Id mapping
//!
//! Langfuse v3 accepted arbitrary string ids. OpenTelemetry needs a 16-byte trace id and an
//! 8-byte span id, written as lowercase hex. Langfuse v4 stores the OTel ids as the trace and
//! observation ids, so every id passes through a deterministic mapping:
//!
//! - [`trace_id`]: a 32-character hex string or any UUID (hyphenated or not) is used as is, in
//!   lowercase simple form. Any other string is hashed with UUID v5 (SHA-1) in a fixed namespace.
//! - [`observation_id`]: a 16-character hex string is used as is, in lowercase. Any other string
//!   (including UUIDs) is hashed with UUID v5 and the first 8 bytes are kept.
//! - [`trace_root_span_id`]: the span that represents a trace (see below) gets an id derived
//!   from the mapped trace id.
//!
//! The mapping is idempotent: an id that is already a valid OTel id maps to itself, so ids
//! returned by the client can be passed back in. The all-zero id, which OTel treats as invalid,
//! is hashed like any other string. Because the mapping is deterministic, parent links, score
//! targets and repeated calls with the same caller-chosen id all resolve to the same OTel id.
//! The builder methods return the mapped id in V4 mode.
//!
//! Data written under Langfuse v3 with ids that are not OTel ids (for example `"my-trace"`)
//! is stored under those raw ids. A V4 client maps them, so it cannot address that data
//! (scores, `delete_trace`, the `trace_id` read filter); use a V3 client for it.
//!
//! # Traces, roots and trace attributes
//!
//! OpenTelemetry has no trace entity of its own. A trace event becomes a zero-duration root
//! span with id [`trace_root_span_id`] that carries the `langfuse.trace.*` attributes and the
//! trace input and output (Langfuse v4 reads trace input and output from the root observation;
//! `langfuse.trace.input`/`output` are sent too, for servers that still write v3 traces).
//!
//! The client keeps a bounded least-recently-used write state ([`STATE_CAPACITY`] entries per
//! map and [`STATE_MAX_BYTES`] of cached spans by default, configurable on
//! [`crate::ClientBuilder`]), shared by its clones and batchers:
//!
//! - Observations without a `parent_observation_id` become children of the trace's root span.
//!   If this client has not written the trace, the first such observation has no parent and
//!   becomes the root; a later `trace()` adds the trace attributes to it. Unlike Langfuse v3,
//!   where parentless observations were siblings, a second parentless observation therefore
//!   becomes a child of the first.
//! - The state is per client. When several processes or clients write to one trace, or the
//!   root was evicted, each may create its own root. To attach to the root written by
//!   `trace()` elsewhere, pass `parent_observation_id(otel::trace_root_span_id(trace_id))`.
//! - The trace's user id, session id, tags, release and version (from `trace()`, or from the
//!   `user_id`/`session_id`/`tags` setters of the observation builders) are stamped on every
//!   observation span written after them. Langfuse needs them on every span to filter
//!   observations by them.
//!
//! # Updates
//!
//! Langfuse v4 stores each span as one complete row keyed by its start time and span id and
//! does not merge rows field by field. A partial span would become a second row. So an update
//! (`update_span()`, `update_generation()`, update events, or `trace()` again with the same id)
//! re-sends the full span from the write state with the changes merged in and the **original
//! start time**. Updating an observation the state does not hold fails with
//! [`Error::Validation`]. The preferred V4 pattern is to write each observation once, complete
//! with its end time and output.
//!
//! # Delivery
//!
//! A successful OTLP export means Langfuse accepted and queued the spans; processing is
//! asynchronous, and OTLP reports no per-span result.
//!
//! # Attribute mapping
//!
//! See the README section "Langfuse v3 and v4" for the full table. Attribute names follow
//! <https://langfuse.com/integrations/native/opentelemetry>.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, FixedOffset, Utc};
use langfuse_client_base::models::{
    CreateScoreRequest, IngestionEvent, IngestionUsage, MapValue, ObservationLevel,
    ObservationType, ScoreBody, UsageDetails,
};
use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

/// Path of the Langfuse OTLP/HTTP traces endpoint.
pub const OTLP_TRACES_PATH: &str = "/api/public/otel/v1/traces";

/// Header that selects the Langfuse v4 real-time ingestion path.
pub const INGESTION_VERSION_HEADER: &str = "x-langfuse-ingestion-version";

/// Instrumentation scope name used for the exported spans.
pub const SCOPE_NAME: &str = env!("CARGO_PKG_NAME");

/// Fixed namespace for hashing non-OTel ids. Changing it would change every mapped id.
const ID_NAMESPACE: Uuid = Uuid::from_bytes([
    0x5b, 0x0e, 0x3c, 0x1f, 0x8a, 0x52, 0x5d, 0x4e, 0x9a, 0x61, 0x2c, 0x7d, 0x40, 0x13, 0xe8, 0x9b,
]);

fn hashed(value: &str) -> Uuid {
    Uuid::new_v5(&ID_NAMESPACE, value.as_bytes())
}

fn is_valid_hex_id(value: &str, len: usize) -> bool {
    value.len() == len
        && value.bytes().all(|b| b.is_ascii_hexdigit())
        && value.bytes().any(|b| b != b'0')
}

/// Map a Langfuse trace id to a 32-character lowercase hex OpenTelemetry trace id.
///
/// See the [module documentation](self) for the rules.
pub fn trace_id(id: &str) -> String {
    if is_valid_hex_id(id, 32) {
        return id.to_ascii_lowercase();
    }
    match Uuid::parse_str(id) {
        Ok(uuid) if !uuid.is_nil() => uuid.simple().to_string(),
        _ => hashed(id).simple().to_string(),
    }
}

/// Map a Langfuse observation id to a 16-character lowercase hex OpenTelemetry span id.
///
/// See the [module documentation](self) for the rules.
pub fn observation_id(id: &str) -> String {
    if is_valid_hex_id(id, 16) {
        return id.to_ascii_lowercase();
    }
    let mut hex = hashed(id).simple().to_string();
    hex.truncate(16);
    if hex.bytes().all(|b| b == b'0') {
        // Practically unreachable, but an all-zero span id is invalid in OTel.
        hex.replace_range(15.., "1");
    }
    hex
}

/// Span id of the root span that represents the trace `trace_id` in V4 mode.
///
/// `trace_id` may be any Langfuse trace id; it is mapped with [`trace_id`] first.
pub fn trace_root_span_id(trace_id_value: &str) -> String {
    observation_id(&format!("langfuse-trace-root:{}", trace_id(trace_id_value)))
}

/// One ingestion event translated for the v4 API.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum V4Item {
    /// An OTLP span (JSON encoding).
    Span(Value),
    /// A score for `POST /api/public/scores`.
    Score(Box<CreateScoreRequest>),
}

fn flat<T>(value: &Option<Option<T>>) -> Option<&T> {
    value.as_ref().and_then(Option::as_ref)
}

fn flat_str(value: &Option<Option<String>>) -> Option<&str> {
    flat(value).map(String::as_str)
}

/// Builder for an OTLP JSON attribute list.
#[derive(Default)]
struct Attributes(Vec<Value>);

impl Attributes {
    fn push(&mut self, key: impl Into<String>, value: Value) {
        self.0.push(json!({ "key": key.into(), "value": value }));
    }

    fn string(&mut self, key: &str, value: Option<&str>) {
        if let Some(value) = value {
            self.push(key, json!({ "stringValue": value }));
        }
    }

    fn bool(&mut self, key: &str, value: Option<bool>) {
        if let Some(value) = value {
            self.push(key, json!({ "boolValue": value }));
        }
    }

    fn int(&mut self, key: &str, value: Option<i64>) {
        if let Some(value) = value {
            // OTLP JSON encodes 64-bit integers as decimal strings.
            self.push(key, json!({ "intValue": value.to_string() }));
        }
    }

    fn string_array(&mut self, key: &str, values: Option<&[String]>) {
        if let Some(values) = values {
            let values: Vec<Value> = values.iter().map(|v| json!({ "stringValue": v })).collect();
            self.push(key, json!({ "arrayValue": { "values": values } }));
        }
    }

    /// A JSON value written as a string: strings are written unchanged, everything else is
    /// serialized to JSON.
    fn json(&mut self, key: &str, value: Option<&Value>) {
        if let Some(value) = value {
            if !value.is_null() {
                self.push(key, json!({ "stringValue": json_string(value) }));
            }
        }
    }

    fn time(&mut self, key: &str, value: Option<&DateTime<FixedOffset>>) {
        if let Some(value) = value {
            let formatted = value
                .with_timezone(&Utc)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            self.push(key, json!({ "stringValue": formatted }));
        }
    }

    /// Write metadata twice: the whole object as a JSON string under `prefix`, which keeps
    /// value types and nesting, and every string value as `<prefix>.<key>`, the form Langfuse
    /// filters on. A non-object value is wrapped as `{"value": ...}`. Keys that collide with
    /// the metadata fields the Langfuse server adds itself (`attributes`, `resourceAttributes`,
    /// `scope`) are renamed to `user_<key>`.
    fn metadata(&mut self, prefix: &str, metadata: Option<&Value>) {
        let object = match metadata {
            None | Some(Value::Null) => return,
            Some(Value::Object(map)) => map.clone(),
            Some(other) => Map::from_iter([("value".to_owned(), other.clone())]),
        };
        let object: Map<String, Value> = object
            .into_iter()
            .filter(|(_, v)| !v.is_null())
            .map(|(k, v)| (guard_metadata_key(k), v))
            .collect();
        if object.is_empty() {
            return;
        }
        for (key, value) in &object {
            if let Value::String(s) = value {
                self.push(format!("{prefix}.{key}"), json!({ "stringValue": s }));
            }
        }
        self.push(
            prefix,
            json!({ "stringValue": Value::Object(object).to_string() }),
        );
    }
}

/// Metadata keys the Langfuse server sets itself on OTel-ingested observations.
const RESERVED_METADATA_KEYS: [&str; 3] = ["attributes", "resourceAttributes", "scope"];

fn guard_metadata_key(key: String) -> String {
    if RESERVED_METADATA_KEYS.contains(&key.as_str()) {
        format!("user_{key}")
    } else {
        key
    }
}

fn json_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn level_str(level: Option<&ObservationLevel>) -> Option<&'static str> {
    match level? {
        ObservationLevel::Debug => Some("DEBUG"),
        ObservationLevel::Default => Some("DEFAULT"),
        ObservationLevel::Warning => Some("WARNING"),
        ObservationLevel::Error => Some("ERROR"),
        ObservationLevel::Unknown => None,
    }
}

fn observation_type_str(kind: ObservationType) -> &'static str {
    match kind {
        ObservationType::Span => "span",
        ObservationType::Generation => "generation",
        ObservationType::Event => "event",
        ObservationType::Agent => "agent",
        ObservationType::Tool => "tool",
        ObservationType::Chain => "chain",
        ObservationType::Retriever => "retriever",
        ObservationType::Evaluator => "evaluator",
        ObservationType::Embedding => "embedding",
        ObservationType::Guardrail => "guardrail",
    }
}

fn nanos(time: &DateTime<FixedOffset>) -> Option<String> {
    time.timestamp_nanos_opt().map(|n| n.max(0).to_string())
}

fn parse_envelope_time(timestamp: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(timestamp).ok()
}

/// The fields shared by all observation bodies, borrowed from the ingestion event.
#[derive(Default)]
struct Observation<'a> {
    kind: &'static str,
    is_update: bool,
    id: Option<&'a str>,
    trace_id: Option<&'a str>,
    parent_observation_id: Option<&'a str>,
    name: Option<&'a str>,
    start_time: Option<DateTime<FixedOffset>>,
    end_time: Option<DateTime<FixedOffset>>,
    completion_start_time: Option<&'a DateTime<FixedOffset>>,
    input: Option<&'a Value>,
    output: Option<&'a Value>,
    metadata: Option<&'a Value>,
    level: Option<&'a ObservationLevel>,
    status_message: Option<&'a str>,
    version: Option<&'a str>,
    environment: Option<&'a str>,
    model: Option<&'a str>,
    model_parameters: Option<&'a HashMap<String, MapValue>>,
    usage_details: Option<Value>,
    cost_details: Option<Value>,
    prompt_name: Option<&'a str>,
    prompt_version: Option<i64>,
}

fn usage_details_json(
    details: Option<&UsageDetails>,
    legacy: Option<&IngestionUsage>,
) -> (Option<Value>, Option<Value>) {
    let mut usage = Map::new();
    let mut cost = Map::new();
    match details {
        Some(UsageDetails::Object(map)) => {
            for (key, value) in map {
                usage.insert(key.clone(), json!(value));
            }
        }
        Some(UsageDetails::OpenAiCompletionUsageSchema(schema)) => {
            usage.insert("input".into(), json!(schema.prompt_tokens));
            usage.insert("output".into(), json!(schema.completion_tokens));
            usage.insert("total".into(), json!(schema.total_tokens));
            for (prefix, details) in [
                ("input", flat(&schema.prompt_tokens_details)),
                ("output", flat(&schema.completion_tokens_details)),
            ] {
                for (key, value) in details.into_iter().flatten() {
                    // A `null` count is not a usage figure: send only real numbers.
                    if let Some(value) = value {
                        usage.insert(format!("{prefix}_{key}"), json!(value));
                    }
                }
            }
        }
        Some(UsageDetails::OpenAiResponseUsageSchema(schema)) => {
            usage.insert("input".into(), json!(schema.input_tokens));
            usage.insert("output".into(), json!(schema.output_tokens));
            usage.insert("total".into(), json!(schema.total_tokens));
            for (prefix, details) in [
                ("input", flat(&schema.input_tokens_details)),
                ("output", flat(&schema.output_tokens_details)),
            ] {
                for (key, value) in details.into_iter().flatten() {
                    // A `null` count is not a usage figure: send only real numbers.
                    if let Some(value) = value {
                        usage.insert(format!("{prefix}_{key}"), json!(value));
                    }
                }
            }
        }
        None => match legacy {
            Some(IngestionUsage::Usage(u)) => {
                usage.insert("input".into(), json!(u.input));
                usage.insert("output".into(), json!(u.output));
                usage.insert("total".into(), json!(u.total));
                for (key, value) in [
                    ("input", flat(&u.input_cost)),
                    ("output", flat(&u.output_cost)),
                    ("total", flat(&u.total_cost)),
                ] {
                    if let Some(value) = value {
                        cost.insert(key.into(), json!(value));
                    }
                }
            }
            Some(IngestionUsage::OpenAiUsage(u)) => {
                for (key, value) in [
                    ("input", flat(&u.prompt_tokens)),
                    ("output", flat(&u.completion_tokens)),
                    ("total", flat(&u.total_tokens)),
                ] {
                    if let Some(value) = value {
                        usage.insert(key.into(), json!(value));
                    }
                }
            }
            None => {}
        },
    }
    (
        (!usage.is_empty()).then_some(Value::Object(usage)),
        (!cost.is_empty()).then_some(Value::Object(cost)),
    )
}

fn cost_details_json(cost: Option<&HashMap<String, f64>>) -> Option<Value> {
    cost.filter(|c| !c.is_empty())
        .map(|c| Value::Object(c.iter().map(|(k, v)| (k.clone(), json!(v))).collect()))
}

fn missing_trace_id(kind: &str) -> Error {
    Error::Validation(format!(
        "{kind} event has no trace id; Langfuse v4 (OpenTelemetry) needs one for every observation"
    ))
}

fn span_json(
    trace_hex: String,
    span_hex: String,
    parent_hex: Option<String>,
    name: &str,
    start: Option<&DateTime<FixedOffset>>,
    end: Option<&DateTime<FixedOffset>>,
    attributes: Attributes,
    error_status: Option<&str>,
) -> Value {
    let mut span = Map::new();
    span.insert("traceId".into(), Value::String(trace_hex));
    span.insert("spanId".into(), Value::String(span_hex));
    if let Some(parent) = parent_hex {
        span.insert("parentSpanId".into(), Value::String(parent));
    }
    span.insert("name".into(), Value::String(name.to_owned()));
    // SPAN_KIND_INTERNAL
    span.insert("kind".into(), json!(1));
    if let Some(start) = start.and_then(nanos) {
        span.insert("startTimeUnixNano".into(), Value::String(start));
    }
    if let Some(end) = end.and_then(nanos) {
        span.insert("endTimeUnixNano".into(), Value::String(end));
    }
    span.insert("attributes".into(), Value::Array(attributes.0));
    if let Some(message) = error_status {
        // STATUS_CODE_ERROR
        span.insert("status".into(), json!({ "code": 2, "message": message }));
    }
    Value::Object(span)
}

fn encode_observation(
    obs: Observation<'_>,
    envelope_time: Option<DateTime<FixedOffset>>,
) -> Result<Value> {
    let trace_raw = obs.trace_id.ok_or_else(|| missing_trace_id(obs.kind))?;
    let id_raw = obs
        .id
        .ok_or_else(|| Error::Validation(format!("{} event has no observation id", obs.kind)))?;
    let trace_hex = trace_id(trace_raw);
    // Only an explicit parent is encoded here; `V4State` links parentless observations to
    // the trace root.
    let parent_hex = obs.parent_observation_id.map(observation_id);

    // Creates fall back to the event timestamp; updates only send the times they change.
    let start = obs
        .start_time
        .or(if obs.is_update { None } else { envelope_time });
    let end = if obs.kind == "event" {
        obs.end_time.or(start)
    } else {
        obs.end_time
    };

    let mut attrs = Attributes::default();
    attrs.string("langfuse.observation.type", Some(obs.kind));
    attrs.json("langfuse.observation.input", obs.input);
    attrs.json("langfuse.observation.output", obs.output);
    attrs.metadata("langfuse.observation.metadata", obs.metadata);
    let level = level_str(obs.level);
    attrs.string("langfuse.observation.level", level);
    attrs.string("langfuse.observation.status_message", obs.status_message);
    attrs.string("langfuse.version", obs.version);
    attrs.string("langfuse.environment", obs.environment);
    attrs.string("langfuse.observation.model.name", obs.model);
    if let Some(params) = obs.model_parameters.filter(|p| !p.is_empty()) {
        let params = serde_json::to_string(params)?;
        attrs.string("langfuse.observation.model.parameters", Some(&params));
    }
    attrs.json(
        "langfuse.observation.usage_details",
        obs.usage_details.as_ref(),
    );
    attrs.json(
        "langfuse.observation.cost_details",
        obs.cost_details.as_ref(),
    );
    attrs.time(
        "langfuse.observation.completion_start_time",
        obs.completion_start_time,
    );
    attrs.string("langfuse.observation.prompt.name", obs.prompt_name);
    attrs.int("langfuse.observation.prompt.version", obs.prompt_version);

    let error_status = (level == Some("ERROR")).then(|| obs.status_message.unwrap_or(""));
    Ok(span_json(
        trace_hex,
        observation_id(id_raw),
        parent_hex,
        obs.name.unwrap_or(""),
        start.as_ref(),
        end.as_ref(),
        attrs,
        error_status,
    ))
}

fn encode_trace(
    body: &langfuse_client_base::models::TraceBody,
    envelope_time: Option<DateTime<FixedOffset>>,
) -> Result<Value> {
    let trace_raw = flat_str(&body.id).ok_or_else(|| missing_trace_id("trace"))?;
    let time = flat(&body.timestamp).copied().or(envelope_time);
    let name = flat_str(&body.name);

    let mut attrs = Attributes::default();
    attrs.string("langfuse.observation.type", Some("span"));
    attrs.string("langfuse.trace.name", name);
    attrs.string("user.id", flat_str(&body.user_id));
    attrs.string("session.id", flat_str(&body.session_id));
    attrs.string_array("langfuse.trace.tags", flat(&body.tags).map(Vec::as_slice));
    attrs.bool("langfuse.trace.public", flat(&body.public).copied());
    attrs.metadata("langfuse.trace.metadata", flat(&body.metadata));
    // Langfuse v4 reads trace input/output from the root observation; servers that still
    // dual-write v3 traces read `langfuse.trace.input`/`output`.
    attrs.json("langfuse.observation.input", flat(&body.input));
    attrs.json("langfuse.observation.output", flat(&body.output));
    attrs.json("langfuse.trace.input", flat(&body.input));
    attrs.json("langfuse.trace.output", flat(&body.output));
    attrs.string("langfuse.release", flat_str(&body.release));
    attrs.string("langfuse.version", flat_str(&body.version));
    attrs.string("langfuse.environment", flat_str(&body.environment));

    Ok(span_json(
        trace_id(trace_raw),
        trace_root_span_id(trace_raw),
        None,
        name.unwrap_or(""),
        time.as_ref(),
        time.as_ref(),
        attrs,
        None,
    ))
}

fn encode_score(
    body: &ScoreBody,
    envelope_time: Option<DateTime<FixedOffset>>,
) -> CreateScoreRequest {
    let metadata = flat(&body.metadata).and_then(|m| match m {
        Value::Null => None,
        Value::Object(map) => Some(map.clone().into_iter().collect::<HashMap<_, _>>()),
        other => Some(HashMap::from([("value".to_owned(), other.clone())])),
    });
    CreateScoreRequest {
        id: body.id.clone(),
        trace_id: flat_str(&body.trace_id).map(|t| Some(trace_id(t))),
        session_id: body.session_id.clone(),
        observation_id: flat_str(&body.observation_id).map(|o| Some(observation_id(o))),
        dataset_run_id: body.dataset_run_id.clone(),
        name: body.name.clone(),
        value: body.value.clone(),
        comment: body.comment.clone(),
        metadata: metadata.map(Some),
        environment: body.environment.clone(),
        queue_id: body.queue_id.clone(),
        data_type: body.data_type,
        config_id: body.config_id.clone(),
        source: None,
        timestamp: envelope_time.map(Some),
    }
}

fn encode_observation_body(
    b: &langfuse_client_base::models::ObservationBody,
    is_update: bool,
    envelope_time: Option<DateTime<FixedOffset>>,
) -> Result<Value> {
    let legacy_usage = b.usage.clone().map(IngestionUsage::Usage);
    let (usage, legacy_cost) = usage_details_json(None, legacy_usage.as_ref());
    encode_observation(
        Observation {
            kind: observation_type_str(b.r#type),
            is_update,
            id: flat_str(&b.id),
            trace_id: flat_str(&b.trace_id),
            parent_observation_id: flat_str(&b.parent_observation_id),
            name: flat_str(&b.name),
            start_time: flat(&b.start_time).copied(),
            end_time: flat(&b.end_time).copied(),
            completion_start_time: flat(&b.completion_start_time),
            input: flat(&b.input),
            output: flat(&b.output),
            metadata: flat(&b.metadata),
            level: b.level.as_ref(),
            status_message: flat_str(&b.status_message),
            version: flat_str(&b.version),
            environment: flat_str(&b.environment),
            model: flat_str(&b.model),
            model_parameters: flat(&b.model_parameters),
            usage_details: usage,
            cost_details: legacy_cost,
            ..Observation::default()
        },
        envelope_time,
    )
}

/// Translate one ingestion event for the v4 API without state (no parent linking, no trace
/// attribute propagation, updates stay partial). [`V4State::apply`] builds on it.
pub(crate) fn encode_event(event: &IngestionEvent) -> Result<V4Item> {
    use IngestionEvent as E;

    let envelope = |timestamp: &str| parse_envelope_time(timestamp);
    let span = match event {
        E::TraceEvent(e) => encode_trace(&e.body, envelope(&e.timestamp))?,
        E::ScoreEvent(e) => {
            return Ok(V4Item::Score(Box::new(encode_score(
                &e.body,
                envelope(&e.timestamp),
            ))))
        }
        E::CreateSpanEvent(e) => {
            let b = &e.body;
            encode_observation(
                Observation {
                    kind: "span",
                    id: flat_str(&b.id),
                    trace_id: flat_str(&b.trace_id),
                    parent_observation_id: flat_str(&b.parent_observation_id),
                    name: flat_str(&b.name),
                    start_time: flat(&b.start_time).copied(),
                    end_time: flat(&b.end_time).copied(),
                    input: flat(&b.input),
                    output: flat(&b.output),
                    metadata: flat(&b.metadata),
                    level: b.level.as_ref(),
                    status_message: flat_str(&b.status_message),
                    version: flat_str(&b.version),
                    environment: flat_str(&b.environment),
                    ..Observation::default()
                },
                envelope(&e.timestamp),
            )?
        }
        E::UpdateSpanEvent(e) => {
            let b = &e.body;
            encode_observation(
                Observation {
                    kind: "span",
                    is_update: true,
                    id: Some(b.id.as_str()),
                    trace_id: flat_str(&b.trace_id),
                    parent_observation_id: flat_str(&b.parent_observation_id),
                    name: flat_str(&b.name),
                    start_time: flat(&b.start_time).copied(),
                    end_time: flat(&b.end_time).copied(),
                    input: flat(&b.input),
                    output: flat(&b.output),
                    metadata: flat(&b.metadata),
                    level: b.level.as_ref(),
                    status_message: flat_str(&b.status_message),
                    version: flat_str(&b.version),
                    environment: flat_str(&b.environment),
                    ..Observation::default()
                },
                envelope(&e.timestamp),
            )?
        }
        E::CreateGenerationEvent(e) => {
            let b = &e.body;
            let (usage, legacy_cost) =
                usage_details_json(b.usage_details.as_ref(), b.usage.as_ref());
            encode_observation(
                Observation {
                    kind: "generation",
                    id: flat_str(&b.id),
                    trace_id: flat_str(&b.trace_id),
                    parent_observation_id: flat_str(&b.parent_observation_id),
                    name: flat_str(&b.name),
                    start_time: flat(&b.start_time).copied(),
                    end_time: flat(&b.end_time).copied(),
                    completion_start_time: flat(&b.completion_start_time),
                    input: flat(&b.input),
                    output: flat(&b.output),
                    metadata: flat(&b.metadata),
                    level: b.level.as_ref(),
                    status_message: flat_str(&b.status_message),
                    version: flat_str(&b.version),
                    environment: flat_str(&b.environment),
                    model: flat_str(&b.model),
                    model_parameters: flat(&b.model_parameters),
                    usage_details: usage,
                    cost_details: cost_details_json(flat(&b.cost_details)).or(legacy_cost),
                    prompt_name: flat_str(&b.prompt_name),
                    prompt_version: flat(&b.prompt_version).copied(),
                    ..Observation::default()
                },
                envelope(&e.timestamp),
            )?
        }
        E::UpdateGenerationEvent(e) => {
            let b = &e.body;
            let (usage, legacy_cost) =
                usage_details_json(b.usage_details.as_ref(), b.usage.as_ref());
            encode_observation(
                Observation {
                    kind: "generation",
                    is_update: true,
                    id: Some(b.id.as_str()),
                    trace_id: flat_str(&b.trace_id),
                    parent_observation_id: flat_str(&b.parent_observation_id),
                    name: flat_str(&b.name),
                    start_time: flat(&b.start_time).copied(),
                    end_time: flat(&b.end_time).copied(),
                    completion_start_time: flat(&b.completion_start_time),
                    input: flat(&b.input),
                    output: flat(&b.output),
                    metadata: flat(&b.metadata),
                    level: b.level.as_ref(),
                    status_message: flat_str(&b.status_message),
                    version: flat_str(&b.version),
                    environment: flat_str(&b.environment),
                    model: flat_str(&b.model),
                    model_parameters: flat(&b.model_parameters),
                    usage_details: usage,
                    cost_details: cost_details_json(flat(&b.cost_details)).or(legacy_cost),
                    prompt_name: flat_str(&b.prompt_name),
                    prompt_version: flat(&b.prompt_version).copied(),
                },
                envelope(&e.timestamp),
            )?
        }
        E::CreateEventEvent(e) => {
            let b = &e.body;
            encode_observation(
                Observation {
                    kind: "event",
                    id: flat_str(&b.id),
                    trace_id: flat_str(&b.trace_id),
                    parent_observation_id: flat_str(&b.parent_observation_id),
                    name: flat_str(&b.name),
                    start_time: flat(&b.start_time).copied(),
                    input: flat(&b.input),
                    output: flat(&b.output),
                    metadata: flat(&b.metadata),
                    level: b.level.as_ref(),
                    status_message: flat_str(&b.status_message),
                    version: flat_str(&b.version),
                    environment: flat_str(&b.environment),
                    ..Observation::default()
                },
                envelope(&e.timestamp),
            )?
        }
        E::CreateObservationEvent(e) => {
            encode_observation_body(&e.body, false, envelope(&e.timestamp))?
        }
        E::UpdateObservationEvent(e) => {
            encode_observation_body(&e.body, true, envelope(&e.timestamp))?
        }
    };
    Ok(V4Item::Span(span))
}

/// Default number of spans, traces and batch events the V4 write state remembers. Set it
/// with [`crate::ClientBuilder::v4_state_capacity`].
pub const STATE_CAPACITY: usize = 10_000;

/// Default budget, in bytes of serialized JSON, for the full spans the V4 write state keeps.
/// Set it with [`crate::ClientBuilder::v4_state_max_bytes`].
pub const STATE_MAX_BYTES: usize = 64 * 1024 * 1024;

/// How many of the least recently used entries eviction inspects to find a preferred victim.
const EVICTION_SCAN: usize = 64;

#[derive(Debug)]
struct LruEntry<V> {
    value: V,
    tick: u64,
    weight: usize,
}

/// Least-recently-used map bounded by entry count and total weight. Reads and writes refresh
/// an entry. Eviction removes the least recently used entry, but first looks among the
/// [`EVICTION_SCAN`] least recently used ones for an entry `prefer_evict` accepts.
#[derive(Debug)]
struct Lru<V> {
    map: HashMap<String, LruEntry<V>>,
    order: BTreeMap<u64, String>,
    next_tick: u64,
    capacity: usize,
    max_weight: usize,
    total_weight: usize,
    prefer_evict: fn(&V) -> bool,
}

impl<V> Lru<V> {
    fn new(capacity: usize, max_weight: usize, prefer_evict: fn(&V) -> bool) -> Self {
        Self {
            map: HashMap::new(),
            order: BTreeMap::new(),
            next_tick: 0,
            capacity: capacity.max(1),
            max_weight,
            total_weight: 0,
            prefer_evict,
        }
    }

    fn touch(&mut self, key: &str) {
        if let Some(entry) = self.map.get_mut(key) {
            self.order.remove(&entry.tick);
            entry.tick = self.next_tick;
            self.order.insert(self.next_tick, key.to_owned());
            self.next_tick += 1;
        }
    }

    /// Whether `key` is present, without marking it as used.
    fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Look up `key` and mark it as recently used.
    fn get(&mut self, key: &str) -> Option<&V> {
        self.touch(key);
        self.map.get(key).map(|entry| &entry.value)
    }

    fn insert(&mut self, key: String, value: V, weight: usize) {
        if let Some(old) = self.map.remove(&key) {
            self.order.remove(&old.tick);
            self.total_weight -= old.weight;
        }
        let tick = self.next_tick;
        self.next_tick += 1;
        self.order.insert(tick, key.clone());
        self.total_weight += weight;
        self.map.insert(
            key.clone(),
            LruEntry {
                value,
                tick,
                weight,
            },
        );
        while self.map.len() > self.capacity
            || (self.total_weight > self.max_weight && self.map.len() > 1)
        {
            let victim = self
                .order
                .values()
                .take(EVICTION_SCAN)
                .filter(|k| **k != key)
                .find(|k| {
                    self.map
                        .get(*k)
                        .is_some_and(|e| (self.prefer_evict)(&e.value))
                })
                .or_else(|| self.order.values().find(|k| **k != key))
                .cloned();
            let Some(victim) = victim else { break };
            if let Some(entry) = self.map.remove(&victim) {
                self.order.remove(&entry.tick);
                self.total_weight -= entry.weight;
            }
        }
    }
}

fn always<V>(_: &V) -> bool {
    true
}

/// A span that has ended is less likely to be updated again, so it is evicted first.
fn span_has_ended(span: &Value) -> bool {
    span.get("endTimeUnixNano").is_some()
}

/// What the client knows about a trace it wrote to.
#[derive(Debug, Default, Clone)]
struct TraceContext {
    /// Span id of the trace's root span (the synthetic `trace()` span or the first
    /// parentless observation).
    root_span_id: Option<String>,
    /// Whether the root is the synthetic span written by `trace()`.
    root_is_synthetic: bool,
    user_id: Option<String>,
    session_id: Option<String>,
    tags: Option<Vec<String>>,
    release: Option<String>,
    version: Option<String>,
}

/// Write-side state for Langfuse v4, shared by a client, its clones and its batchers.
///
/// Langfuse v4 stores every OTel span as one complete row, keyed by its start time and span
/// id, and does not merge rows field by field. The state therefore keeps:
///
/// - the last full span sent per span id, so an update re-sends the complete span with the
///   original start time (instead of a partial span that would become a second row);
/// - per trace, the root span and the trace attributes (user id, session id, tags, release,
///   version), which are stamped on every observation span of the trace, as Langfuse
///   requires for filtering observations by them.
///
/// Each map is a least-recently-used cache holding at most `capacity` entries (default
/// [`STATE_CAPACITY`]); the cached spans also stay within `max_bytes` of serialized JSON
/// (default [`STATE_MAX_BYTES`]), evicting ended spans first.
#[derive(Debug)]
pub(crate) struct V4State {
    spans: Lru<Value>,
    traces: Lru<TraceContext>,
    /// Ingestion event id -> span id, for the batcher's send step.
    events: Lru<String>,
}

impl Default for V4State {
    fn default() -> Self {
        Self::new(STATE_CAPACITY, STATE_MAX_BYTES)
    }
}

impl V4State {
    pub(crate) fn new(capacity: usize, max_bytes: usize) -> Self {
        Self {
            spans: Lru::new(capacity, max_bytes, span_has_ended),
            traces: Lru::new(capacity, usize::MAX, always),
            events: Lru::new(capacity.saturating_mul(2), usize::MAX, always),
        }
    }

    fn store_span(&mut self, span_id: String, span: Value) {
        let weight = serde_json::to_vec(&span).map_or(0, |bytes| bytes.len());
        self.spans.insert(span_id, span, weight);
    }
}

/// Trace attributes stamped on every observation span of a trace.
const PROPAGATED_KEYS: [&str; 5] = [
    "user.id",
    "session.id",
    "langfuse.trace.tags",
    "langfuse.release",
    "langfuse.version",
];

fn span_str<'a>(span: &'a Value, key: &str) -> Option<&'a str> {
    span.get(key).and_then(Value::as_str)
}

fn attributes_mut(span: &mut Value) -> &mut Vec<Value> {
    if !span.get("attributes").is_some_and(Value::is_array) {
        span["attributes"] = Value::Array(Vec::new());
    }
    match span.get_mut("attributes") {
        Some(Value::Array(attributes)) => attributes,
        _ => unreachable!("attributes was just set to an array"),
    }
}

fn find_attribute<'a>(span: &'a Value, key: &str) -> Option<&'a Value> {
    span.get("attributes")?
        .as_array()?
        .iter()
        .find(|a| a["key"] == key)
        .map(|a| &a["value"])
}

/// Set an attribute, replacing an existing one with the same key.
fn set_attribute(span: &mut Value, key: &str, value: Value) {
    let attributes = attributes_mut(span);
    match attributes.iter_mut().find(|a| a["key"] == key) {
        Some(existing) => existing["value"] = value,
        None => attributes.push(json!({ "key": key, "value": value })),
    }
}

fn is_metadata_object_key(key: &str) -> bool {
    key == "langfuse.observation.metadata" || key == "langfuse.trace.metadata"
}

/// Merge the attribute `value` for `key` into `span`. The JSON metadata objects are merged key
/// by key; every other attribute is replaced.
fn merge_attribute(span: &mut Value, key: &str, value: &Value) {
    if is_metadata_object_key(key) {
        let parse = |v: Option<&Value>| {
            v.and_then(|v| v["stringValue"].as_str())
                .and_then(|s| serde_json::from_str::<Map<String, Value>>(s).ok())
        };
        let merged = match (parse(find_attribute(span, key)), parse(Some(value))) {
            (Some(mut existing), Some(update)) => {
                existing.extend(update);
                Some(existing)
            }
            (None, update) => update,
            (Some(existing), None) => Some(existing),
        };
        if let Some(merged) = merged {
            replace_metadata(span, key, merged);
            return;
        }
    }
    set_attribute(span, key, value.clone());
}

/// Set the metadata object attribute `prefix` and rebuild its per-key attributes
/// (`<prefix>.<key>`, string values only) from the object, so no stale per-key value can
/// override the object on the server.
fn replace_metadata(span: &mut Value, prefix: &str, object: Map<String, Value>) {
    let key_prefix = format!("{prefix}.");
    attributes_mut(span).retain(|a| {
        !a["key"]
            .as_str()
            .is_some_and(|k| k.starts_with(&key_prefix))
    });
    for (key, value) in &object {
        if let Value::String(s) = value {
            set_attribute(
                span,
                &format!("{prefix}.{key}"),
                json!({ "stringValue": s }),
            );
        }
    }
    set_attribute(
        span,
        prefix,
        json!({ "stringValue": Value::Object(object).to_string() }),
    );
}

/// Merge the changed fields of `update` into the full span `cached`. The start time of the
/// cached span is kept, so the result replaces the stored row instead of adding one.
fn merge_span(cached: &Value, update: &Value) -> Value {
    let mut merged = cached.clone();
    if let Some(attributes) = update.get("attributes").and_then(Value::as_array) {
        for attribute in attributes {
            if let Some(key) = attribute["key"].as_str() {
                merge_attribute(&mut merged, key, &attribute["value"]);
            }
        }
    }
    if let Some(name) = span_str(update, "name").filter(|n| !n.is_empty()) {
        merged["name"] = Value::String(name.to_owned());
    }
    for key in ["endTimeUnixNano", "parentSpanId", "status"] {
        if let Some(value) = update.get(key) {
            merged[key] = value.clone();
        }
    }
    if merged.get("startTimeUnixNano").is_none() {
        if let Some(start) = update.get("startTimeUnixNano") {
            merged["startTimeUnixNano"] = start.clone();
        }
    }
    if let (Some(original), Some(requested)) = (
        span_str(cached, "startTimeUnixNano"),
        span_str(update, "startTimeUnixNano"),
    ) {
        if original != requested {
            tracing::warn!(
                span_id = span_str(cached, "spanId").unwrap_or_default(),
                "Langfuse v4 keys spans by start time; keeping the original start time"
            );
        }
    }
    merged
}

/// Stamp the trace attributes on an observation span. Trace-level values replace older ones;
/// an observation's own `langfuse.version` is kept.
fn stamp_trace_context(span: &mut Value, context: &TraceContext) {
    let string = |v: &Option<String>| v.as_ref().map(|s| json!({ "stringValue": s }));
    let tags = context.tags.as_ref().map(|tags| {
        let values: Vec<Value> = tags.iter().map(|t| json!({ "stringValue": t })).collect();
        json!({ "arrayValue": { "values": values } })
    });
    for (key, value) in PROPAGATED_KEYS.iter().zip([
        string(&context.user_id),
        string(&context.session_id),
        tags,
        string(&context.release),
        string(&context.version),
    ]) {
        let Some(value) = value else { continue };
        if *key == "langfuse.version" && find_attribute(span, key).is_some() {
            continue;
        }
        set_attribute(span, key, value);
    }
}

/// Keep only the trace-level attributes of a synthetic root span, for merging into a root that
/// is a real observation.
fn trace_attributes_for_observation_root(raw_root: &Value, root: &Value) -> Value {
    let mut filtered = Vec::new();
    if let Some(attributes) = raw_root.get("attributes").and_then(Value::as_array) {
        for attribute in attributes {
            let key = attribute["key"].as_str().unwrap_or_default();
            let keep = key.starts_with("langfuse.trace.")
                || key == "user.id"
                || key == "session.id"
                || key == "langfuse.release"
                || ((key == "langfuse.version" || key == "langfuse.environment")
                    && find_attribute(root, key).is_none());
            if keep {
                filtered.push(attribute.clone());
            }
        }
    }
    json!({ "attributes": filtered })
}

/// An observation span encoded from one ingestion event, before the state is applied.
struct RawObservation {
    span: Value,
    is_update: bool,
}

fn raw_observation(event: &IngestionEvent) -> Result<Option<RawObservation>> {
    use IngestionEvent as E;
    let is_update = matches!(
        event,
        E::UpdateSpanEvent(_) | E::UpdateGenerationEvent(_) | E::UpdateObservationEvent(_)
    );
    match encode_event(event)? {
        V4Item::Span(span) if !matches!(event, E::TraceEvent(_)) => {
            Ok(Some(RawObservation { span, is_update }))
        }
        _ => Ok(None),
    }
}

fn event_id(event: &IngestionEvent) -> &str {
    use IngestionEvent as E;
    match event {
        E::TraceEvent(e) => &e.id,
        E::ScoreEvent(e) => &e.id,
        E::CreateSpanEvent(e) => &e.id,
        E::UpdateSpanEvent(e) => &e.id,
        E::CreateGenerationEvent(e) => &e.id,
        E::UpdateGenerationEvent(e) => &e.id,
        E::CreateEventEvent(e) => &e.id,
        E::CreateObservationEvent(e) => &e.id,
        E::UpdateObservationEvent(e) => &e.id,
    }
}

fn unknown_span_error(span_hex: &str) -> Error {
    Error::Validation(format!(
        "cannot update observation {span_hex} with Langfuse v4: this client has not written it \
         (or no longer remembers it). Langfuse v4 stores each span as one complete row, so an \
         update must re-send the whole span. Create the observation with this client first, or \
         create it once with its end time and output instead of updating it"
    ))
}

impl V4State {
    /// Check that [`V4State::apply`] would accept `event`, without changing the state.
    pub(crate) fn validate(&self, event: &IngestionEvent) -> Result<()> {
        if let Some(raw) = raw_observation(event)? {
            let span_hex = span_str(&raw.span, "spanId").unwrap_or_default();
            if raw.is_update && !self.spans.contains(span_hex) {
                return Err(unknown_span_error(span_hex));
            }
        } else {
            encode_event(event)?;
        }
        Ok(())
    }

    /// Translate one ingestion event, applying and updating the state.
    ///
    /// Spans come back complete: an update returns the cached span with the changes merged in
    /// and its original start time. An update of a span this state does not know fails with
    /// [`Error::Validation`].
    pub(crate) fn apply(&mut self, event: &IngestionEvent) -> Result<V4Item> {
        if let IngestionEvent::TraceEvent(e) = event {
            let span = self.apply_trace(&e.body, parse_envelope_time(&e.timestamp))?;
            self.remember_event(event, &span);
            return Ok(V4Item::Span(span));
        }
        let Some(raw) = raw_observation(event)? else {
            // Scores carry no span state.
            return encode_event(event);
        };
        let span = self.apply_observation(raw)?;
        self.remember_event(event, &span);
        Ok(V4Item::Span(span))
    }

    fn remember_event(&mut self, event: &IngestionEvent, span: &Value) {
        if let Some(span_id) = span_str(span, "spanId") {
            self.events
                .insert(event_id(event).to_owned(), span_id.to_owned(), 0);
        }
    }

    /// The latest full span for a batched event, if the state still holds it.
    pub(crate) fn span_for_event(&mut self, event_id: &str) -> Option<Value> {
        let span_id = self.events.get(event_id)?.clone();
        self.spans.get(&span_id).cloned()
    }

    /// Record trace attributes for `trace_id` (any Langfuse trace id), keeping values that are
    /// not given.
    pub(crate) fn remember_trace_attributes(
        &mut self,
        trace_id_value: &str,
        user_id: Option<&str>,
        session_id: Option<&str>,
        tags: Option<&[String]>,
    ) {
        let trace_hex = trace_id(trace_id_value);
        let mut context = self.traces.get(&trace_hex).cloned().unwrap_or_default();
        if let Some(user_id) = user_id {
            context.user_id = Some(user_id.to_owned());
        }
        if let Some(session_id) = session_id {
            context.session_id = Some(session_id.to_owned());
        }
        if let Some(tags) = tags {
            context.tags = Some(tags.to_vec());
        }
        self.traces.insert(trace_hex, context, 0);
    }

    fn apply_observation(&mut self, raw: RawObservation) -> Result<Value> {
        let trace_hex = span_str(&raw.span, "traceId")
            .unwrap_or_default()
            .to_owned();
        let span_hex = span_str(&raw.span, "spanId").unwrap_or_default().to_owned();
        let mut context = self.traces.get(&trace_hex).cloned().unwrap_or_default();

        let mut span = match self.spans.get(&span_hex) {
            Some(cached) => merge_span(cached, &raw.span),
            None if raw.is_update => return Err(unknown_span_error(&span_hex)),
            None => raw.span,
        };

        if span.get("parentSpanId").is_none() {
            match &context.root_span_id {
                Some(root) if *root != span_hex => {
                    span["parentSpanId"] = Value::String(root.clone());
                }
                Some(_) => {}
                None => {
                    // No root known for this trace: this observation becomes the root.
                    context.root_span_id = Some(span_hex.clone());
                    context.root_is_synthetic = false;
                    self.traces.insert(trace_hex.clone(), context.clone(), 0);
                }
            }
        }
        stamp_trace_context(&mut span, &context);
        self.store_span(span_hex, span.clone());
        Ok(span)
    }

    fn apply_trace(
        &mut self,
        body: &langfuse_client_base::models::TraceBody,
        envelope_time: Option<DateTime<FixedOffset>>,
    ) -> Result<Value> {
        let raw = encode_trace(body, envelope_time)?;
        let trace_hex = span_str(&raw, "traceId").unwrap_or_default().to_owned();
        let mut context = self.traces.get(&trace_hex).cloned().unwrap_or_default();
        if let Some(user_id) = flat_str(&body.user_id) {
            context.user_id = Some(user_id.to_owned());
        }
        if let Some(session_id) = flat_str(&body.session_id) {
            context.session_id = Some(session_id.to_owned());
        }
        if let Some(tags) = flat(&body.tags) {
            context.tags = Some(tags.clone());
        }
        if let Some(release) = flat_str(&body.release) {
            context.release = Some(release.to_owned());
        }
        if let Some(version) = flat_str(&body.version) {
            context.version = Some(version.to_owned());
        }

        let cached_root = context
            .root_span_id
            .as_ref()
            .and_then(|root| self.spans.get(root))
            .cloned();
        let span = match cached_root {
            // `trace()` again: update the synthetic root, keeping its original times.
            Some(root) if context.root_is_synthetic => {
                let mut merged = merge_span(&root, &raw);
                merged["endTimeUnixNano"] =
                    root.get("endTimeUnixNano").cloned().unwrap_or(Value::Null);
                if merged["endTimeUnixNano"].is_null() {
                    if let Some(map) = merged.as_object_mut() {
                        map.remove("endTimeUnixNano");
                    }
                }
                merged
            }
            // The root is an observation written before `trace()`: add the trace attributes
            // to it rather than creating a second root.
            Some(root) => {
                let mut merged =
                    merge_span(&root, &trace_attributes_for_observation_root(&raw, &root));
                stamp_trace_context(&mut merged, &context);
                merged
            }
            None => {
                if context.root_span_id.is_some() && !context.root_is_synthetic {
                    tracing::warn!(
                        trace_id = trace_hex.as_str(),
                        "the root observation of this trace is no longer cached; writing a \
                         separate trace root span"
                    );
                }
                context.root_span_id = span_str(&raw, "spanId").map(str::to_owned);
                context.root_is_synthetic = true;
                raw
            }
        };
        self.traces.insert(trace_hex, context, 0);
        if let Some(span_id) = span_str(&span, "spanId") {
            self.store_span(span_id.to_owned(), span.clone());
        }
        Ok(span)
    }
}

/// Keep the last span per span id, in first-seen order.
pub(crate) fn dedupe_spans(spans: Vec<Value>) -> Vec<Value> {
    let mut order: Vec<String> = Vec::new();
    let mut latest: HashMap<String, Value> = HashMap::new();
    for span in spans {
        let id = span_str(&span, "spanId").unwrap_or_default().to_owned();
        if !latest.contains_key(&id) {
            order.push(id.clone());
        }
        latest.insert(id, span);
    }
    order
        .into_iter()
        .filter_map(|id| latest.remove(&id))
        .collect()
}

/// Wrap spans in an OTLP `ExportTraceServiceRequest` (JSON encoding).
pub(crate) fn export_request(spans: Vec<Value>) -> Value {
    json!({
        "resourceSpans": [{
            "resource": {
                "attributes": [
                    { "key": "service.name", "value": { "stringValue": SCOPE_NAME } }
                ]
            },
            "scopeSpans": [{
                "scope": { "name": SCOPE_NAME, "version": env!("CARGO_PKG_VERSION") },
                "spans": spans
            }]
        }]
    })
}

/// Log OTLP partial success (rejected spans), which carries no per-span detail.
pub(crate) async fn log_partial_success(response: reqwest::Response) {
    let Ok(body) = response.text().await else {
        return;
    };
    let Ok(value) = serde_json::from_str::<Value>(&body) else {
        return;
    };
    let Some(partial) = value.get("partialSuccess") else {
        return;
    };
    let rejected = match partial.get("rejectedSpans") {
        Some(Value::String(s)) => s.parse::<u64>().unwrap_or(0),
        Some(v) => v.as_u64().unwrap_or(0),
        None => 0,
    };
    if rejected > 0 {
        let error_message = partial
            .get("errorMessage")
            .and_then(|m| m.as_str())
            .unwrap_or_default()
            .to_owned();
        tracing::warn!(
            rejected_spans = rejected,
            error_message,
            "Langfuse rejected part of an OTLP export"
        );
    }
}

impl LangfuseClient {
    /// POST spans to the OTLP endpoint and return the raw response.
    pub(crate) async fn post_otlp(&self, spans: Vec<Value>) -> Result<reqwest::Response> {
        let body = serde_json::to_vec(&export_request(spans))?;
        let mut request = self
            .configuration
            .client
            .post(format!(
                "{}{OTLP_TRACES_PATH}",
                self.configuration.base_path
            ))
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(INGESTION_VERSION_HEADER, "4");
        if let Some(user_agent) = &self.configuration.user_agent {
            request = request.header(reqwest::header::USER_AGENT, user_agent);
        }
        request.body(body).send().await.map_err(Error::Middleware)
    }

    /// Export spans, mapping a non-success status to an [`Error`].
    pub(crate) async fn export_spans(&self, spans: Vec<Value>) -> Result<()> {
        let response = self.post_otlp(spans).await?;
        if response.status().is_success() {
            log_partial_success(response).await;
            Ok(())
        } else {
            Err(crate::error::error_from_response(response).await)
        }
    }

    /// Create one score with `POST /api/public/scores`.
    pub(crate) async fn create_score_v4(&self, score: CreateScoreRequest) -> Result<()> {
        use langfuse_client_base::apis::scores_api;
        use langfuse_client_base::models::CreateScoresRequest;

        let request = CreateScoresRequest::CreateScoreRequest(score);
        scores_api::scores_create()
            .configuration(self.configuration())
            .create_scores_request(&request)
            .call()
            .await
            .map(|_| ())
            .map_err(crate::error::map_api_error)
    }

    /// Lock the shared V4 write state.
    pub(crate) fn v4_state(&self) -> std::sync::MutexGuard<'_, V4State> {
        self.v4_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Send ingestion events through the v4 APIs.
    pub(crate) async fn ingest_events_v4(&self, events: Vec<IngestionEvent>) -> Result<()> {
        let mut spans = Vec::new();
        let mut scores = Vec::new();
        {
            let mut state = self.v4_state();
            for event in &events {
                match state.apply(event)? {
                    V4Item::Span(span) => spans.push(span),
                    V4Item::Score(score) => scores.push(*score),
                }
            }
        }
        if !spans.is_empty() {
            self.export_spans(dedupe_spans(spans)).await?;
        }
        for score in scores {
            self.create_score_v4(score).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use langfuse_client_base::models::{
        ingestion_event_create_event_event, ingestion_event_create_generation_event,
        ingestion_event_create_span_event, ingestion_event_score_event,
        ingestion_event_trace_event, ingestion_event_update_span_event, CreateEventBody,
        CreateGenerationBody, CreateScoreValue, CreateSpanBody, IngestionEventCreateEventEvent,
        IngestionEventCreateGenerationEvent, IngestionEventCreateSpanEvent,
        IngestionEventScoreEvent, IngestionEventTraceEvent, IngestionEventUpdateSpanEvent,
        ScoreDataType, TraceBody, UpdateSpanBody,
    };

    const TS: &str = "2026-11-10T12:00:00.000Z";

    fn time(s: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(s).unwrap()
    }

    fn span_of(event: IngestionEvent) -> Value {
        match encode_event(&event).unwrap() {
            V4Item::Span(span) => span,
            V4Item::Score(_) => panic!("expected a span"),
        }
    }

    /// Collect the attributes of an encoded span as `key -> value object`.
    fn attrs(span: &Value) -> HashMap<String, Value> {
        span["attributes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| (a["key"].as_str().unwrap().to_owned(), a["value"].clone()))
            .collect()
    }

    fn string_attr(attrs: &HashMap<String, Value>, key: &str) -> String {
        attrs
            .get(key)
            .unwrap_or_else(|| panic!("missing attribute {key}"))["stringValue"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn trace_ids_pass_through_valid_otel_ids_and_uuids() {
        let hex = "0af7651916cd43dd8448eb211c80319c";
        assert_eq!(trace_id(hex), hex);
        assert_eq!(trace_id(&hex.to_uppercase()), hex);
        assert_eq!(
            trace_id("0af76519-16cd-43dd-8448-eb211c80319c"),
            "0af7651916cd43dd8448eb211c80319c"
        );
    }

    #[test]
    fn trace_ids_hash_other_strings_deterministically() {
        let mapped = trace_id("my-trace");
        assert_eq!(mapped.len(), 32);
        assert!(mapped
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        assert_eq!(mapped, trace_id("my-trace"));
        assert_ne!(mapped, trace_id("my-trace-2"));
        // The all-zero id is invalid in OTel and is hashed instead.
        let zero = "0".repeat(32);
        assert_ne!(trace_id(&zero), zero);
        assert_ne!(trace_id("00000000-0000-0000-0000-000000000000"), zero);
    }

    #[test]
    fn observation_ids_map_to_span_ids() {
        assert_eq!(observation_id("b7ad6b7169203331"), "b7ad6b7169203331");
        assert_eq!(observation_id("B7AD6B7169203331"), "b7ad6b7169203331");
        let from_uuid = observation_id("0af76519-16cd-43dd-8448-eb211c80319c");
        assert_eq!(from_uuid.len(), 16);
        assert_eq!(
            from_uuid,
            observation_id("0af76519-16cd-43dd-8448-eb211c80319c")
        );
        assert_ne!(observation_id(&"0".repeat(16)), "0".repeat(16));
    }

    #[test]
    fn id_mapping_is_idempotent() {
        for id in ["my-trace", "550e8400-e29b-41d4-a716-446655440000", "x", ""] {
            let t = trace_id(id);
            assert_eq!(trace_id(&t), t, "{id}");
            let o = observation_id(id);
            assert_eq!(observation_id(&o), o, "{id}");
        }
    }

    #[test]
    fn root_span_id_is_stable_across_trace_id_spellings() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            trace_root_span_id(uuid),
            trace_root_span_id(&trace_id(uuid))
        );
        assert_eq!(trace_root_span_id(uuid).len(), 16);
        assert_ne!(trace_root_span_id("a"), trace_root_span_id("b"));
    }

    #[test]
    fn trace_event_becomes_root_span_with_trace_attributes() {
        let body = TraceBody {
            id: Some(Some("trace-1".into())),
            timestamp: Some(Some(time(TS))),
            name: Some(Some("chat".into())),
            user_id: Some(Some("user-1".into())),
            session_id: Some(Some("session-1".into())),
            input: Some(Some(json!({"q": "hi"}))),
            output: Some(Some(json!("hello"))),
            release: Some(Some("r1".into())),
            version: Some(Some("v1".into())),
            metadata: Some(Some(json!({"team": "a", "n": 1, "skip": null}))),
            tags: Some(Some(vec!["t1".into(), "t2".into()])),
            environment: Some(Some("prod".into())),
            public: Some(Some(true)),
        };
        let span = span_of(IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
            "ev-1".into(),
            TS.into(),
            body,
            ingestion_event_trace_event::Type::TraceCreate,
        )));

        assert_eq!(span["traceId"], trace_id("trace-1"));
        assert_eq!(span["spanId"], trace_root_span_id("trace-1"));
        assert!(span.get("parentSpanId").is_none());
        assert_eq!(span["name"], "chat");
        assert_eq!(span["kind"], 1);
        assert_eq!(span["startTimeUnixNano"], "1794312000000000000");
        assert_eq!(span["endTimeUnixNano"], "1794312000000000000");

        let a = attrs(&span);
        assert_eq!(string_attr(&a, "langfuse.observation.type"), "span");
        assert_eq!(string_attr(&a, "langfuse.trace.name"), "chat");
        assert_eq!(string_attr(&a, "user.id"), "user-1");
        assert_eq!(string_attr(&a, "session.id"), "session-1");
        assert_eq!(
            a["langfuse.trace.tags"],
            json!({"arrayValue": {"values": [{"stringValue": "t1"}, {"stringValue": "t2"}]}})
        );
        assert_eq!(a["langfuse.trace.public"], json!({"boolValue": true}));
        // String values per key (filterable), and the whole object with its types.
        assert_eq!(string_attr(&a, "langfuse.trace.metadata.team"), "a");
        assert!(!a.contains_key("langfuse.trace.metadata.n"));
        assert!(!a.contains_key("langfuse.trace.metadata.skip"));
        let object: Value =
            serde_json::from_str(&string_attr(&a, "langfuse.trace.metadata")).unwrap();
        assert_eq!(object, json!({"team": "a", "n": 1}));
        assert_eq!(string_attr(&a, "langfuse.trace.input"), r#"{"q":"hi"}"#);
        assert_eq!(string_attr(&a, "langfuse.trace.output"), "hello");
        assert_eq!(
            string_attr(&a, "langfuse.observation.input"),
            r#"{"q":"hi"}"#
        );
        assert_eq!(string_attr(&a, "langfuse.observation.output"), "hello");
        assert_eq!(string_attr(&a, "langfuse.release"), "r1");
        assert_eq!(string_attr(&a, "langfuse.version"), "v1");
        assert_eq!(string_attr(&a, "langfuse.environment"), "prod");
    }

    #[test]
    fn raw_span_encodes_only_explicit_parent() {
        let body = CreateSpanBody {
            id: Some(Some("span-1".into())),
            trace_id: Some(Some("trace-1".into())),
            name: Some(Some("retrieve".into())),
            start_time: Some(Some(time(TS))),
            end_time: Some(Some(time("2026-11-10T12:00:01.500Z"))),
            level: Some(ObservationLevel::Error),
            status_message: Some(Some("boom".into())),
            metadata: Some(Some(json!("plain"))),
            ..CreateSpanBody::default()
        };
        let span = span_of(IngestionEvent::CreateSpanEvent(
            IngestionEventCreateSpanEvent::new(
                "ev-2".into(),
                TS.into(),
                body,
                ingestion_event_create_span_event::Type::SpanCreate,
            ),
        ));
        assert_eq!(span["spanId"], observation_id("span-1"));
        assert!(span.get("parentSpanId").is_none());
        assert_eq!(span["endTimeUnixNano"], "1794312001500000000");
        assert_eq!(span["status"], json!({"code": 2, "message": "boom"}));
        let a = attrs(&span);
        assert_eq!(string_attr(&a, "langfuse.observation.type"), "span");
        assert_eq!(string_attr(&a, "langfuse.observation.level"), "ERROR");
        assert_eq!(
            string_attr(&a, "langfuse.observation.status_message"),
            "boom"
        );
        assert_eq!(
            string_attr(&a, "langfuse.observation.metadata.value"),
            "plain"
        );
        assert_eq!(
            string_attr(&a, "langfuse.observation.metadata"),
            r#"{"value":"plain"}"#
        );
    }

    #[test]
    fn generation_maps_model_usage_cost_and_prompt() {
        let body = CreateGenerationBody {
            id: Some(Some("gen-1".into())),
            trace_id: Some(Some("trace-1".into())),
            parent_observation_id: Some(Some("span-1".into())),
            name: Some(Some("llm".into())),
            start_time: Some(Some(time(TS))),
            completion_start_time: Some(Some(time("2026-11-10T12:00:00.250Z"))),
            model: Some(Some("gpt-4o".into())),
            model_parameters: Some(Some(HashMap::from([(
                "temperature".to_owned(),
                MapValue::FloatNumber(0.5),
            )]))),
            usage_details: Some(UsageDetails::Object(HashMap::from([(
                "input".to_owned(),
                10,
            )]))),
            cost_details: Some(Some(HashMap::from([("total".to_owned(), 0.25)]))),
            prompt_name: Some(Some("p".into())),
            prompt_version: Some(Some(3)),
            ..CreateGenerationBody::default()
        };
        let span = span_of(IngestionEvent::CreateGenerationEvent(
            IngestionEventCreateGenerationEvent::new(
                "ev-3".into(),
                TS.into(),
                body,
                ingestion_event_create_generation_event::Type::GenerationCreate,
            ),
        ));
        assert_eq!(span["parentSpanId"], observation_id("span-1"));
        assert!(span.get("endTimeUnixNano").is_none());
        assert!(span.get("status").is_none());
        let a = attrs(&span);
        assert_eq!(string_attr(&a, "langfuse.observation.type"), "generation");
        assert_eq!(string_attr(&a, "langfuse.observation.model.name"), "gpt-4o");
        assert_eq!(
            string_attr(&a, "langfuse.observation.model.parameters"),
            r#"{"temperature":0.5}"#
        );
        assert_eq!(
            string_attr(&a, "langfuse.observation.usage_details"),
            r#"{"input":10}"#
        );
        assert_eq!(
            string_attr(&a, "langfuse.observation.cost_details"),
            r#"{"total":0.25}"#
        );
        assert_eq!(
            string_attr(&a, "langfuse.observation.completion_start_time"),
            "2026-11-10T12:00:00.250Z"
        );
        assert_eq!(string_attr(&a, "langfuse.observation.prompt.name"), "p");
        assert_eq!(
            a["langfuse.observation.prompt.version"],
            json!({"intValue": "3"})
        );
    }

    #[test]
    fn openai_and_legacy_usage_are_normalised() {
        use langfuse_client_base::models::{OpenAiCompletionUsageSchema, Usage};
        let schema = UsageDetails::OpenAiCompletionUsageSchema(OpenAiCompletionUsageSchema {
            prompt_tokens: 3,
            completion_tokens: 4,
            total_tokens: 7,
            prompt_tokens_details: Some(Some(HashMap::from([("cached_tokens".into(), Some(1))]))),
            completion_tokens_details: None,
        });
        let (usage, cost) = usage_details_json(Some(&schema), None);
        assert_eq!(
            usage.unwrap(),
            json!({"input": 3, "output": 4, "total": 7, "input_cached_tokens": 1})
        );
        assert!(cost.is_none());

        let legacy = IngestionUsage::Usage(Usage {
            input: 1,
            output: 2,
            total: 3,
            unit: None,
            input_cost: Some(Some(0.1)),
            output_cost: None,
            total_cost: Some(Some(0.3)),
        });
        let (usage, cost) = usage_details_json(None, Some(&legacy));
        assert_eq!(usage.unwrap(), json!({"input": 1, "output": 2, "total": 3}));
        assert_eq!(cost.unwrap(), json!({"input": 0.1, "total": 0.3}));
    }

    #[test]
    fn null_token_detail_counts_are_skipped() {
        use langfuse_client_base::models::{
            OpenAiCompletionUsageSchema, OpenAiResponseUsageSchema,
        };
        let completion = UsageDetails::OpenAiCompletionUsageSchema(OpenAiCompletionUsageSchema {
            prompt_tokens: 3,
            completion_tokens: 4,
            total_tokens: 7,
            prompt_tokens_details: Some(Some(HashMap::from([
                ("cached_tokens".into(), Some(1)),
                ("audio_tokens".into(), None),
            ]))),
            completion_tokens_details: Some(Some(HashMap::from([(
                "reasoning_tokens".into(),
                None,
            )]))),
        });
        let (usage, _) = usage_details_json(Some(&completion), None);
        assert_eq!(
            usage.unwrap(),
            json!({"input": 3, "output": 4, "total": 7, "input_cached_tokens": 1})
        );

        let response = UsageDetails::OpenAiResponseUsageSchema(OpenAiResponseUsageSchema {
            input_tokens: 5,
            output_tokens: 6,
            total_tokens: 11,
            input_tokens_details: Some(Some(HashMap::from([("cached_tokens".into(), None)]))),
            output_tokens_details: Some(Some(HashMap::from([(
                "reasoning_tokens".into(),
                Some(2),
            )]))),
        });
        let (usage, _) = usage_details_json(Some(&response), None);
        assert_eq!(
            usage.unwrap(),
            json!({"input": 5, "output": 6, "total": 11, "output_reasoning_tokens": 2})
        );
    }

    #[test]
    fn event_has_zero_duration() {
        let body = CreateEventBody {
            id: Some(Some("event-1".into())),
            trace_id: Some(Some("trace-1".into())),
            name: Some(Some("click".into())),
            start_time: Some(Some(time(TS))),
            ..CreateEventBody::default()
        };
        let span = span_of(IngestionEvent::CreateEventEvent(
            IngestionEventCreateEventEvent::new(
                "ev-4".into(),
                TS.into(),
                body,
                ingestion_event_create_event_event::Type::EventCreate,
            ),
        ));
        assert_eq!(span["startTimeUnixNano"], span["endTimeUnixNano"]);
        assert_eq!(
            string_attr(&attrs(&span), "langfuse.observation.type"),
            "event"
        );
    }

    fn trace_event(id: &str, ts: &str, body: TraceBody) -> IngestionEvent {
        IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
            format!("ev-{id}-{ts}"),
            ts.into(),
            TraceBody {
                id: Some(Some(id.into())),
                ..body
            },
            ingestion_event_trace_event::Type::TraceCreate,
        ))
    }

    fn create_span(id: &str, trace: &str, body: CreateSpanBody) -> IngestionEvent {
        IngestionEvent::CreateSpanEvent(IngestionEventCreateSpanEvent::new(
            format!("ev-{id}"),
            TS.into(),
            CreateSpanBody {
                id: Some(Some(id.into())),
                trace_id: Some(Some(trace.into())),
                ..body
            },
            ingestion_event_create_span_event::Type::SpanCreate,
        ))
    }

    fn update_span(id: &str, trace: &str, body: UpdateSpanBody) -> IngestionEvent {
        IngestionEvent::UpdateSpanEvent(IngestionEventUpdateSpanEvent::new(
            format!("ev-update-{id}"),
            "2026-11-10T13:00:00.000Z".into(),
            UpdateSpanBody {
                trace_id: Some(Some(trace.into())),
                ..body
            },
            ingestion_event_update_span_event::Type::SpanUpdate,
        ))
    }

    fn applied(state: &mut V4State, event: IngestionEvent) -> Value {
        match state.apply(&event).unwrap() {
            V4Item::Span(span) => span,
            V4Item::Score(_) => panic!("expected a span"),
        }
    }

    #[test]
    fn update_resends_full_span_with_original_start() {
        let mut state = V4State::default();
        let created = applied(
            &mut state,
            create_span(
                "span-1",
                "trace-1",
                CreateSpanBody {
                    name: Some(Some("retrieve".into())),
                    start_time: Some(Some(time(TS))),
                    input: Some(Some(json!({"q": 1}))),
                    metadata: Some(Some(json!({"a": "x", "n": 1}))),
                    ..CreateSpanBody::default()
                },
            ),
        );
        let mut update = UpdateSpanBody::new("span-1".into());
        update.end_time = Some(Some(time("2026-11-10T12:00:02.000Z")));
        update.start_time = Some(Some(time("2026-11-10T12:30:00.000Z")));
        update.output = Some(Some(json!({"done": true})));
        update.metadata = Some(Some(json!({"b": "y"})));
        let updated = applied(&mut state, update_span("span-1", "trace-1", update));

        assert_eq!(updated["spanId"], created["spanId"]);
        // The original start time keeps the row key; the requested start is ignored.
        assert_eq!(updated["startTimeUnixNano"], "1794312000000000000");
        assert_eq!(updated["endTimeUnixNano"], "1794312002000000000");
        assert_eq!(updated["name"], "retrieve");
        let a = attrs(&updated);
        assert_eq!(string_attr(&a, "langfuse.observation.input"), r#"{"q":1}"#);
        assert_eq!(
            string_attr(&a, "langfuse.observation.output"),
            r#"{"done":true}"#
        );
        assert_eq!(string_attr(&a, "langfuse.observation.type"), "span");
        assert_eq!(string_attr(&a, "langfuse.observation.metadata.a"), "x");
        assert_eq!(string_attr(&a, "langfuse.observation.metadata.b"), "y");
        let object: Value =
            serde_json::from_str(&string_attr(&a, "langfuse.observation.metadata")).unwrap();
        assert_eq!(object, json!({"a": "x", "n": 1, "b": "y"}));
        // The batcher's send step finds the latest full span through the event id.
        assert_eq!(state.span_for_event("ev-span-1").unwrap(), updated);
    }

    #[test]
    fn update_of_unknown_span_is_rejected() {
        let mut state = V4State::default();
        let event = update_span("missing", "trace-1", UpdateSpanBody::new("missing".into()));
        assert!(matches!(state.apply(&event), Err(Error::Validation(_))));
    }

    #[test]
    fn observations_are_linked_to_the_trace_root_and_get_trace_attributes() {
        let mut state = V4State::default();
        let root = applied(
            &mut state,
            trace_event(
                "trace-1",
                TS,
                TraceBody {
                    name: Some(Some("chat".into())),
                    user_id: Some(Some("user-1".into())),
                    session_id: Some(Some("session-1".into())),
                    tags: Some(Some(vec!["t".into()])),
                    release: Some(Some("r1".into())),
                    version: Some(Some("v1".into())),
                    ..TraceBody::default()
                },
            ),
        );
        let child = applied(
            &mut state,
            create_span(
                "span-1",
                "trace-1",
                CreateSpanBody {
                    version: Some(Some("span-version".into())),
                    ..CreateSpanBody::default()
                },
            ),
        );
        assert_eq!(child["parentSpanId"], root["spanId"]);
        let a = attrs(&child);
        assert_eq!(string_attr(&a, "user.id"), "user-1");
        assert_eq!(string_attr(&a, "session.id"), "session-1");
        assert_eq!(
            a["langfuse.trace.tags"],
            json!({"arrayValue": {"values": [{"stringValue": "t"}]}})
        );
        assert_eq!(string_attr(&a, "langfuse.release"), "r1");
        // The observation's own version wins over the trace version.
        assert_eq!(string_attr(&a, "langfuse.version"), "span-version");
    }

    #[test]
    fn unknown_trace_gets_its_first_observation_as_root() {
        let mut state = V4State::default();
        let first = applied(
            &mut state,
            create_span(
                "span-1",
                "trace-1",
                CreateSpanBody {
                    name: Some(Some("first".into())),
                    start_time: Some(Some(time(TS))),
                    ..CreateSpanBody::default()
                },
            ),
        );
        assert!(first.get("parentSpanId").is_none());
        let second = applied(
            &mut state,
            create_span("span-2", "trace-1", CreateSpanBody::default()),
        );
        assert_eq!(second["parentSpanId"], first["spanId"]);

        // A later trace() adds the trace attributes to that root instead of a second root.
        let root = applied(
            &mut state,
            trace_event(
                "trace-1",
                "2026-11-10T12:05:00.000Z",
                TraceBody {
                    name: Some(Some("chat".into())),
                    user_id: Some(Some("user-1".into())),
                    input: Some(Some(json!("trace input"))),
                    ..TraceBody::default()
                },
            ),
        );
        assert_eq!(root["spanId"], first["spanId"]);
        assert_eq!(root["name"], "first");
        assert_eq!(root["startTimeUnixNano"], first["startTimeUnixNano"]);
        let a = attrs(&root);
        assert_eq!(string_attr(&a, "langfuse.trace.name"), "chat");
        assert_eq!(string_attr(&a, "user.id"), "user-1");
        assert_eq!(string_attr(&a, "langfuse.trace.input"), "trace input");
        assert!(!a.contains_key("langfuse.observation.input"));
        assert_eq!(string_attr(&a, "langfuse.observation.type"), "span");
    }

    #[test]
    fn trace_called_twice_keeps_the_original_root_times() {
        let mut state = V4State::default();
        let first = applied(
            &mut state,
            trace_event(
                "trace-1",
                TS,
                TraceBody {
                    name: Some(Some("chat".into())),
                    input: Some(Some(json!("in"))),
                    ..TraceBody::default()
                },
            ),
        );
        let second = applied(
            &mut state,
            trace_event(
                "trace-1",
                "2026-11-10T12:10:00.000Z",
                TraceBody {
                    timestamp: Some(Some(time("2026-11-10T12:10:00.000Z"))),
                    output: Some(Some(json!("out"))),
                    ..TraceBody::default()
                },
            ),
        );
        assert_eq!(second["spanId"], first["spanId"]);
        assert_eq!(second["startTimeUnixNano"], first["startTimeUnixNano"]);
        assert_eq!(second["endTimeUnixNano"], first["endTimeUnixNano"]);
        assert_eq!(second["name"], "chat");
        let a = attrs(&second);
        assert_eq!(string_attr(&a, "langfuse.observation.input"), "in");
        assert_eq!(string_attr(&a, "langfuse.observation.output"), "out");
    }

    #[test]
    fn reserved_metadata_keys_are_renamed() {
        let span = span_of(create_span(
            "span-1",
            "trace-1",
            CreateSpanBody {
                metadata: Some(Some(json!({"scope": "mine", "attributes": 1, "ok": "v"}))),
                ..CreateSpanBody::default()
            },
        ));
        let a = attrs(&span);
        assert_eq!(
            string_attr(&a, "langfuse.observation.metadata.user_scope"),
            "mine"
        );
        assert!(!a.contains_key("langfuse.observation.metadata.scope"));
        let object: Value =
            serde_json::from_str(&string_attr(&a, "langfuse.observation.metadata")).unwrap();
        assert_eq!(
            object,
            json!({"user_scope": "mine", "user_attributes": 1, "ok": "v"})
        );
    }

    #[test]
    fn dedupe_keeps_the_last_span_per_id() {
        let spans = vec![
            json!({"spanId": "a", "v": 1}),
            json!({"spanId": "b", "v": 1}),
            json!({"spanId": "a", "v": 2}),
        ];
        assert_eq!(
            dedupe_spans(spans),
            vec![
                json!({"spanId": "a", "v": 2}),
                json!({"spanId": "b", "v": 1})
            ]
        );
    }

    #[test]
    fn lru_refreshes_on_get_and_insert() {
        let mut map: Lru<i32> = Lru::new(2, usize::MAX, always);
        map.insert("a".to_owned(), 1, 0);
        map.insert("b".to_owned(), 2, 0);
        assert_eq!(map.get("a"), Some(&1)); // "a" is now the most recently used
        map.insert("c".to_owned(), 3, 0);
        assert!(map.get("b").is_none());
        assert_eq!(map.get("a"), Some(&1));
        map.insert("c".to_owned(), 4, 0); // re-insert refreshes too
        map.insert("d".to_owned(), 5, 0);
        assert!(map.get("a").is_none());
        assert_eq!(map.get("c"), Some(&4));
        assert_eq!(map.get("d"), Some(&5));
    }

    #[test]
    fn lru_prefers_evicting_ended_spans() {
        let mut map: Lru<Value> = Lru::new(2, usize::MAX, span_has_ended);
        map.insert("open".to_owned(), json!({"spanId": "open"}), 0);
        map.insert(
            "ended".to_owned(),
            json!({"spanId": "ended", "endTimeUnixNano": "1"}),
            0,
        );
        map.insert("new".to_owned(), json!({"spanId": "new"}), 0);
        // "open" is the least recently used, but the ended span goes first.
        assert!(map.get("ended").is_none());
        assert!(map.get("open").is_some());
        assert!(map.get("new").is_some());
    }

    #[test]
    fn lru_stays_within_the_byte_budget() {
        let mut map: Lru<i32> = Lru::new(100, 10, always);
        map.insert("a".to_owned(), 1, 4);
        map.insert("b".to_owned(), 2, 4);
        map.insert("c".to_owned(), 3, 4);
        assert!(map.get("a").is_none());
        assert!(map.get("b").is_some());
        assert!(map.get("c").is_some());
        // A single entry over the budget is kept rather than leaving nothing.
        map.insert("big".to_owned(), 4, 50);
        assert_eq!(map.get("big"), Some(&4));
    }

    #[test]
    fn metadata_type_change_drops_the_stale_per_key_value() {
        let mut state = V4State::default();
        applied(
            &mut state,
            create_span(
                "span-1",
                "trace-1",
                CreateSpanBody {
                    metadata: Some(Some(json!({"a": "x", "keep": "k"}))),
                    ..CreateSpanBody::default()
                },
            ),
        );
        let mut update = UpdateSpanBody::new("span-1".into());
        update.metadata = Some(Some(json!({"a": 5})));
        let updated = applied(&mut state, update_span("span-1", "trace-1", update));
        let a = attrs(&updated);
        assert!(!a.contains_key("langfuse.observation.metadata.a"));
        assert_eq!(string_attr(&a, "langfuse.observation.metadata.keep"), "k");
        let object: Value =
            serde_json::from_str(&string_attr(&a, "langfuse.observation.metadata")).unwrap();
        assert_eq!(object, json!({"a": 5, "keep": "k"}));
    }

    #[test]
    fn validate_does_not_change_the_state() {
        let mut state = V4State::default();
        let create = create_span("span-1", "trace-1", CreateSpanBody::default());
        state.validate(&create).unwrap();
        // Nothing was recorded: the span is unknown and no root was registered.
        let update = update_span("span-1", "trace-1", UpdateSpanBody::new("span-1".into()));
        assert!(matches!(state.validate(&update), Err(Error::Validation(_))));
        let other = applied(
            &mut state,
            create_span("span-2", "trace-1", CreateSpanBody::default()),
        );
        assert!(other.get("parentSpanId").is_none());
    }

    #[test]
    fn observation_without_trace_id_is_rejected() {
        let event = IngestionEvent::CreateSpanEvent(IngestionEventCreateSpanEvent::new(
            "ev-6".into(),
            TS.into(),
            CreateSpanBody {
                id: Some(Some("span-1".into())),
                ..CreateSpanBody::default()
            },
            ingestion_event_create_span_event::Type::SpanCreate,
        ));
        assert!(matches!(encode_event(&event), Err(Error::Validation(_))));
    }

    #[test]
    fn score_maps_ids_and_metadata() {
        let body = ScoreBody {
            id: Some(Some("score-1".into())),
            trace_id: Some(Some("trace-1".into())),
            observation_id: Some(Some("span-1".into())),
            name: "quality".into(),
            value: CreateScoreValue::DoubleNumber(0.9),
            data_type: Some(ScoreDataType::Numeric),
            metadata: Some(Some(json!({"k": "v"}))),
            ..ScoreBody::new("quality".into(), CreateScoreValue::DoubleNumber(0.9))
        };
        let event = IngestionEvent::ScoreEvent(IngestionEventScoreEvent::new(
            "ev-7".into(),
            TS.into(),
            body,
            ingestion_event_score_event::Type::ScoreCreate,
        ));
        let V4Item::Score(score) = encode_event(&event).unwrap() else {
            panic!("expected a score");
        };
        assert_eq!(score.id, Some(Some("score-1".into())));
        assert_eq!(score.trace_id, Some(Some(trace_id("trace-1"))));
        assert_eq!(score.observation_id, Some(Some(observation_id("span-1"))));
        assert_eq!(
            score.metadata,
            Some(Some(HashMap::from([("k".to_owned(), json!("v"))])))
        );
        assert_eq!(score.timestamp, Some(Some(time(TS))));
    }

    #[test]
    fn export_request_wraps_spans_in_one_scope() {
        let request = export_request(vec![json!({"spanId": "1"})]);
        let scope = &request["resourceSpans"][0]["scopeSpans"][0];
        assert_eq!(scope["scope"]["name"], SCOPE_NAME);
        assert_eq!(scope["spans"][0]["spanId"], "1");
    }
}
