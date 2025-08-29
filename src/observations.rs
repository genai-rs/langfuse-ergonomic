//! Observation-related functionality (spans, generations, and events)
//!
//! This module contains types and utilities for observations.
//! The actual client methods are implemented in the traces module to
//! consolidate all client methods under a single #[bon] impl block.

// Re-export common types that might be useful
pub use langfuse_client_base::models::{
    CreateEventBody, CreateGenerationBody, CreateSpanBody, ObservationLevel,
};
