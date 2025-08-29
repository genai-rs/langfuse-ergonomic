//! Ergonomic Rust client for Langfuse
//!
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the `bon` crate.

pub mod client;
pub mod error;
pub mod observations;
pub mod scores;
pub mod traces;

pub use client::LangfuseClient;
pub use error::{Error, Result};
pub use observations::{ObservationBuilder, ObservationType};
pub use scores::ScoreBuilder;
pub use traces::TraceResponse;
