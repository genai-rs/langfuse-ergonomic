# Migrating

Notes for upgrading between releases, newest first. The [CHANGELOG](../CHANGELOG.md) lists
every change.

## Migrating to 0.15

0.15 adds automatic tracing and closes most gaps to the other Langfuse SDKs. Existing code
keeps compiling; a few behaviours change.

New:

- `#[observe]` (feature `macros`, on by default) and the `langfuse_ergonomic::observe` module:
  see [Observe](observe.md). The attribute lives in a new crate, `langfuse-ergonomic-macros`,
  released with this one; depend on `langfuse-ergonomic` only. To leave the proc macro out,
  disable the default features and enable `rustls` (or `native-tls`) again.
- Feature `otel-context`: OpenTelemetry spans (openai-ergonomic's `LangfuseInterceptor`,
  opentelemetry-langfuse) nest under observations, and root observations join the active
  OpenTelemetry span ([Observe](observe.md#opentelemetry-and-openai-ergonomic)).
- All observation types in the builders: `observation().as_type(..)` and `agent()`, `tool()`,
  `chain()`, `retriever()`, `evaluator()`, `embedding()`, `guardrail()`
  ([Tracing](tracing.md#observation-types)).
- Generations: `usage_details`, `cost_details`, `completion_start_time`,
  `prompt_name`/`prompt_version` and `version` on `generation()`; usage, cost, prompt and model
  parameters on `update_generation()` ([Tracing](tracing.md#generations)).
- Prompts: `client.prompt(name)` with a cache, a stale-on-error fallback, a caller-supplied
  fallback and `compile()` ([Prompts](prompts.md)).
- Experiments: `client.run_experiment(dataset, task)` with evaluators
  ([Datasets and experiments](datasets-and-experiments.md)); `create_dataset_run_item()` and
  `list_dataset_run_items()`.
- Media: `upload_media()`, `get_media()`, `download_media()` and `MediaReference`
  ([Media](media.md)).
- Masking: `ClientBuilder::mask(fn)` ([Masking](masking.md)).
- `Batcher::try_add` (never waits) and `Batcher: Clone` (clones share the queue).
- `update_prompt_version()` takes `remove_labels`, and `labels` may be left out; on success it
  drops the client's cached entries for that prompt.

Changed behaviour:

- The batcher retries 5xx and network errors with exponential backoff and jitter
  (`initial_retry_delay * 2^n`, capped at `max_retry_delay`), and honours a `Retry-After`
  header on 5xx responses as well as on 429. Before, a 5xx waited a fixed 5 seconds and a
  network error 1 second.
- `Error::retry_after()` returns only the server's `Retry-After` (rate limits); it returns
  `None` for `Error::Server` and `Error::Network` instead of the former fixed 5 s / 1 s. If you
  used it to schedule your own retries, add a backoff for those errors.
- `update_generation()` sends the new optional fields only when set (as before for the
  others).
- `generation().usage_details(map)` and `prompt_tokens`/`completion_tokens`/`total_tokens`
  combine: the token setters override the `input`/`output`/`total` keys of the map.

Dependencies:

- `langfuse-client-base` 0.25 (generated with `timvw-rust` 0.29, Langfuse spec of November
  2026) and `timvw-rust-runtime` 0.4. A client given to `ClientBuilder::http_client` is a
  `timvw_rust_runtime::Client` of 0.4: bump your own `timvw-rust-runtime` dependency to 0.4.
- Code that matched `langfuse_client_base::apis::Error::Io` should use a wildcard arm: the
  variant is removed in langfuse-client-base 0.24.
- Code that builds `langfuse_client_base::models::PromptVersionUpdateRequest` itself: see the
  [client-base 0.25 note](https://github.com/genai-rs/langfuse-client-base/blob/main/migration/api-diff-0.24.0-to-0.25.0.md).

The [Langfuse v4 migration](api-versions.md#migrating-to-v4) is described with the API
versions.

## Migrating to 0.14 (client-base 0.22, timvw-rust-runtime 0.3)

- `langfuse-client-base` 0.22 is generated with `timvw-rust` 0.27 and sends through
  `timvw-rust-runtime` 0.3. A client given to `ClientBuilder::http_client` is a
  `timvw_rust_runtime::Client` of 0.3: bump your own `timvw-rust-runtime` dependency to 0.3.
- The secret key is kept as a `SecretString` in the base crate's `Configuration`
  (`configuration().basic_auth`), so it is redacted from `Debug` output.
- New: the crate builds for WebAssembly (`wasm32-unknown-unknown`); see
  [WebAssembly](webassembly.md). There `ClientBuilder::from_env` is not available, and in
  request-scoped hosts (Cloudflare Workers) call `flush()` before returning a response.
  Native behaviour is unchanged.

## Migrating to client-base 0.21

`langfuse-client-base` 0.21 is generated with `timvw-rust` 0.25 and sends through
[`timvw-rust-runtime`](https://docs.rs/timvw-rust-runtime) (tower) instead of
`reqwest-middleware`. The ergonomic builders keep their signatures; the HTTP types change:

- `ClientBuilder::http_client` takes a `timvw_rust_runtime::Client` instead of a
  `reqwest_middleware::ClientWithMiddleware`. Build one with
  `Client::from_service(ReqwestService::new(reqwest_client))` and add tower layers with
  `.layer(..)`; `reqwest-retry` becomes `RetryLayer::new(RetryPolicy::new(n))` (feature `retry`).
- `Error::Network` wraps a `timvw_rust_runtime::Error` (`is_timeout()`, `is_connect()`, ...)
  instead of a `reqwest::Error`, and `Error::Middleware` is gone.
- A timed-out `validate()` returns `Error::Api`.
- Code that uses the base API directly matches `apis::Error::Http` instead of
  `Reqwest`/`ReqwestMiddleware`. See the base crate's `migration/api-diff-0.20.0-to-0.21.0.md`.

## Migrating to client-base 0.20

`langfuse-client-base` 0.20 is generated with `timvw-rust` 0.22. The ergonomic
builders are unchanged. In the base models `OpenAiCompletionUsageSchema` and
`OpenAiResponseUsageSchema`, the four `*_tokens_details` maps now hold
`Option<i64>` values, so code that builds them directly writes
`("cached_tokens".into(), Some(1))`. In V4 mode a `null` count is skipped
instead of being sent as a usage detail. See the base crate's
`migration/api-diff-0.19.0-to-0.20.0.md`.

## Migrating to client-base 0.19

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

## Migrating to client-base 0.18

`langfuse-client-base` 0.18 adds role-based auth for blob storage integrations:
a `BlobStorageIntegrationAuthType` enum (`AccessKeys`, `RoleArn`), four new fields
on `BlobStorageIntegrationResponse` and two optional fields on
`CreateBlobStorageIntegrationRequest`. The ergonomic builders are unchanged; only
code that builds those base models directly needs the new fields. See the base
crate's `migration/api-diff-0.17.0-to-0.18.0.md`.

## Migrating to client-base 0.17

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

## Migrating to client-base 0.15

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
