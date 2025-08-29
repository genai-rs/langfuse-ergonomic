//! Score-related functionality for evaluating traces and observations
//!
//! This module contains types and utilities for scoring.
//! The actual client methods are implemented in the traces module to
//! consolidate all client methods under a single #[bon] impl block.

// Re-export common types that might be useful
pub use langfuse_client_base::models::{CreateScoreValue, ScoreBody, ScoreDataType};
