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
//! use langfuse_ergonomic::LangfuseClient;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create client from environment variables
//! let client = LangfuseClient::from_env()?;
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
//! # Ok(())
//! # }
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
//! use langfuse_ergonomic::LangfuseClient;
//! use std::time::Duration;
//!
//! let client = LangfuseClient::builder()
//!     .public_key("pk-lf-...")
//!     .secret_key("sk-lf-...")
//!     .base_url("https://cloud.langfuse.com".to_string())
//!     .timeout(Duration::from_secs(30))
//!     .build();
//! ```
//!
//! ## Batch Processing
//!
//! The client supports efficient batch processing with automatic chunking and retry logic:
//!
//! ```no_run
//! use langfuse_ergonomic::{Batcher, BackpressurePolicy, LangfuseClient};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = LangfuseClient::from_env()?;
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
pub use client::LangfuseClient;
pub use error::{Error, EventError, IngestionResponse, Result};
pub use traces::{IdGenerator, TraceResponse};

// Re-export frequently used types from langfuse-client-base to reduce direct imports
pub use langfuse_client_base::models::{
    CreateEventBody, CreateGenerationBody, CreateSpanBody, IngestionBatchRequest, IngestionEvent,
    ObservationLevel, ScoreDataType, TraceBody,
};
