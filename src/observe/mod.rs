//! Automatic tracing: the `#[observe]` attribute and the observation context behind it.
//!
//! # Quick start
//!
//! ```no_run
//! # #[cfg(feature = "macros")] {
//! use langfuse_ergonomic::{observe, ClientBuilder};
//!
//! #[observe(as_type = "generation", model = "gpt-4o")]
//! async fn answer(question: &str) -> Result<String, std::io::Error> {
//!     Ok(format!("You asked: {question}"))
//! }
//!
//! #[observe(name = "chat-turn", user_id = "user-1")]
//! async fn chat(question: String) -> Result<String, std::io::Error> {
//!     answer(&question).await // a child of `chat-turn`
//! }
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
//! langfuse_ergonomic::observe::install_batcher(batcher.clone());
//! chat("What is Langfuse?".into()).await?;
//! batcher.shutdown().await?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # How it works
//!
//! - **Sink.** Finished observations go to the sink installed with [`install`] (a client: one
//!   request per observation) or [`install_batcher`] (queued with [`Batcher::try_add`], the
//!   recommended setup). Without a sink, `#[observe]` and the functions here are cheap no-ops.
//! - **Context.** The current observation lives on a thread-local stack. Synchronous functions
//!   enter it for their body; async functions enter it on every `poll` of their future, so it
//!   follows the task across `.await` on any executor, WebAssembly included, without needing a
//!   tokio runtime. A task spawned from inside an observation does not inherit it on its own: wrap
//!   the future with [`ObserveFutureExt::in_current_observation`].
//! - **Nesting.** An observation started while another is current is its child (same trace, the
//!   parent id set). One started with nothing current is a new trace root.
//! - **Sampling.** [`set_sample_rate`] decides once per trace, at its root; children inherit the
//!   decision. Observations of a trace that is not sampled are not recorded at all.
//! - **Write once.** An observation is sent once, complete, when it ends: one create event with
//!   start and end time, input, output and level (the pattern Langfuse v4 prefers). A trace root
//!   also sends the trace (name, user, session, tags, input and output).
//! - **Always ended.** A drop guard ends the observation on early `return`, `?`, panic (level
//!   `ERROR`, status `panicked`) and cancellation (a future dropped before it finished: level
//!   `WARNING`, status `cancelled`).
//! - **Errors.** A function returning `Result` that returns `Err` gets level `ERROR` and the
//!   error's `Display` (or `Debug`) as status message.
//! - **Capture.** Arguments and return values are captured with `serde::Serialize` when the type
//!   implements it, else with `Debug`, else not at all. Capturing never panics and needs no
//!   dependency of yours beyond this crate. The client's [mask](crate::mask) applies before
//!   anything is sent.
//! - **API versions.** With Langfuse v3 observations are ingestion events; with v4 OpenTelemetry
//!   spans. Ids are generated in OpenTelemetry form (32 and 16 hex digits), which the v4 id
//!   mapping ([`crate::otel`]) keeps as they are, so the ids you read here are the ids Langfuse
//!   stores in both versions.
//!
//! # The attribute
//!
//! `#[observe]` (feature `macros`, on by default) takes these options:
//!
//! | Option | Meaning |
//! |---|---|
//! | `name = "..."` | Observation name (default: the function name) |
//! | `as_type = "..."` | `span` (default), `generation`, `event`, `agent`, `tool`, `chain`, `retriever`, `evaluator`, `embedding`, `guardrail` |
//! | `capture_input = false` | Do not record the arguments |
//! | `capture_output = false` | Do not record the return value |
//! | `skip(a, b)` | Do not record these arguments (`self` is never recorded) |
//! | `err` | On `Err`, also record the error as output (default: output omitted) |
//! | `level = "DEBUG"` | Level unless an error, panic or cancellation sets one |
//! | `model = expr` | Model name (generations) |
//! | `user_id = expr`, `session_id = expr` | Trace user and session |
//! | `tags = expr` | Trace tags (`Vec<String>`, `Vec<&str>`, `[&str; N]`) |
//! | `metadata = expr` | Metadata (any value, captured like arguments) |
//! | `stream` / `stream = "iterator"` | The function returns `impl Stream` / `impl Iterator`: the observation stays open until it is exhausted or dropped, and records the items |
//!
//! Expressions may use the arguments; they are evaluated when the observation starts, and only
//! when it records.
//!
//! # Manual use
//!
//! [`Observation::builder`] starts an observation by hand, [`current`] returns the current one,
//! [`update_current`] changes it and [`score_current`] scores it.

mod context;
mod future;
#[cfg(feature = "otel-context")]
mod otel_context;
mod sink;

#[doc(hidden)]
pub mod __private;

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use langfuse_client_base::models::{IngestionEvent, ObservationLevel};
use serde::Serialize;
use serde_json::Value;

pub use crate::observations::ObservationType;
pub use crate::scores::ScoreValue;
pub use context::EnterGuard;
pub use future::{Instrumented, ObserveFutureExt, ObservedIter, ObservedStream, WithObservation};
pub use sink::Sink;

use crate::ApiVersion;
use crate::{Batcher, LangfuseClient};
use sink::Shared;

/// Install a client as the global sink: each finished observation is sent with its own request.
///
/// Natively the request runs on the current tokio runtime (observations ending outside a
/// runtime are dropped with a warning); on WebAssembly with `spawn_local`. Prefer
/// [`install_batcher`] in services. Returns the previous sink.
pub fn install(client: LangfuseClient) -> Option<Sink> {
    sink::set_global(Some(Sink::Client(client)))
}

/// Install a batcher as the global sink: finished observations are queued with
/// [`Batcher::try_add`], which never blocks (so observations can end in `Drop`).
///
/// [`Batcher`] clones share their queue: keep a clone to [`Batcher::flush`] or
/// [`Batcher::shutdown`] it. Returns the previous sink.
pub fn install_batcher(batcher: Batcher) -> Option<Sink> {
    sink::set_global(Some(Sink::Batcher(batcher)))
}

/// Remove the global sink: new traces are no longer recorded (observations already started keep
/// the sink they started with). Returns it.
pub fn uninstall() -> Option<Sink> {
    sink::set_global(None)
}

/// Whether a global sink is installed.
pub fn is_installed() -> bool {
    sink::global().is_some()
}

/// Flush the global sink if it is a batcher (does not shut it down). A no-op otherwise.
pub async fn flush() -> crate::Result<()> {
    if let Some(sink) = sink::global() {
        if let Sink::Batcher(batcher) = &*sink {
            batcher.flush().await?;
        }
    }
    Ok(())
}

/// The fraction of new traces to record, between 0 and 1 (default 1). Decided once per trace at
/// its root; observations inside a trace follow the root.
pub fn set_sample_rate(rate: f64) {
    sink::set_sample_rate(rate);
}

/// The current sample rate (see [`set_sample_rate`]).
pub fn sample_rate() -> f64 {
    sink::sample_rate()
}

/// The current observation, if one is recording.
pub fn current() -> Option<Observation> {
    context::top().filter(Observation::is_recording)
}

/// Run `f` with the current observation, if one is recording. Returns whether it ran.
///
/// ```
/// use langfuse_ergonomic::observe;
///
/// observe::update_current(|observation| {
///     observation.set_metadata(serde_json::json!({ "cache": "hit" }));
/// });
/// ```
pub fn update_current(f: impl FnOnce(&Observation)) -> bool {
    match current() {
        Some(observation) => {
            f(&observation);
            true
        }
        None => false,
    }
}

/// Score the current observation (see [`Observation::score`]). Returns whether there was one.
pub fn score_current(name: impl Into<String>, value: impl Into<ScoreValue>) -> bool {
    update_current(|observation| observation.score(name, value))
}

/// Score the trace of the current observation (see [`Observation::score_trace`]).
pub fn score_current_trace(name: impl Into<String>, value: impl Into<ScoreValue>) -> bool {
    update_current(|observation| observation.score_trace(name, value))
}

/// A Langfuse trace id in OpenTelemetry form: 32 lowercase hex digits.
fn new_trace_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// A Langfuse observation id in OpenTelemetry form: 16 lowercase hex digits, not all zero.
fn new_observation_id() -> String {
    loop {
        let mut id = uuid::Uuid::new_v4().simple().to_string();
        id.truncate(16);
        if id.bytes().any(|b| b != b'0') {
            return id;
        }
    }
}

/// Trace-level attributes, shared by the observations of one trace in this process.
#[derive(Debug, Default, Clone)]
struct TraceAttributes {
    name: Option<String>,
    user_id: Option<String>,
    session_id: Option<String>,
    tags: Option<Vec<String>>,
    metadata: Option<Value>,
    release: Option<String>,
    version: Option<String>,
    public: Option<bool>,
}

#[derive(Debug, Default)]
struct ObservationData {
    name: String,
    input: Option<Value>,
    output: Option<Value>,
    metadata: Option<Value>,
    level: Option<ObservationLevel>,
    status_message: Option<String>,
    version: Option<String>,
    environment: Option<String>,
    model: Option<String>,
    model_parameters: Option<Value>,
    usage_details: Option<HashMap<String, i64>>,
    cost_details: Option<HashMap<String, f64>>,
    completion_start_time: Option<DateTime<Utc>>,
    prompt_name: Option<String>,
    prompt_version: Option<i64>,
    panicked: bool,
    ended: bool,
}

struct Inner {
    sink: Shared<Sink>,
    trace: Shared<Mutex<TraceAttributes>>,
    trace_id: String,
    id: String,
    parent_id: Option<String>,
    kind: ObservationType,
    /// Whether this observation sends the trace (it started the trace in this process).
    trace_root: bool,
    start_time: DateTime<Utc>,
    data: Mutex<ObservationData>,
}

/// How an observation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndReason {
    Completed,
    Cancelled,
    Panicked,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Serialise a value, lossily: `None` when it cannot be serialised (or its `Serialize` panics).
fn to_value<T: Serialize + ?Sized>(value: &T) -> Option<Value> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| serde_json::to_value(value)))
        .ok()
        .and_then(Result::ok)
}

/// A handle to an observation (span, generation, agent, ...), cheap to clone.
///
/// Every method is a no-op on an observation that does not record (its trace was not sampled).
/// An observation is sent once, when it ends: explicitly with [`Observation::end`], or by the
/// guard of `#[observe]`, [`Observation::instrument`] or [`Observation::enter_scope`].
#[derive(Clone)]
pub struct Observation {
    inner: Option<Shared<Inner>>,
}

impl std::fmt::Debug for Observation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            Some(inner) => f
                .debug_struct("Observation")
                .field("trace_id", &inner.trace_id)
                .field("id", &inner.id)
                .field("parent_id", &inner.parent_id)
                .field("kind", &inner.kind)
                .finish_non_exhaustive(),
            None => f.write_str("Observation(not recording)"),
        }
    }
}

impl Observation {
    /// An observation that records nothing (a trace that was not sampled).
    pub(crate) fn disabled() -> Self {
        Self { inner: None }
    }

    /// Start building an observation by hand.
    pub fn builder(name: impl Into<String>) -> ObservationBuilder {
        ObservationBuilder::new(name.into())
    }

    /// Whether this observation records (and will be sent).
    pub fn is_recording(&self) -> bool {
        self.inner.is_some()
    }

    /// The trace id (32 hex digits), if recording.
    pub fn trace_id(&self) -> Option<&str> {
        self.inner.as_ref().map(|inner| inner.trace_id.as_str())
    }

    /// The observation id (16 hex digits), if recording.
    pub fn id(&self) -> Option<&str> {
        self.inner.as_ref().map(|inner| inner.id.as_str())
    }

    /// The parent observation id, if recording and not a root.
    pub fn parent_id(&self) -> Option<&str> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.parent_id.as_deref())
    }

    /// The observation type, if recording.
    pub fn kind(&self) -> Option<ObservationType> {
        self.inner.as_ref().map(|inner| inner.kind)
    }

    /// The start time, if recording.
    pub fn start_time(&self) -> Option<DateTime<Utc>> {
        self.inner.as_ref().map(|inner| inner.start_time)
    }

    /// The URL of this observation's trace in the Langfuse UI, if recording.
    pub fn trace_url(&self) -> Option<String> {
        let inner = self.inner.as_ref()?;
        Some(format!(
            "{}/trace/{}",
            inner.sink.client().web_base_url(),
            inner.trace_id
        ))
    }

    /// The sink this observation (and its children) send to.
    pub(crate) fn sink(&self) -> Option<&Shared<Sink>> {
        self.inner.as_ref().map(|inner| &inner.sink)
    }

    fn with_data(&self, f: impl FnOnce(&mut ObservationData)) {
        if let Some(inner) = &self.inner {
            let mut data = lock(&inner.data);
            if !data.ended {
                f(&mut data);
            }
        }
    }

    fn with_trace(&self, f: impl FnOnce(&mut TraceAttributes)) {
        let Some(inner) = &self.inner else { return };
        let attributes = {
            let mut attributes = lock(&inner.trace);
            f(&mut attributes);
            attributes.clone()
        };
        inner.remember_trace_attributes(&attributes);
    }

    /// Rename the observation.
    pub fn set_name(&self, name: impl Into<String>) {
        let name = name.into();
        self.with_data(|data| data.name = name);
    }

    /// Set the input (any `Serialize` value; one that fails to serialise is ignored).
    pub fn set_input(&self, input: impl Serialize) {
        self.set_input_value(to_value(&input));
    }

    /// Set the output (any `Serialize` value; one that fails to serialise is ignored).
    pub fn set_output(&self, output: impl Serialize) {
        self.set_output_value(to_value(&output));
    }

    pub(crate) fn set_input_value(&self, input: Option<Value>) {
        if input.is_some() {
            self.with_data(|data| data.input = input);
        }
    }

    pub(crate) fn set_output_value(&self, output: Option<Value>) {
        if output.is_some() {
            self.with_data(|data| data.output = output);
        }
    }

    /// Set the metadata, replacing what was set.
    pub fn set_metadata(&self, metadata: impl Serialize) {
        let metadata = to_value(&metadata);
        if metadata.is_some() {
            self.with_data(|data| data.metadata = metadata);
        }
    }

    /// Add one metadata key (the metadata becomes an object if it was something else).
    pub fn add_metadata(&self, key: impl Into<String>, value: impl Serialize) {
        let key = key.into();
        let Some(value) = to_value(&value) else {
            return;
        };
        self.with_data(|data| {
            let mut map = match data.metadata.take() {
                Some(Value::Object(map)) => map,
                Some(other) => serde_json::Map::from_iter([("value".to_owned(), other)]),
                None => serde_json::Map::new(),
            };
            map.insert(key, value);
            data.metadata = Some(Value::Object(map));
        });
    }

    /// Set the level (an error, panic or cancellation overrides `DEFAULT`/`DEBUG`).
    pub fn set_level(&self, level: ObservationLevel) {
        self.with_data(|data| data.level = Some(level));
    }

    /// Set the status message.
    pub fn set_status_message(&self, message: impl Into<String>) {
        let message = message.into();
        self.with_data(|data| data.status_message = Some(message));
    }

    /// Mark the observation as failed: level `ERROR` and `message` as status message.
    pub fn set_error(&self, message: impl Into<String>) {
        let message = message.into();
        self.with_data(|data| {
            data.level = Some(ObservationLevel::Error);
            data.status_message = Some(message);
        });
    }

    /// Set the observation's version.
    pub fn set_version(&self, version: impl Into<String>) {
        let version = version.into();
        self.with_data(|data| data.version = Some(version));
    }

    /// Set the environment (for example `production`).
    pub fn set_environment(&self, environment: impl Into<String>) {
        let environment = environment.into();
        self.with_data(|data| data.environment = Some(environment));
    }

    /// Set the model name (generations, embeddings).
    pub fn set_model(&self, model: impl Into<String>) {
        let model = model.into();
        self.with_data(|data| data.model = Some(model));
    }

    /// Set the model parameters (a JSON object, for example `{"temperature": 0.2}`).
    pub fn set_model_parameters(&self, parameters: impl Serialize) {
        let parameters = to_value(&parameters);
        self.with_data(|data| data.model_parameters = parameters);
    }

    /// Set the token counts by usage type (`input`, `output`, `total`, ...).
    pub fn set_usage_details<K: Into<String>>(&self, usage: impl IntoIterator<Item = (K, i64)>) {
        let usage: HashMap<String, i64> = usage.into_iter().map(|(k, v)| (k.into(), v)).collect();
        self.with_data(|data| data.usage_details = Some(usage));
    }

    /// Set the costs in USD by usage type (`input`, `output`, `total`, ...).
    pub fn set_cost_details<K: Into<String>>(&self, cost: impl IntoIterator<Item = (K, f64)>) {
        let cost: HashMap<String, f64> = cost.into_iter().map(|(k, v)| (k.into(), v)).collect();
        self.with_data(|data| data.cost_details = Some(cost));
    }

    /// Set when the first token arrived (time to first token).
    pub fn set_completion_start_time(&self, time: DateTime<Utc>) {
        self.with_data(|data| data.completion_start_time = Some(time));
    }

    /// Link the Langfuse prompt that produced this generation. Fallback prompts are not linked.
    pub fn set_prompt(&self, prompt: &crate::prompts::FetchedPrompt) {
        if let Some(version) = prompt.version() {
            let name = prompt.name().to_owned();
            self.with_data(|data| {
                data.prompt_name = Some(name);
                data.prompt_version = Some(version);
            });
        }
    }

    /// Set the trace name (default: the root observation's name).
    pub fn set_trace_name(&self, name: impl Into<String>) {
        let name = name.into();
        self.with_trace(|trace| trace.name = Some(name));
    }

    /// Set the trace's user id.
    pub fn set_user_id(&self, user_id: impl Into<String>) {
        let user_id = user_id.into();
        self.with_trace(|trace| trace.user_id = Some(user_id));
    }

    /// Set the trace's session id.
    pub fn set_session_id(&self, session_id: impl Into<String>) {
        let session_id = session_id.into();
        self.with_trace(|trace| trace.session_id = Some(session_id));
    }

    /// Set the trace's tags.
    pub fn set_tags(&self, tags: impl crate::traces::IntoTags) {
        let tags = tags.into_tags();
        self.with_trace(|trace| trace.tags = Some(tags));
    }

    /// Set the trace's metadata.
    pub fn set_trace_metadata(&self, metadata: impl Serialize) {
        let metadata = to_value(&metadata);
        self.with_trace(|trace| trace.metadata = metadata);
    }

    /// Set the trace's release.
    pub fn set_release(&self, release: impl Into<String>) {
        let release = release.into();
        self.with_trace(|trace| trace.release = Some(release));
    }

    /// Make the trace public (shareable by link) or not.
    pub fn set_public(&self, public: bool) {
        self.with_trace(|trace| trace.public = Some(public));
    }

    /// Score this observation (sent at once through the sink).
    pub fn score(&self, name: impl Into<String>, value: impl Into<ScoreValue>) {
        self.score_with(name, value, None::<String>);
    }

    /// Score this observation, with a comment.
    pub fn score_with(
        &self,
        name: impl Into<String>,
        value: impl Into<ScoreValue>,
        comment: Option<impl Into<String>>,
    ) {
        let Some(inner) = &self.inner else { return };
        inner.sink.send(vec![crate::scores::score_event(
            crate::scores::ScoreTarget {
                trace_id: Some(inner.trace_id.clone()),
                observation_id: Some(inner.id.clone()),
                ..Default::default()
            },
            name.into(),
            &value.into(),
            comment.map(Into::into),
            None,
        )]);
    }

    /// Score this observation's trace.
    pub fn score_trace(&self, name: impl Into<String>, value: impl Into<ScoreValue>) {
        self.score_trace_with(name, value, None::<String>);
    }

    /// Score this observation's trace, with a comment.
    pub fn score_trace_with(
        &self,
        name: impl Into<String>,
        value: impl Into<ScoreValue>,
        comment: Option<impl Into<String>>,
    ) {
        let Some(inner) = &self.inner else { return };
        inner.sink.send(vec![crate::scores::score_event(
            crate::scores::ScoreTarget {
                trace_id: Some(inner.trace_id.clone()),
                ..Default::default()
            },
            name.into(),
            &value.into(),
            comment.map(Into::into),
            None,
        )]);
    }

    /// Start a child observation of this one.
    pub fn child(&self, name: impl Into<String>, kind: ObservationType) -> Observation {
        match &self.inner {
            None => Observation::disabled(),
            Some(parent) => Observation::start_child_of(parent, name.into(), kind, None),
        }
    }

    /// Make this observation the current one until the guard is dropped (it does not end it).
    pub fn enter(&self) -> EnterGuard {
        context::enter(self)
    }

    /// Make this observation current for a synchronous scope and end it when the returned
    /// guard is dropped (also on panic, which sets level `ERROR`).
    pub fn enter_scope(self) -> ScopeGuard {
        ScopeGuard::new(Some(self))
    }

    /// Run a future with this observation current on every poll, and end the observation when
    /// the future completes, is dropped before (cancelled) or panics.
    pub fn instrument<F: std::future::Future>(self, future: F) -> Instrumented<F> {
        Instrumented::new(Some(self), future, future::NoRecord, true)
    }

    /// End the observation now and send it (later calls do nothing).
    pub fn end(&self) {
        self.end_with(EndReason::Completed);
    }

    /// End the observation with an output.
    pub fn end_with_output(&self, output: impl Serialize) {
        self.set_output(output);
        self.end();
    }

    pub(crate) fn mark_panicked(&self) {
        self.with_data(|data| data.panicked = true);
    }

    pub(crate) fn end_with(&self, reason: EndReason) {
        let Some(inner) = &self.inner else { return };
        let events = {
            let mut data = lock(&inner.data);
            if data.ended {
                return;
            }
            data.ended = true;
            let reason = if data.panicked {
                EndReason::Panicked
            } else {
                reason
            };
            match reason {
                EndReason::Completed => {}
                EndReason::Panicked => {
                    data.level = Some(ObservationLevel::Error);
                    data.status_message = Some("panicked".to_owned());
                }
                EndReason::Cancelled => {
                    if data.level != Some(ObservationLevel::Error) {
                        data.level = Some(ObservationLevel::Warning);
                        data.status_message = Some("cancelled".to_owned());
                    }
                }
            }
            inner.events(&mut data, crate::time::now_utc())
        };
        inner.sink.send(events);
    }

    /// Start an observation: a child of the current one, or a new trace with the global sink.
    /// `None` when there is nothing to record into (no sink, nothing current).
    pub(crate) fn start(name: String, kind: ObservationType) -> Option<Observation> {
        match context::top() {
            Some(parent) => Some(match &parent.inner {
                None => Observation::disabled(),
                Some(parent) => Observation::start_child_of(parent, name, kind, None),
            }),
            None => {
                let sink = sink::global()?;
                Some(Observation::start_root(
                    sink,
                    name,
                    kind,
                    RootOptions::default(),
                    sink::sample(),
                ))
            }
        }
    }

    fn start_child_of(
        parent: &Shared<Inner>,
        name: String,
        kind: ObservationType,
        id: Option<String>,
    ) -> Observation {
        Observation {
            inner: Some(Shared::new(Inner {
                sink: parent.sink.clone(),
                trace: parent.trace.clone(),
                trace_id: parent.trace_id.clone(),
                id: id.unwrap_or_else(new_observation_id),
                parent_id: Some(parent.id.clone()),
                kind,
                trace_root: false,
                start_time: crate::time::now_utc(),
                data: Mutex::new(ObservationData {
                    name,
                    ..ObservationData::default()
                }),
            })),
        }
    }

    fn start_root(
        sink: Shared<Sink>,
        name: String,
        kind: ObservationType,
        options: RootOptions,
        sampled: bool,
    ) -> Observation {
        #[cfg_attr(not(feature = "otel-context"), allow(unused_mut))]
        let mut trace_id = options.trace_id;
        #[cfg_attr(not(feature = "otel-context"), allow(unused_mut))]
        let mut parent_id = options.parent_observation_id;
        #[cfg_attr(not(feature = "otel-context"), allow(unused_mut))]
        let mut sampled = sampled;
        #[cfg(feature = "otel-context")]
        if trace_id.is_none() && sink.client().api_version() == ApiVersion::V4 {
            if let Some(remote) = otel_context::current_span() {
                trace_id = Some(remote.trace_id);
                parent_id = Some(remote.span_id);
                sampled = remote.sampled;
            }
        }
        if !sampled {
            return Observation::disabled();
        }
        let api_version = sink.client().api_version();
        let map_trace = |id: String| match api_version {
            ApiVersion::V3 => id,
            ApiVersion::V4 => crate::otel::trace_id(&id),
        };
        let map_observation = |id: String| match api_version {
            ApiVersion::V3 => id,
            ApiVersion::V4 => crate::otel::observation_id(&id),
        };
        let trace_root = parent_id.is_none();
        let inner = Inner {
            sink,
            trace: Shared::new(Mutex::new(TraceAttributes::default())),
            trace_id: map_trace(trace_id.unwrap_or_else(new_trace_id)),
            id: map_observation(options.id.unwrap_or_else(new_observation_id)),
            parent_id: parent_id.map(map_observation),
            kind,
            trace_root,
            start_time: crate::time::now_utc(),
            data: Mutex::new(ObservationData {
                name,
                ..ObservationData::default()
            }),
        };
        Observation {
            inner: Some(Shared::new(inner)),
        }
    }
}

impl Inner {
    /// V4 stamps the trace attributes on every span written after them: record them now, so
    /// that children (which end, and are written, before the root) carry them too.
    fn remember_trace_attributes(&self, attributes: &TraceAttributes) {
        let client = self.sink.client();
        if client.api_version() == ApiVersion::V4 {
            client.v4_state().remember_trace_attributes(
                &self.trace_id,
                attributes.user_id.as_deref(),
                attributes.session_id.as_deref(),
                attributes.tags.as_deref(),
            );
        }
    }

    /// The events of the ended observation: its create event, and for a trace root the trace.
    fn events(&self, data: &mut ObservationData, end_time: DateTime<Utc>) -> Vec<IngestionEvent> {
        use crate::observations::{observation_event, ObservationFields};

        let mut fields = ObservationFields::new(self.kind, self.id.clone(), self.trace_id.clone());
        fields.parent_observation_id = self.parent_id.clone();
        fields.name = Some(data.name.clone());
        fields.start_time = self.start_time;
        fields.end_time = Some(end_time.max(self.start_time));
        fields.completion_start_time = data.completion_start_time;
        fields.input = data.input.take();
        fields.output = data.output.take();
        fields.metadata = data.metadata.take();
        fields.level = data.level;
        fields.status_message = data.status_message.take();
        fields.version = data.version.take();
        fields.environment = data.environment.take();
        fields.model = data.model.take();
        fields.model_parameters = data.model_parameters.take();
        fields.usage_details = data.usage_details.take();
        fields.cost_details = data.cost_details.take();
        fields.prompt_name = data.prompt_name.take();
        fields.prompt_version = data.prompt_version.take();

        let trace_event = self.trace_root.then(|| {
            use langfuse_client_base::models::{
                ingestion_event_trace_event::Type, IngestionEventTraceEvent, TraceBody,
            };
            let attributes = lock(&self.trace).clone();
            let body = TraceBody {
                id: Some(Some(self.trace_id.clone())),
                timestamp: Some(Some(self.start_time.fixed_offset())),
                name: Some(Some(attributes.name.unwrap_or_else(|| data.name.clone()))),
                user_id: attributes.user_id.map(Some),
                session_id: attributes.session_id.map(Some),
                tags: attributes.tags.map(Some),
                metadata: attributes.metadata.map(Some),
                release: attributes.release.map(Some),
                version: attributes.version.map(Some),
                public: attributes.public.map(Some),
                input: fields.input.clone().map(Some),
                output: fields.output.clone().map(Some),
                environment: fields.environment.clone().map(Some),
            };
            IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
                uuid::Uuid::new_v4().to_string(),
                self.start_time
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                body,
                Type::TraceCreate,
            ))
        });
        // The observation first: with V4 the trace event then adds the trace attributes to the
        // root span instead of creating a second root.
        let mut events = vec![observation_event(fields)];
        events.extend(trace_event);
        events
    }
}

#[derive(Debug, Default)]
struct RootOptions {
    trace_id: Option<String>,
    parent_observation_id: Option<String>,
    id: Option<String>,
}

/// Builds an observation by hand ([`Observation::builder`]).
#[derive(Debug)]
pub struct ObservationBuilder {
    name: String,
    kind: ObservationType,
    sink: Option<Sink>,
    options: RootOptions,
    detached: bool,
    input: Option<Value>,
    metadata: Option<Value>,
}

impl ObservationBuilder {
    fn new(name: String) -> Self {
        Self {
            name,
            kind: ObservationType::Span,
            sink: None,
            options: RootOptions::default(),
            detached: false,
            input: None,
            metadata: None,
        }
    }

    /// The observation type (default span).
    #[must_use]
    pub fn as_type(mut self, kind: ObservationType) -> Self {
        self.kind = kind;
        self
    }

    /// Send to this client instead of the current observation's or the global sink. Starts a new
    /// trace (always recorded, whatever the sample rate) unless [`Self::trace_id`] and
    /// [`Self::parent_observation_id`] place it in an existing one.
    #[must_use]
    pub fn client(mut self, client: LangfuseClient) -> Self {
        self.sink = Some(Sink::Client(client));
        self
    }

    /// Like [`Self::client`], sending through a batcher.
    #[must_use]
    pub fn batcher(mut self, batcher: Batcher) -> Self {
        self.sink = Some(Sink::Batcher(batcher));
        self
    }

    /// Use this trace id for a new root (mapped like the builders' ids with Langfuse v4).
    #[must_use]
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.options.trace_id = Some(trace_id.into());
        self.detached = true;
        self
    }

    /// Attach the new observation under an observation written elsewhere (with
    /// [`Self::trace_id`]). It then does not send the trace itself.
    #[must_use]
    pub fn parent_observation_id(mut self, id: impl Into<String>) -> Self {
        self.options.parent_observation_id = Some(id.into());
        self.detached = true;
        self
    }

    /// Use this observation id.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.options.id = Some(id.into());
        self
    }

    /// Start a new trace even when an observation is current.
    #[must_use]
    pub fn new_trace(mut self) -> Self {
        self.detached = true;
        self
    }

    /// The input.
    #[must_use]
    pub fn input(mut self, input: impl Serialize) -> Self {
        self.input = to_value(&input);
        self
    }

    /// The metadata.
    #[must_use]
    pub fn metadata(mut self, metadata: impl Serialize) -> Self {
        self.metadata = to_value(&metadata);
        self
    }

    /// Start it: a child of the current observation (unless a client, batcher, trace id or
    /// [`Self::new_trace`] was given), else a new trace on the given or global sink. Without a
    /// sink, or when the trace is not sampled, the observation does not record.
    pub fn start(self) -> Observation {
        let observation = match (self.sink, self.detached) {
            (Some(sink), _) => {
                Observation::start_root(Shared::new(sink), self.name, self.kind, self.options, true)
            }
            (None, false) => {
                if let Some(id) = self.options.id {
                    match context::top() {
                        Some(Observation {
                            inner: Some(parent),
                        }) => Observation::start_child_of(&parent, self.name, self.kind, Some(id)),
                        Some(_) => Observation::disabled(),
                        None => match sink::global() {
                            Some(sink) => Observation::start_root(
                                sink,
                                self.name,
                                self.kind,
                                RootOptions {
                                    id: Some(id),
                                    ..RootOptions::default()
                                },
                                sink::sample(),
                            ),
                            None => Observation::disabled(),
                        },
                    }
                } else {
                    Observation::start(self.name, self.kind).unwrap_or_else(Observation::disabled)
                }
            }
            (None, true) => match context::top()
                .and_then(|top| top.sink().cloned())
                .or_else(sink::global)
            {
                Some(sink) => Observation::start_root(
                    sink,
                    self.name,
                    self.kind,
                    self.options,
                    sink::sample(),
                ),
                None => Observation::disabled(),
            },
        };
        observation.set_input_value(self.input);
        if let Some(metadata) = self.metadata {
            observation.with_data(|data| data.metadata = Some(metadata));
        }
        observation
    }
}

/// Ends an observation when dropped ([`Observation::enter_scope`], and synchronous `#[observe]`
/// functions); while it lives, the observation is current.
#[must_use = "the observation ends when the guard is dropped"]
pub struct ScopeGuard {
    observation: Option<Observation>,
    _enter: Option<EnterGuard>,
}

impl std::fmt::Debug for ScopeGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopeGuard")
            .field("observation", &self.observation)
            .finish()
    }
}

impl ScopeGuard {
    pub(crate) fn new(observation: Option<Observation>) -> Self {
        let enter = observation.as_ref().map(context::enter);
        Self {
            observation,
            _enter: enter,
        }
    }

    /// The observation, if any.
    pub fn observation(&self) -> Option<&Observation> {
        self.observation.as_ref()
    }
}

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        if let Some(observation) = &self.observation {
            if std::thread::panicking() {
                observation.mark_panicked();
            }
            observation.end_with(EndReason::Completed);
        }
    }
}
