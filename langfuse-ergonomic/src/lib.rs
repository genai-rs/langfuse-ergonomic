//! Ergonomic Rust client for Langfuse
//! 
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the `bon` crate.

pub mod client;
pub mod traces;
pub mod observations;
pub mod scores;
pub mod error;

pub use client::LangfuseClient;
pub use error::{Error, Result};

// Re-export commonly used types from the base client
pub use langfuse_client_base::{ApiError, TraceResponse};