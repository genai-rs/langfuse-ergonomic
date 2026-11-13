//! # langfuse-ergonomic
//!
//! Ergonomic Rust client for [Langfuse](https://langfuse.com), the open-source LLM observability platform.
//!
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the [`bon`](https://bon-rs.com) crate.
//!
//! ## Features
//!
//! - **`#[observe]`** - trace functions automatically, nested across `.await`, with errors,
//!   panics and cancellation recorded: see [`mod@observe`]
//! - **Builders** - traces and every observation type (spans, generations, events, agents,
//!   tools, chains, retrievers, evaluators, embeddings, guardrails), scores, datasets, prompts
//! - **Batching** - the [`Batcher`], with backpressure, exponential backoff and `Retry-After`
//! - **Prompts** - a cache with stale-on-error and fallbacks, and compilation: [`prompts`]
//! - **Experiments** - run a task over a dataset with evaluators: [`experiments`]
//! - **Media** and **masking** - [`media`], [`mask`]
//! - **Langfuse v3 and v4** - OpenTelemetry ingestion by default: [`ApiVersion`], [`otel`]
//! - **WebAssembly** - `wasm32-unknown-unknown` in JavaScript hosts
//!
//! The guides live in the repository's
//! [`docs/`](https://github.com/genai-rs/langfuse-ergonomic/tree/main/docs) directory.
//!
//! ## Quick Start
//!
//! ```no_run
//! use langfuse_ergonomic::{ClientBuilder, LangfuseClient};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client from environment variables
//! let client = ClientBuilder::from_env()?.build()?;
//!
//! // Create a trace
//! let trace = client.trace()
//!     .name("my-application")
//!     .input(json!({"query": "Hello, world!"}))
//!     .output(json!({"response": "Hi there!"}))
//!     .user_id("user-123")
//!     .tags(vec!["production".to_string(), "chat".to_string()])
//!     .call()
//!     .await?;
//!
//! println!("Created trace: {}", trace.id);
//!
//! // Read the trace back (Langfuse v4 observations API)
//! let observations = client.trace_observations(&trace.id).await?;
//! for observation in &observations {
//!     println!("Observation {}: {:?}", observation.id, observation.name);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Type Safety
//!
//! All API methods return strongly-typed structs instead of JSON values:
//!
//! ```no_run
//! # use langfuse_ergonomic::{ClientBuilder, ObservationsPage, Dataset, Prompt};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = ClientBuilder::from_env()?.build()?;
//! // Traces (root observations) are strongly typed
//! let traces: ObservationsPage = client.observations().is_root_observation(true).call().await?;
//! println!("Found {} traces", traces.data.len());
//!
//! // Datasets are strongly typed
//! let dataset: Dataset = client.get_dataset("my-dataset").await?;
//! println!("Dataset: {}", dataset.name);
//!
//! // Prompts are strongly typed
//! let prompt: Prompt = client.get_prompt("my-prompt", None, None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! **Note:** This crate re-exports types from `langfuse-client-base`, which are auto-generated
//! from the Langfuse OpenAPI specification. This ensures type accuracy and allows direct access
//! to all fields and their documentation. You can import types from either crate:
//!
//! ```rust,ignore
//! // Both imports are equivalent:
//! use langfuse_ergonomic::Traces;
//! // or
//! use langfuse_client_base::models::Traces;
//! ```
//!
//! ## Configuration
//!
//! Set these environment variables:
//!
//! ```bash
//! LANGFUSE_PUBLIC_KEY=pk-lf-...
//! LANGFUSE_SECRET_KEY=sk-lf-...
//! LANGFUSE_BASE_URL=https://cloud.langfuse.com  # Optional
//! LANGFUSE_API_VERSION=v4                        # Optional: v3 or v4 (default v4)
//! ```
//!
//! Or configure explicitly:
//!
//! ```no_run
//! # use langfuse_ergonomic::ClientBuilder;
//! # use std::time::Duration;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::new()
//!     .public_key("pk-lf-...")
//!     .secret_key("sk-lf-...")
//!     .base_url("https://cloud.langfuse.com".to_string())
//!     .timeout(Duration::from_secs(30))
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Langfuse v3 and v4
//!
//! The client writes to Langfuse v4 by default: traces and observations are sent as
//! OpenTelemetry spans and scores go to the v4 scores API. Select [`ApiVersion::V3`] for a
//! self-hosted Langfuse v3 server, either with [`ClientBuilder::api_version`] or with
//! `LANGFUSE_API_VERSION=v3`. Langfuse Cloud removes the v3 ingestion of traces and
//! observations on 2026-11-16. See [`ApiVersion`] and [`otel`] for what changes, including
//! how ids are mapped to OpenTelemetry ids.
//!
//! Reads use the v2 observations API: [`LangfuseClient::observations`] (filters and cursor
//! pagination) and [`LangfuseClient::trace_observations`]. The v3-only reads (`get_trace`,
//! `list_traces`, `get_observation(s)`, dataset runs) are deprecated and return
//! [`Error::Configuration`] with [`ApiVersion::V4`].
//!
//! ```no_run
//! use langfuse_ergonomic::{ApiVersion, ClientBuilder};
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::from_env()?
//!     .api_version(ApiVersion::V3)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Batch Processing
//!
//! The client supports efficient batch processing with automatic chunking and retry logic:
//!
//! ```no_run
//! use langfuse_ergonomic::{Batcher, BackpressurePolicy, ClientBuilder};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::from_env()?.build()?;
//!
//! // Create a batcher with custom configuration
//! let batcher = Batcher::builder()
//!     .client(client)
//!     .max_events(50)                            // Events per batch
//!     .flush_interval(Duration::from_secs(10))   // Auto-flush interval
//!     .max_retries(3)                            // Retry attempts
//!     .backpressure_policy(BackpressurePolicy::Block)
//!     .build()
//!     .await;
//!
//! // Events are automatically batched and sent
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `rustls` (default) / `native-tls` - the TLS implementation
//! - `macros` (default) - the [`observe`](macro@observe) attribute
//! - `otel-context` - OpenTelemetry context propagation for observations (Langfuse v4): see
//!   [`mod@observe`]
//! - `compression` - gzip, brotli, and deflate compression for responses
//! - `retry` - `timvw_rust_runtime::retry::RetryPolicy` for clients set with `http_client`
//! - `tracing-http` - `timvw_rust_runtime::trace::TraceLayer`, which logs each request
//!
//! ## WebAssembly
//!
//! The crate builds for `wasm32-unknown-unknown` in a JavaScript host (browsers, Node.js,
//! Cloudflare Workers) with `default-features = false`: the TLS features add nothing there, as the
//! host's `fetch` does TLS (and owns timeouts and compression). The batcher's background task runs
//! with `wasm_bindgen_futures::spawn_local` and waits with the host's `setTimeout`; nothing needs to
//! be `Send`. `ClientBuilder::from_env` is not available there: configure the builder explicitly.
//! Hosts that stop a request's tasks once its response is sent (Cloudflare Workers) also stop the
//! flush interval: call [`Batcher::flush`] before returning the response. The observation
//! context needs no runtime there (add the `macros` feature for `#[observe]`). See
//! `docs/webassembly.md`.
//!
//! ## Examples
//!
//! See the `examples/` directory for more usage patterns:
//! - `observe` - the `#[observe]` attribute
//! - `basic_trace` - Simple trace creation
//! - `observation_types` - agents, tools, chains, retrievers, ...
//! - `batch_ingestion` - Batch processing with automatic chunking
//! - `prompts_cache` - cached prompts with a fallback, compiled
//! - `experiments` - a task over a dataset with evaluators
//! - `self_hosted` - Connecting to self-hosted instances
//!
//! ## License
//!
//! Licensed under either of:
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
//! - MIT license ([LICENSE-MIT](LICENSE-MIT))

#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod api_version;
pub mod batcher;
pub mod client;
pub mod datasets;
pub mod error;
pub mod experiments;
pub mod mask;
pub mod media;
pub mod observations;
pub mod observe;
pub mod otel;
pub mod prompts;
pub mod scores;
mod time;
pub mod traces;

// Re-export commonly used types at the crate root for convenience
pub use api_version::ApiVersion;
pub use batcher::{
    BackpressurePolicy, BatchEvent, Batcher, BatcherBuilderWithClient, BatcherConfig,
    BatcherMetrics, BatcherMetricsSnapshot,
};
pub use client::{ClientBuilder, LangfuseClient};
pub use error::{Error, EventError, IngestionResponse, Result};
pub use experiments::{evaluator_fn, Evaluation, Evaluator, EvaluatorContext, ExperimentResult};
pub use mask::Mask;
pub use media::{MediaField, MediaReference};
pub use observations::{
    ObservationType, ObservationsPage, ALL_OBSERVATION_FIELDS, MAX_OBSERVATIONS_PAGE_SIZE,
};
pub use observe::Observation;
pub use prompts::{CompiledPrompt, FetchedPrompt, PromptFallback, PromptSource};
pub use scores::ScoreValue;
pub use traces::{IdGenerator, TraceResponse};

/// Trace a function as a Langfuse observation: see [`mod@observe`] (feature `macros`).
#[cfg(feature = "macros")]
#[cfg_attr(docsrs, doc(cfg(feature = "macros")))]
pub use langfuse_ergonomic_macros::observe;

#[doc(hidden)]
pub use observe::__private;

// Re-export types from langfuse-client-base for convenience
//
// These types are auto-generated from the Langfuse OpenAPI specification and provide
// strongly-typed access to all API responses. Re-exporting them here allows users to
// import everything they need from a single crate without depending directly on
// langfuse-client-base.
//
// Users can import from either crate:
//   use langfuse_ergonomic::Traces;
//   use langfuse_client_base::models::Traces;  // equivalent
//
// Type categories:
// - Trace types: Trace, TraceBody, TraceWithDetails, TraceWithFullDetails, Traces
// - Observation types: ObservationsView, ObservationsViews, ObservationLevel
// - Dataset types: Dataset, DatasetItem, DatasetRunWithItems, PaginatedDatasets,
//                  PaginatedDatasetItems, PaginatedDatasetRuns
// - Prompt types: Prompt, PromptMetaListResponse
// - Event/Ingestion types: CreateEventBody, CreateGenerationBody, CreateSpanBody,
//                          IngestionEvent, IngestionBatchRequest
// - Utility types: ScoreDataType
pub use langfuse_client_base::models::{
    CreateEventBody, CreateGenerationBody, CreateSpanBody, Dataset, DatasetItem,
    DatasetRunWithItems, IngestionBatchRequest, IngestionEvent, LegacyObservationsViews,
    ObservationLevel, ObservationV2, ObservationsV2Response, ObservationsView,
    ObservationsViewSingle, PaginatedDatasetItems, PaginatedDatasetRuns, PaginatedDatasets, Prompt,
    PromptMetaListResponse, ScoreDataType, Trace, TraceBody, TraceWithDetails,
    TraceWithFullDetails, Traces,
};

/// Response from the legacy observations API, retaining the name used in 0.6.3.
pub use langfuse_client_base::models::LegacyObservationsViews as ObservationsViews;

/// The README and the guides under `docs/`, compiled as doc tests so that their examples keep
/// working.
#[cfg(doctest)]
mod doc_pages {
    #[cfg(feature = "macros")]
    #[doc = include_str!("../README.md")]
    struct Readme;
    #[cfg(feature = "macros")]
    #[doc = include_str!("../docs/getting-started.md")]
    struct GettingStarted;
    #[doc = include_str!("../docs/configuration.md")]
    struct Configuration;
    #[doc = include_str!("../docs/api-versions.md")]
    struct ApiVersions;
    #[doc = include_str!("../docs/tracing.md")]
    struct Tracing;
    #[cfg(feature = "macros")]
    #[doc = include_str!("../docs/observe.md")]
    struct Observe;
    #[doc = include_str!("../docs/batching.md")]
    struct Batching;
    #[doc = include_str!("../docs/scores.md")]
    struct Scores;
    #[doc = include_str!("../docs/prompts.md")]
    struct Prompts;
    #[cfg(feature = "macros")]
    #[doc = include_str!("../docs/datasets-and-experiments.md")]
    struct DatasetsAndExperiments;
    #[doc = include_str!("../docs/media.md")]
    struct Media;
    #[doc = include_str!("../docs/masking.md")]
    struct Masking;
    #[doc = include_str!("../docs/webassembly.md")]
    struct WebAssembly;
    #[doc = include_str!("../docs/migration.md")]
    struct Migration;
    #[doc = include_str!("../docs/development.md")]
    struct Development;
}
