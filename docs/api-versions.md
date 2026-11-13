# Langfuse API versions (v3 and v4)

The client talks to either a Langfuse v3 or a Langfuse v4 server. You choose which
with an explicit setting; the client never probes the server.

```rust,no_run
use langfuse_ergonomic::{ApiVersion, ClientBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?    // also reads LANGFUSE_API_VERSION=v3|v4
        .api_version(ApiVersion::V3)           // only for servers still on Langfuse v3
        .build()?;
    let _ = client;
    Ok(())
}
```

**The default is `ApiVersion::V4`.** Langfuse Cloud shuts down the v3 ingestion of
traces and observations (`POST /api/public/ingestion` accepts only `score-create`
events from then on) and the v3 trace, observation and session reads on
**2026-11-16**. Self-hosted v3 servers keep the v3 endpoints until they are
upgraded, and so does a self-hosted v4 server in the `dual` or `legacy`
`LANGFUSE_MIGRATION_V4_WRITE_MODE` (not in the default `events_only`); set
`ApiVersion::V3` (or `LANGFUSE_API_VERSION=v3`) to use them.

What changes with the version:

| | `ApiVersion::V3` | `ApiVersion::V4` (default) |
|---|---|---|
| `trace()`, `span()`, `generation()`, `event()`, `observation()` and the typed builders, `update_*()`, `#[observe]` | `POST /api/public/ingestion` | OTLP/HTTP JSON to `POST /api/public/otel/v1/traces` with `x-langfuse-ingestion-version: 4` |
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

## Ids in V4

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

## Traces in V4

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

## Updates in V4

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

## Attribute mapping

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

## Reading data

Langfuse v4 reads traces, observations and sessions through
`GET /api/public/v2/observations`. A trace is its root observation.

```rust,no_run
use langfuse_ergonomic::{LangfuseClient, ObservationLevel, ALL_OBSERVATION_FIELDS};

async fn read(client: &LangfuseClient, trace_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Every observation of one trace (all pages, all field groups)
    let observations = client.trace_observations(trace_id).await?;
    println!("{} observations", observations.len());

    // Filtered listing with cursor pagination
    let mut cursor = None;
    loop {
        let page = client
            .observations()
            .is_root_observation(true) // traces only
            .from_start_time(chrono::Utc::now() - chrono::Duration::days(1))
            .user_id("user-123")
            .level(ObservationLevel::Error)
            .fields(ALL_OBSERVATION_FIELDS) // default: core + basic
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
    Ok(())
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
| `get_dataset_run(s)`, `delete_dataset_run` | `list_dataset_run_items()`, [experiments](datasets-and-experiments.md) |

A trace is listed through its **root observation**. With this client that is the
span written by `trace()`, or, when the trace was not created with `trace()`,
its first parentless observation (see [Traces in V4](#traces-in-v4)). Traces
whose root span never reached Langfuse (for example, every span has a parent
that was not exported) have no root observation, so
`is_root_observation(true)` does not list them; read them by `trace_id`.

Deletes (`delete_trace`, `delete_multiple_traces`, datasets, prompts) work in
both modes.

## Migrating to v4

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
