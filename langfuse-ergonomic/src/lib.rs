//! Ergonomic Rust client for Langfuse
//!
//! This crate provides a user-friendly interface to the Langfuse API using builder patterns
//! powered by the `bon` crate.

pub mod client;
pub mod error;
// TODO: Re-enable these modules once updated to use generated API
// pub mod observations;
// pub mod scores;
pub mod traces;

pub use client::LangfuseClient;
pub use error::{Error, Result};
pub use traces::TraceResponse;