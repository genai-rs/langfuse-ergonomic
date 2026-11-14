# Observe: automatic tracing

`#[observe]` turns a function into a Langfuse observation. Calls made inside it become its
children, so a request handler, its agent loop, its tool calls and its LLM calls form one
trace without passing ids around. It works on `async fn` and on synchronous functions, on any
executor, and on WebAssembly.

```rust,no_run
use langfuse_ergonomic::{observe, ClientBuilder};

#[observe(as_type = "retriever")]
async fn search(query: &str) -> Vec<String> {
    vec![format!("doc about {query}")]
}

#[observe(as_type = "generation", model = "gpt-4o")]
async fn answer(question: &str, documents: &[String]) -> Result<String, std::io::Error> {
    // Usage, costs and the like can be added to the current observation.
    observe::update_current(|o| o.set_usage_details([("input", 120), ("output", 40)]));
    Ok(format!("{question}: {}", documents.join(", ")))
}

#[observe(name = "support-request", user_id = user.as_str(), tags = ["support"])]
async fn handle(user: String, question: String) -> Result<String, std::io::Error> {
    let documents = search(&question).await;
    let reply = answer(&question, &documents).await?;
    observe::score_current("answered", true);
    Ok(reply)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
    observe::install_batcher(batcher.clone());

    handle("user-1".into(), "How do I reset my password?".into()).await?;

    batcher.shutdown().await?;
    Ok(())
}
```

This writes one trace, `support-request`, with a retriever and a generation as children; the
trace has the user id and tag, the root's input (`{"user": .., "question": ..}`) and output,
and a score.

## Setup

Observations go to a global **sink**:

- `observe::install_batcher(batcher)`: queue them on a [`Batcher`](batching.md) (recommended).
  Observations end with `Batcher::try_add`, which never waits, so they can end in `Drop`.
  `Batcher` clones share their queue: keep one to `flush()` or `shutdown()` before the process
  exits.
- `observe::install(client)`: send each observation with its own request, in a background
  task of the current tokio runtime (or `spawn_local` on WebAssembly). Fine for scripts and
  tests.

Without a sink, `#[observe]` is a cheap no-op: the function runs, nothing is recorded, and
`observe::current()` returns `None`. `observe::uninstall()` removes the sink.

`observe::set_sample_rate(0.1)` records one trace in ten. The decision is made once per
trace, at its root; every observation in the trace follows it.

## Options

| Option | Meaning |
|---|---|
| `name = "..."` | Observation name (default: the function name) |
| `as_type = "..."` | `span` (default), `generation`, `event`, `agent`, `tool`, `chain`, `retriever`, `evaluator`, `embedding`, `guardrail` |
| `capture_input = false` | Do not record the arguments |
| `capture_output = false` | Do not record the return value |
| `skip(a, b)` | Do not record these arguments (`self` is never recorded) |
| `err` | On `Err`, also record the error as the output (default: no output) |
| `level = "DEBUG"` | The level, unless an error, panic or cancellation sets one |
| `model = expr` | Model name (generations, embeddings) |
| `user_id = expr`, `session_id = expr` | Trace user and session |
| `tags = expr` | Trace tags (`Vec<String>`, `Vec<&str>`, `[&str; N]`) |
| `metadata = expr` | Metadata, any value (captured like the arguments) |
| `stream` | The function returns `impl Stream`: the observation stays open until the stream ends |
| `stream = "iterator"` | The same for `impl Iterator` |

Expressions may use the arguments. They are evaluated when the observation starts, and only
when it records. Misspelled options, unknown types or levels and `skip` of a missing argument
are compile errors.

## What is recorded

- **Input**: the arguments as a JSON object keyed by name. Each is captured with
  `serde::Serialize` when its type implements it, else with `Debug` (as a string), else left
  out. Capturing never panics, and needs no dependency of yours beyond this crate.
- **Output**: the return value, captured the same way. For a `Result`, the `Ok` value.
- **Errors**: when a function returns `Err`, the observation gets level `ERROR` and the
  error's `Display` (else `Debug`) as status message. Its output is left out, unless `err`.
- **Panics**: a panic ends the observation with level `ERROR` and status `panicked`, then
  unwinds as before.
- **Cancellation**: an `async fn` whose future is dropped before it finished (a `select!`
  branch that lost, a timeout, an aborted task) ends with level `WARNING` and status
  `cancelled`.
- **Early return and `?`** end the observation like a normal return.
- **Streams** (`stream`): the observation records the items as an array, the time of the
  first item as completion start (generations), and an `Err` item as an error; it ends when
  the stream ends, or with `cancelled` when it is dropped first.
- **Timing**: an observation is sent once, complete, when it ends: one create event with
  start and end time, input, output and level. A trace root also sends the trace (name, user,
  session, tags, input, output). The [mask](masking.md) applies before anything is sent.

## Context and nesting

The current observation lives on a thread-local stack. A synchronous function enters it for
its body; an `async fn` enters it on every poll of its future (like `tracing::Instrumented`),
so it follows the task across `.await` and threads of a multi-threaded runtime.

A task spawned from inside an observation does not inherit it on its own. Wrap the future:

```rust,no_run
use langfuse_ergonomic::observe;
use langfuse_ergonomic::observe::ObserveFutureExt;

#[observe(as_type = "tool")]
async fn fetch(url: String) -> usize {
    url.len()
}

#[observe]
async fn crawl(urls: Vec<String>) -> usize {
    let tasks: Vec<_> = urls
        .into_iter()
        .map(|url| tokio::spawn(fetch(url).in_current_observation()))
        .collect();
    let mut total = 0;
    for task in tasks {
        total += task.await.unwrap_or_default();
    }
    total
}
```

`with_observation(observation)` does the same with a given observation.

## Manual observations and the current observation

```rust,no_run
use langfuse_ergonomic::observe::{self, Observation, ObservationType};

fn example() {
    // A child of the current observation (or a new trace on the global sink).
    let step = Observation::builder("rerank")
        .as_type(ObservationType::Chain)
        .input(["a", "b"])
        .start();
    step.add_metadata("model", "rerank-v3");
    step.end_with_output(["b", "a"]);

    // Change, read or score whatever is current.
    observe::update_current(|o| o.set_trace_name("nightly-batch"));
    if let Some(current) = observe::current() {
        println!("in trace {:?}: {:?}", current.trace_id(), current.trace_url());
    }
    observe::score_current_trace("quality", 0.8);
}
```

`Observation` also has `enter()` (make it current), `enter_scope()` (current, and ended when
the guard drops), `instrument(future)` (current while the future runs, ended when it
completes), `child(..)`, and setters for every field: model, usage and cost details, prompt
link (`set_prompt(&fetched_prompt)`), level, status, user, session, tags and more.

`Observation::builder(..).client(client)` or `.batcher(batcher)` starts a new trace on that
sink regardless of the global one (and always records); `.trace_id(..)` and
`.parent_observation_id(..)` place it in a trace written elsewhere.

## Langfuse v3 and v4

Both work. With v3, observations are `*-create` ingestion events and the root sends a
`trace-create`. With v4 they are OpenTelemetry spans: children name their parent span, the
root is the trace's root span (the trace event adds the trace attributes to it), and the user,
session and tags are stamped on every span of the trace. Ids are generated in OpenTelemetry
form (32 and 16 hex digits), which the [v4 id mapping](api-versions.md#ids-in-v4) keeps as
they are, so `observation.trace_id()` is the id Langfuse stores in both versions.

## OpenTelemetry and openai-ergonomic

With the `otel-context` feature (OpenTelemetry 0.33, the version openai-ergonomic and
opentelemetry-langfuse use), observations take part in OpenTelemetry context propagation:

- While an observation is current, the current OpenTelemetry `Context` holds a remote span
  context with the observation's trace and span id. Spans started from `Context::current()`
  become its children: [openai-ergonomic](https://github.com/genai-rs/openai-ergonomic)'s
  `LangfuseInterceptor`, [opentelemetry-langfuse](https://github.com/genai-rs/opentelemetry-langfuse)
  and any other OpenTelemetry instrumentation. An unsampled trace is propagated as unsampled,
  so a parent-based sampler drops those spans too.
- A root observation started inside an active OpenTelemetry span joins that span's trace as
  its child, and follows its sampling decision.

This needs Langfuse v4 (both sides then write OpenTelemetry ids into the same trace). With a
v3 sink nothing is attached: Langfuse v3 ingestion and OpenTelemetry spans do not share ids.

```toml
[dependencies]
langfuse-ergonomic = { version = "0.x", features = ["otel-context"] }  # see the crates.io badge
openai-ergonomic = "0.x"   # its default `otel` feature has the Langfuse interceptor
opentelemetry = { version = "0.x", features = ["trace"] }
opentelemetry_sdk = { version = "0.x", features = ["rt-tokio", "trace", "experimental_trace_batch_span_processor_with_async_runtime"] }
tokio = { version = "1", features = ["full"] }
```

```rust,ignore
use langfuse_ergonomic::{observe, ClientBuilder};
use openai_ergonomic::{
    langfuse_exporter_from_env, Client, LangfuseConfig, LangfuseInterceptor, LangfuseState,
};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::span_processor_with_async_runtime::BatchSpanProcessor;
use opentelemetry_sdk::trace::{SdkTracerProvider, Span};

type OpenAi = Client<LangfuseState<Span>>;

#[observe(as_type = "agent")]
async fn agent(openai: &OpenAi, question: &str) -> Result<String, Box<dyn std::error::Error>> {
    // The interceptor's span for this call is a child of the `agent` observation.
    let response = openai.send_chat(openai.chat_simple(question)).await?;
    Ok(response.content().unwrap_or_default().to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // OpenAI calls: OpenTelemetry spans exported to Langfuse by openai-ergonomic.
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(langfuse_exporter_from_env()?, Tokio).build())
        .build();
    let interceptor = LangfuseInterceptor::new(provider.tracer("openai-ergonomic"), LangfuseConfig::new());
    let openai = Client::from_env()?.with_interceptor(interceptor).build();

    // Your own code: #[observe] observations, sent by langfuse-ergonomic (Langfuse v4).
    let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
    observe::install_batcher(batcher.clone());

    agent(&openai, "What is the capital of Belgium?").await?;

    batcher.shutdown().await?;
    // The Tokio batch processor needs the runtime while shutdown waits for export.
    tokio::task::spawn_blocking(move || provider.shutdown()).await??;
    Ok(())
}
```

`examples/observe_otel.rs` shows the nesting with an in-memory OpenTelemetry exporter, and
`tests/observe_otel_test.rs` checks the ids.

## WebAssembly

Everything above works in a JavaScript host: the context is entered on every poll, so no
runtime is involved, nothing needs to be `Send`, and `in_current_observation()` carries an
observation into `wasm_bindgen_futures::spawn_local`. See [WebAssembly](webassembly.md).

## Limits

- Synchronous functions run their body in a closure (to see the return value), as
  `tracing`'s `#[instrument(ret)]` does: a body that returns a borrow of a `&mut` argument
  does not compile. Use `Observation::builder` by hand there.
- A function that returns a future without being `async fn` is observed only while it builds
  the future; make it `async fn`, or wrap the future with `Observation::instrument`.
- The expansion refers to `::langfuse_ergonomic`: the crate cannot be renamed in
  `Cargo.toml`.
