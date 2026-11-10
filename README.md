# langfuse-ergonomic

[![Crates.io](https://img.shields.io/crates/v/langfuse-ergonomic.svg)](https://crates.io/crates/langfuse-ergonomic)
[![Documentation](https://docs.rs/langfuse-ergonomic/badge.svg)](https://docs.rs/langfuse-ergonomic)
[![CI](https://github.com/genai-rs/langfuse-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/langfuse-ergonomic/actions)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](https://www.rust-lang.org/)
[![License](https://img.shields.io/crates/l/langfuse-ergonomic)](./LICENSE-MIT)

Ergonomic Rust client for [Langfuse](https://langfuse.com), the open-source LLM observability platform.

## Features

- **Builder pattern** - intuitive API using the [Bon](https://bon-rs.com) builder library
- **Async/await ready** - full async support with Tokio
- **Type safe** - strongly typed with compile-time guarantees
- **Easy setup** - simple configuration from environment variables
- **Comprehensive** - support for traces, observations, scores, and more
- **Batch processing** - automatic batching with retry logic and chunking
- **Production ready** - built-in timeouts, connection pooling, and error handling
- **Self-hosted support** - full support for self-hosted Langfuse instances

## Installation

```toml
[dependencies]
langfuse-ergonomic = "*"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### Optional Features

```toml
[dependencies]
langfuse-ergonomic = { version = "*", features = ["compression"] }
```

- `compression` - Enable gzip, brotli, and deflate compression for requests (reduces bandwidth usage)

## Quick Start

```rust
use langfuse_ergonomic::{ClientBuilder, LangfuseClient};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    // Create a trace
    let trace = client.trace()
        .name("my-application")
        .input(json!({"query": "Hello, world!"}))
        .output(json!({"response": "Hi there!"}))
        .user_id("user-123")
        .tags(["production", "chat"])
        .call()
        .await?;

    println!("Created trace: {}", trace.id);

    // Read a trace back: all of its observations (Langfuse v4 observations API)
    let observations = client.trace_observations(&trace.id).await?;
    println!("Trace has {} observations", observations.len());

    // List traces: root observations, with filters and cursor pagination
    let page = client.observations()
        .is_root_observation(true)
        .user_id("user-123")
        .limit(10)
        .call()
        .await?;

    // Access trace data with type safety
    for root in &page.data {
        println!("Trace ID: {:?}, Name: {:?}", root.trace_id, root.name);
    }
    // page.next_cursor holds the cursor for the next page, if any

    // Create a dataset
    let dataset = client.create_dataset()
        .name("my-dataset")
        .description("Example dataset")
        .call()
        .await?;

    Ok(())
}
```

## Langfuse v3 and v4

The client talks to either a Langfuse v3 or a Langfuse v4 server. You choose which
with an explicit setting; the client never probes the server.

```rust
use langfuse_ergonomic::{ApiVersion, ClientBuilder};

let client = ClientBuilder::from_env()?        // also reads LANGFUSE_API_VERSION=v3|v4
    .api_version(ApiVersion::V3)               // only for servers still on Langfuse v3
    .build()?;
```

**The default is `ApiVersion::V4`.** Langfuse Cloud shuts down the v3 ingestion of
traces and observations (`POST /api/public/ingestion` accepts only `score-create`
events from then on) and the v3 trace, observation and session reads on
**2026-11-16**. Self-hosted v3 servers keep the v3 endpoints until they are
upgraded; set `ApiVersion::V3` (or `LANGFUSE_API_VERSION=v3`) for them.

What changes with the version:

| | `ApiVersion::V3` | `ApiVersion::V4` (default) |
|---|---|---|
| `trace()`, `span()`, `generation()`, `event()`, `update_*()` | `POST /api/public/ingestion` | OTLP/HTTP JSON to `POST /api/public/otel/v1/traces` with `x-langfuse-ingestion-version: 4` |
| `score()` and its helpers | `score-create` ingestion event | `POST /api/public/scores` |
| `Batcher` | ingestion batches, 207 per-event results | one OTLP export per batch, then one scores request per score |
| Returned ids | as passed or generated | OpenTelemetry ids (see below) |
| `get_trace`, `list_traces`, `get_observation(s)`, dataset runs (deprecated) | v3 read endpoints | `Error::Configuration`, no request |
| `observations()`, `trace_observations()` | `GET /api/public/v2/observations` (if the v3 server has it) | `GET /api/public/v2/observations` |

Reads move to the v2 observations API (`GET /api/public/v2/observations`), see
[Reading data](#reading-data).

The builders and the batcher keep their signatures; only the transport changes.
The span, generation and event builders gain optional `user_id`, `session_id`
and `tags` setters for traces created elsewhere. The batcher keeps batching,
retries with backoff, `Retry-After` and splitting a batch on HTTP 413 in both
modes. An OTLP export succeeds or fails as a whole, so in V4 there are no
per-span 207 results. A successful export means Langfuse *queued* the spans;
processing is asynchronous. Scores are sent one by one: a 429 waits for
`Retry-After` and retries (the wait holds the batcher's flush lock), a 401/403
stops the batch, and any other failure is reported for its own event.

In V3, the observation builders' `user_id`/`session_id`/`tags` setters upsert
the trace. Used before `trace()`, that upsert creates the trace and fixes its
timestamp, which v3 does not change afterwards.

### Ids in V4

Langfuse v4 stores the OpenTelemetry ids: a 32-hex-character trace id and a
16-hex-character observation (span) id. The client maps every id
deterministically (`langfuse_ergonomic::otel::{trace_id, observation_id}`):

- a trace id that is already 32 hex characters, or any UUID, is used as is
  (lowercase, without hyphens);
- an observation id that is already 16 hex characters is used as is;
- any other string is hashed (UUID v5 in a fixed namespace) to the right length.

The mapping is idempotent, so you can pass either your own id or the id the
client returned: both resolve to the same trace or observation. Parent links and
score targets keep working. The builders return the mapped id, and
`TraceResponse::url()` points at it. `client.effective_trace_id(..)` and
`client.effective_observation_id(..)` give the stored id for any input.

Data written under Langfuse v3 with ids that are not OTel ids keeps those raw
ids. A V4 client maps every id, so it cannot address that data (scores,
`delete_trace`, the `trace_id` read filter); use a V3 client for it.

### Traces in V4

OpenTelemetry has no trace record of its own. `trace()` writes a zero-duration
root span (id `otel::trace_root_span_id(trace_id)`) that carries the trace
attributes and the trace input and output.

The client keeps a bounded write state (`otel::STATE_CAPACITY` entries), shared
by its clones and batchers, to keep traces consistent:

- Observations without a `parent_observation_id` become children of the
  trace's root span. If this client has not written the trace, the first such
  observation has no parent and becomes the root; a later `trace()` adds the
  trace attributes to it instead of creating a second root. Unlike V3, where
  parentless observations were siblings, a second parentless observation
  becomes a child of the first.
- The state is per client and least-recently-used (`ClientBuilder::v4_state_capacity`
  and `v4_state_max_bytes` set its size; ended spans are evicted first). When
  several processes or clients write to one trace, attach observations to the
  root written by `trace()` with
  `parent_observation_id(otel::trace_root_span_id(trace_id))`.
- The trace's user id, session id, tags, release and version are stamped on
  every observation span written after `trace()` (or after an observation
  builder's `user_id`/`session_id`/`tags` setters). Langfuse needs them on
  every span to filter observations, sessions and dashboards by them. Call
  `trace()` before the trace's observations.

### Updates in V4

Langfuse v4 stores each span as one complete row keyed by its start time and
does not merge rows field by field; a partial span would become a duplicate
row. So `update_span()`, `update_generation()` and calling `trace()` again with
the same id re-send the **full** span this client wrote, with the changes merged
in and the **original start time** (a new start time is ignored). Updating an
observation this client did not write, or no longer remembers, fails with
`Error::Validation`.

Langfuse keeps the row with the latest processing time, and separate exports of
the same span are not ordered, so a `span()` followed by a separate
`update_span()` can leave the older payload in place. Send create and update
through one `Batcher` flush (it exports only the merged span), or better:

**Preferred V4 pattern:** write each observation once, complete with its
`end_time` and output, instead of creating it and updating it later.

### Attribute mapping

Attribute names follow Langfuse's
[OpenTelemetry property mapping](https://langfuse.com/integrations/native/opentelemetry).

| Field | OTel attribute |
|---|---|
| trace name | `langfuse.trace.name` (and the root span name) |
| trace user id / session id | `user.id` / `session.id` |
| trace tags | `langfuse.trace.tags` (string array) |
| trace public | `langfuse.trace.public` (bool) |
| trace metadata | `langfuse.trace.metadata` (JSON object) and `langfuse.trace.metadata.<key>` |
| trace input / output | `langfuse.observation.input` / `.output` and `langfuse.trace.input` / `.output` on the root span |
| observation type | `langfuse.observation.type` (`span`, `generation`, `event`, ...) |
| input / output | `langfuse.observation.input` / `langfuse.observation.output` (JSON) |
| observation metadata | `langfuse.observation.metadata` (JSON object) and `langfuse.observation.metadata.<key>` |
| level / status message | `langfuse.observation.level` / `langfuse.observation.status_message`; `ERROR` also sets the span status to error |
| model / model parameters | `langfuse.observation.model.name` / `langfuse.observation.model.parameters` (JSON) |
| usage / cost | `langfuse.observation.usage_details` / `langfuse.observation.cost_details` (JSON) |
| completion start time | `langfuse.observation.completion_start_time` (ISO 8601) |
| prompt name / version | `langfuse.observation.prompt.name` / `langfuse.observation.prompt.version` |
| version / release / environment | `langfuse.version` / `langfuse.release` / `langfuse.environment` |
| trace user id, session id, tags, release, version on child spans | stamped from the trace (see above) |
| start / end time | `startTimeUnixNano` / `endTimeUnixNano` (events: end = start) |

Metadata is written twice: the whole object as a JSON string, which keeps value
types and nesting, and each string value as its own attribute, the form Langfuse
filters on. Non-object metadata is wrapped as `{"value": ...}`. The keys
`attributes`, `resourceAttributes` and `scope` collide with fields the Langfuse
server adds to OTel metadata, so they are renamed to `user_<key>`.

### Reading data

Langfuse v4 reads traces, observations and sessions through
`GET /api/public/v2/observations`. A trace is its root observation.

```rust
use langfuse_ergonomic::{ALL_OBSERVATION_FIELDS, ObservationLevel};

// Every observation of one trace (all pages, all field groups)
let observations = client.trace_observations(&trace_id).await?;

// Filtered listing with cursor pagination
let mut cursor = None;
loop {
    let page = client.observations()
        .is_root_observation(true)          // traces only
        .from_start_time(chrono::Utc::now() - chrono::Duration::days(1))
        .user_id("user-123")
        .level(ObservationLevel::Error)
        .fields(ALL_OBSERVATION_FIELDS)     // default: core + basic
        .limit(100)
        .maybe_cursor(cursor)
        .call()
        .await?;
    // ... use page.data ...
    match page.next_cursor {
        Some(next) => cursor = Some(next),
        None => break,
    }
}
```

The other filters are `trace_id`, `parent_observation_id`, `to_start_time`,
`session_id`, `observation_type`, `name`, `environment`, `version` and the
structured JSON `filter`. In V4, `trace_id` and `parent_observation_id` go
through the same id mapping as writes, so your own ids work.

- `limit` is at most `MAX_OBSERVATIONS_PAGE_SIZE` (1000, the server maximum);
  larger values fail with `Error::Validation` before the request.
  `trace_observations()` uses pages of 1000.
- The API does not de-duplicate rows. `trace_observations()` keeps one
  observation per id (the latest `updated_at`) as a safety net.
- `usage_details` holds integers; non-integer usage counts from the server are
  rounded.

The v3-only reads stay available for Langfuse v3 servers, but are
`#[deprecated]` and return `Error::Configuration` (without a request) when
`ApiVersion::V4` is set:

| v3 method | v4 replacement |
|---|---|
| `get_trace(id)` | `trace_observations(id)` |
| `list_traces()` | `observations().is_root_observation(true)` (see the note below) |
| `get_observation(id)` / `get_observations()` | `observations()` (with `trace_id`, `name`, ...) |
| `get_dataset_run(s)`, `delete_dataset_run` | Langfuse experiments (not wrapped yet) |

A trace is listed through its **root observation**. With this client that is the
span written by `trace()`, or, when the trace was not created with `trace()`,
its first parentless observation (see [Traces in V4](#traces-in-v4)). Traces
whose root span never reached Langfuse (for example, every span has a parent
that was not exported) have no root observation, so
`is_root_observation(true)` does not list them; read them by `trace_id`.

Deletes (`delete_trace`, `delete_multiple_traces`, datasets, prompts) work in
both modes.

### Migrating to v4

1. Self-hosted v3 server: set `.api_version(ApiVersion::V3)` or
   `LANGFUSE_API_VERSION=v3` and nothing else changes. Switch to V4 when the
   server is upgraded.
2. Langfuse Cloud or a v4 server: keep the default. Then:
   - Store and reuse the ids the builders return (or keep passing your own: the
     mapping is deterministic). Ids that are not OTel ids, such as
     `"my-trace"`, are stored as their hashed form, so look them up with
     `client.effective_trace_id("my-trace")`.
   - Call `trace()` for each trace, before its observations; it creates the
     root span and the trace attributes propagate to the observations.
   - Write observations once, complete, rather than create-then-update.
     Updates only work for observations this client wrote.
   - Batches no longer return 207 per-event results for spans.
   - Replace `get_trace`/`list_traces`/`get_observation(s)` with
     `trace_observations`/`observations` (see [Reading data](#reading-data)).
     Data written with OpenTelemetry is available there in real time.
3. `generation().model_parameters(json!({...}))` is now sent (it was ignored
   before) in both modes.

## Type Safety

### Migrating to client-base 0.20

`langfuse-client-base` 0.20 is generated with `timvw-rust` 0.22. The ergonomic
builders are unchanged. In the base models `OpenAiCompletionUsageSchema` and
`OpenAiResponseUsageSchema`, the four `*_tokens_details` maps now hold
`Option<i64>` values, so code that builds them directly writes
`("cached_tokens".into(), Some(1))`. In V4 mode a `null` count is skipped
instead of being sent as a usage detail. See the base crate's
`migration/api-diff-0.19.0-to-0.20.0.md`.

### Migrating to client-base 0.19

`langfuse-client-base` 0.19 is generated with `timvw-rust` 0.21. The ergonomic
builders keep their signatures. Code that calls the base API directly (for
example through `client.configuration()`) changes:

- Request bodies are passed by reference: `.ingestion_batch_request(&request)`.
- `apis::Error::ResponseError` holds a `Box<ResponseContent<T>>`, and
  `ResponseContent` has a `headers` field.
- Every base model implements `models::validation::Validate`.

For errors mapped by status (`Error::Auth`, `RateLimit`, `Client`, `Server`)
from unbatched calls such as `trace()` and `get_trace()`, `Error::request_id()`
and `Error::retry_after()` now return the `x-request-id` and `Retry-After`
response headers instead of `None`. See
the base crate's `migration/api-diff-0.18.0-to-0.19.0.md`.

`list_traces()` now goes through the generated client. A 2xx response must have a
JSON content type, and transport and serde errors read
`Failed to list traces: error in reqwest: ...` (or `error in serde: ...`).

### Migrating to client-base 0.18

`langfuse-client-base` 0.18 adds role-based auth for blob storage integrations:
a `BlobStorageIntegrationAuthType` enum (`AccessKeys`, `RoleArn`), four new fields
on `BlobStorageIntegrationResponse` and two optional fields on
`CreateBlobStorageIntegrationRequest`. The ergonomic builders are unchanged; only
code that builds those base models directly needs the new fields. See the base
crate's `migration/api-diff-0.17.0-to-0.18.0.md`.

### Migrating to client-base 0.17

This release uses `langfuse-client-base` 0.17, generated with the stable naming,
union and integer rules of its generator. The ergonomic builders keep their
signatures; code that builds base models directly changes:

- Ingestion variants drop the numeric suffix and the `Box`:
  `IngestionEvent::TraceEvent(IngestionEventTraceEvent { body: trace_body, ... })`
  instead of `IngestionEvent::TraceEvent1(Box::new(TraceEvent1 { body: Box::new(trace_body), ... }))`.
  The same holds for the score, span, generation, event and observation events,
  and for unions such as `CreatePromptRequest` and `ScoreV3`. A model is boxed
  only where a type contains itself.
- Integers are `i64` unless the spec says `format: int32`: page and limit
  parameters, prompt versions, usage counts and similar fields.
- Structs derive `Default` only when they have no required field, and enums no
  longer implement `Default`. Build such a struct with `new(..)` or its builder.
- Enums a response can carry accept values the spec does not list as `Unknown`,
  and tagged unions as `Unknown(serde_json::Value)`, so one new value no longer
  fails a whole response. An exhaustive `match` on such a type needs a new arm.
- Closed enums implement `FromStr` and `TryFrom<&str>`; structs no longer
  implement `Display`.

### Migrating to client-base 0.15

This release uses `langfuse-client-base` 0.15.0 and requires Rust 1.88.
The ergonomic trace, span, generation, and event builders still accept
`chrono::DateTime<Utc>`. When constructing base models directly, timestamp fields
now use `DateTime<FixedOffset>` (for example, `Utc::now().fixed_offset()`).
Trace list filters still accept RFC3339 strings; invalid dates return a validation error.
Dataset schemas must be JSON objects or explicit `null`.

`get_observation()` now returns `ObservationsViewSingle`, re-exported from this
crate. Its `trace_id` is `Option<String>` and its dates are typed. Observation
lists continue to use `ObservationsViews`, whose entries are `ObservationsView`.

Batch ingestion uses named variants such as
`IngestionEvent::TraceEvent1(Box::new(TraceEvent1 { ... }))` (0.17:
`IngestionEvent::TraceEvent(IngestionEventTraceEvent { ... })`) instead of
`IngestionEvent::IngestionEventOneOf(...)`. See the updated `batch_ingestion`
example. The base client's SDK-log ingestion variant has been removed.

All API methods return strongly-typed structs instead of JSON values. Types are auto-generated from the Langfuse OpenAPI specification and re-exported for convenience:

```rust
use langfuse_ergonomic::{Traces, Dataset, Prompt, ObservationsView};

// You can also import directly from the base crate (both are equivalent):
// use langfuse_ergonomic::Traces;
// use langfuse_client_base::models::Traces;
```

Benefits:
- ✅ Compile-time type checking
- ✅ IDE autocomplete for all fields
- ✅ No runtime JSON parsing errors
- ✅ Full API documentation on types

## Configuration

Set these environment variables:

```bash
LANGFUSE_PUBLIC_KEY=pk-lf-...
LANGFUSE_SECRET_KEY=sk-lf-...
LANGFUSE_BASE_URL=https://cloud.langfuse.com  # Optional
LANGFUSE_API_VERSION=v4                        # Optional: v3 or v4 (default v4)
```

Or configure explicitly with advanced options:

```rust
use langfuse_ergonomic::{ApiVersion, ClientBuilder};
use std::time::Duration;

let client = ClientBuilder::new()
    .public_key("pk-lf-...")
    .secret_key("sk-lf-...")
    .base_url("https://cloud.langfuse.com")
    .timeout(Duration::from_secs(30))        // Custom timeout
    .connect_timeout(Duration::from_secs(5)) // Connection timeout
    .user_agent("my-app/1.0.0")              // Custom user agent
    .api_version(ApiVersion::V4)             // Langfuse server API (default V4)
    .build()?;
```

## Examples

Check the `examples/` directory for more usage examples:

```bash
# Trace examples
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces

# Trace fetching and management
cargo run --example traces_fetch

# Observations (spans, generations, events)
cargo run --example observations

# Scoring and evaluation
cargo run --example scores

# Dataset management
cargo run --example datasets

# Prompt management
cargo run --example prompts

# Batch processing
cargo run --example batch_ingestion

# Self-hosted configuration
cargo run --example self_hosted

# Advanced features (prompts, dataset items, observation updates)
cargo run --example advanced_features
```

### Batch Processing

The client supports efficient batch processing with automatic chunking, retry logic, and comprehensive error handling:

#### Default Configuration
- **Max events per batch**: 100 events
- **Max batch size**: 3.5 MB (conservative limit for Langfuse Cloud's 5MB limit)
- **Auto-flush interval**: 5 seconds
- **Max retries**: 3 attempts with exponential backoff
- **Retry jitter**: Enabled by default (25% random jitter to avoid thundering herd)
- **Backpressure policy**: Block (waits when queue is full)
- **Max queue size**: 10,000 events

```rust
use langfuse_ergonomic::{Batcher, BackpressurePolicy, ClientBuilder, LangfuseClient};
use std::time::Duration;

let client = ClientBuilder::from_env()?.build()?;

// Create a batcher with custom configuration
let batcher = Batcher::builder()
    .client(client)
    .max_events(50)                            // Events per batch (default: 100)
    .max_bytes(2_000_000)                      // Max batch size in bytes (default: 3.5MB)
    .flush_interval(Duration::from_secs(10))   // Auto-flush interval (default: 5s)
    .max_retries(5)                            // Retry attempts (default: 3)
    .max_queue_size(5000)                      // Max events to queue (default: 10,000)
    .backpressure_policy(BackpressurePolicy::DropNew) // What to do when queue is full
    .build()
    .await;

// Add events - they'll be automatically batched
for event in events {
    batcher.add(event).await?;
}

// Manual flush if needed
let response = batcher.flush().await?;
println!("Sent {} events", response.success_count);

// Monitor metrics
let metrics = batcher.metrics();
println!("Queued: {}, Flushed: {}, Failed: {}, Dropped: {}", 
    metrics.queued, metrics.flushed, metrics.failed, metrics.dropped);

// Graceful shutdown (flushes remaining events)
let final_response = batcher.shutdown().await?;
```

#### Advanced Features

**207 Multi-Status Handling** (V3): Automatically handles partial failures where some events succeed and others fail. In V4 an OTLP export succeeds or fails as a whole, and scores are reported one by one.

**Backpressure Policies**:
- `Block`: Wait when queue is full (default)
- `DropNew`: Drop new events when queue is full
- `DropOldest`: Remove oldest events to make room

**Metrics & Monitoring**:
```rust
let metrics = batcher.metrics();
// Available metrics:
// - queued: Current events waiting to be sent
// - flushed: Total successfully sent
// - failed: Total failed after all retries
// - dropped: Total dropped due to backpressure
// - retries: Total retry attempts
// - last_error_ts: Unix timestamp of last error
```

**Error Handling**:
```rust
match batcher.flush().await {
    Ok(response) => {
        println!("Success: {}, Failed: {}", 
            response.success_count, response.failure_count);
    }
    Err(Error::PartialFailure { success_count, failure_count, errors, .. }) => {
        println!("Partial success: {} ok, {} failed", success_count, failure_count);
        for error in errors {
            if error.retryable {
                println!("Retryable error: {}", error.message);
            }
        }
    }
    Err(e) => eprintln!("Complete failure: {}", e),
}
```

## API Coverage

### Implemented Features

#### Traces
- **Creation** - Full trace creation with metadata support
- **Fetching** - `trace_observations()` reads all observations of a trace (v4); `get_trace()` on v3
- **Listing** - `observations()` with filters and cursor pagination (v4); `list_traces()` on v3
- **Management** - Delete single or multiple traces
- Session and user tracking
- Tags and custom timestamps
- Input/output data capture

#### Observations
- **Spans** - Track execution steps and nested operations
- **Generations** - Monitor LLM calls with token usage
- **Events** - Log important milestones and errors
- Nested observations with parent-child relationships
- Log levels (DEBUG, INFO, WARNING, ERROR)

#### Scoring
- **Numeric scores** - Evaluate with decimal values (0.0-1.0)
- **Categorical scores** - Text-based classifications
- **Binary scores** - Success/failure tracking
- **Rating scores** - Star ratings and scales
- Trace-level and observation-level scoring
- Score metadata and comments
- Annotation queue linkage for human-review workflows

Attach scores to annotation queues when triaging human review tasks:

```rust
let score_id = client
    .score()
    .trace_id(&trace.id)
    .name("manual_review_verdict")
    .queue_id("annotation-queue-123")
    .string_value("needs_follow_up")
    .comment("Flagged during human review")
    .call()
    .await?;
```

#### Dataset Management
- **Creation** - Create datasets with metadata
- **Listing** - List all datasets with pagination
- **Fetching** - Get dataset details by name
- **Run Management** - Get, list, and delete dataset runs

#### Prompt Management
- **Fetching** - Get prompts by name and version
- **Listing** - List prompts with filtering
- **Creation** - Create text and chat prompts; invalid chat messages return a validation error with their zero-based index before sending a request

#### Batch Processing
- **Automatic Batching** - Events are automatically grouped into optimal batch sizes
- **Size Limits** - Respects Langfuse's 3.5MB batch size limit
- **Retry Logic** - Exponential backoff for failed requests
- **Partial Failures** - Handles 207 Multi-Status responses
- **Background Processing** - Non-blocking event submission

#### Production Features
- **Timeouts** - Configurable request and connection timeouts
- **Compression** - Optional gzip, brotli, and deflate support (via `compression` feature flag)
- **HTTP/2** - Efficient connection multiplexing
- **Connection Pooling** - Reuses connections for better performance
- **Error Handling** - Structured error types with retry metadata
- **Self-Hosted Support** - Full compatibility with self-hosted instances

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Links

- [Langfuse Documentation](https://langfuse.com/docs)
- [API Reference](https://api.reference.langfuse.com)
- [Base Client](https://github.com/genai-rs/langfuse-client-base)
