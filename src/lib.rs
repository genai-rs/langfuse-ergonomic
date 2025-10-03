//! # langfuse-ergonomic
//!
//! Ergonomic Rust client for [Langfuse](https://langfuse.com), the open-source LLM observability platform.
//!
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the [`bon`](https://bon-rs.com) crate.
//!
//! ## Features
//!
//! - **Builder Pattern** - Intuitive API using the bon builder pattern library
//! - **Async/Await** - Full async support with Tokio
//! - **Type Safe** - Strongly typed with compile-time guarantees
//! - **Easy Setup** - Simple configuration from environment variables
//! - **Comprehensive** - Support for traces, observations, scores, and more
//! - **Batch Processing** - Automatic batching with retry logic and chunking
//! - **Production Ready** - Built-in timeouts, connection pooling, and error handling
//! - **Self-Hosted Support** - Full support for self-hosted Langfuse instances
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
//! // List traces with strongly-typed response
//! let traces = client.list_traces().limit(10).call().await?;
//! for trace in &traces.data {
//!     println!("Trace ID: {}, Name: {:?}", trace.id, trace.name);
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
//! # use langfuse_ergonomic::{ClientBuilder, Traces, Dataset, Prompt};
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let client = ClientBuilder::from_env()?.build()?;
//! // Traces are strongly typed
//! let traces: Traces = client.list_traces().call().await?;
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
//! - `compression` - Enable gzip, brotli, and deflate compression for requests
//!
//! ## Examples
//!
//! See the `examples/` directory for more usage patterns:
//! - `basic_trace` - Simple trace creation
//! - `batch_ingestion` - Batch processing with automatic chunking
//! - `trace_with_metadata` - Rich metadata and tagging
//! - `self_hosted` - Connecting to self-hosted instances
//!
//! ## License
//!
//! Licensed under either of:
//! - Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
//! - MIT license ([LICENSE-MIT](LICENSE-MIT))

#![warn(rustdoc::broken_intra_doc_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod batcher;
pub mod client;
pub mod datasets;
pub mod error;
pub mod observations;
pub mod prompts;
pub mod scores;
pub mod traces;

// Re-export commonly used types at the crate root for convenience
pub use batcher::{
    BackpressurePolicy, BatchEvent, Batcher, BatcherBuilderWithClient, BatcherConfig,
    BatcherMetrics, BatcherMetricsSnapshot,
};
pub use client::{ClientBuilder, LangfuseClient};
pub use error::{Error, EventError, IngestionResponse, Result};
pub use traces::{IdGenerator, TraceResponse};

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
    DatasetRunWithItems, IngestionBatchRequest, IngestionEvent, ObservationLevel, ObservationsView,
    ObservationsViews, PaginatedDatasetItems, PaginatedDatasetRuns, PaginatedDatasets, Prompt,
    PromptMetaListResponse, ScoreDataType, Trace, TraceBody, TraceWithDetails,
    TraceWithFullDetails, Traces,
};
