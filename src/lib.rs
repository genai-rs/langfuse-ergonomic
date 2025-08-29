//! Ergonomic Rust client for Langfuse
//!
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the `bon` crate.

pub mod client;
pub mod datasets;
pub mod error;
pub mod observations;
pub mod prompts;
pub mod scores;
pub mod traces;

pub use client::LangfuseClient;
pub use error::{Error, Result};
pub use traces::TraceResponse;
