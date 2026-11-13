//! Masking (redaction) of inputs, outputs and metadata before they leave the process.
//!
//! Set a mask function with [`ClientBuilder::mask`](crate::ClientBuilder::mask). The client
//! applies it to the `input`, `output` and `metadata` of every event it sends: the observation
//! and trace builders, [`Batcher::add`](crate::Batcher::add) /
//! [`Batcher::try_add`](crate::Batcher::try_add), and the [`crate::observe`] module (which sends
//! through them). Scores' metadata is masked too. Values that are absent are not passed to it.
//!
//! The function must not panic. If it does anyway, the value is replaced by
//! [`MASK_FAILED`] (the original is never sent) and a warning is logged. On WebAssembly a panic
//! aborts the module (`panic = "abort"`), so it cannot be caught there.
//!
//! ```
//! use langfuse_ergonomic::ClientBuilder;
//! use serde_json::Value;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::new()
//!     .public_key("pk-lf-...")
//!     .secret_key("sk-lf-...")
//!     .mask(|value: Value| match value {
//!         Value::String(s) if s.contains('@') => Value::String("[email redacted]".into()),
//!         other => other,
//!     })
//!     .build()?;
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use langfuse_client_base::models::IngestionEvent;
use serde_json::Value;

/// What a value becomes when the mask function panics.
pub const MASK_FAILED: &str = "<masking failed>";

type MaskFn = dyn Fn(Value) -> Value + Send + Sync;

/// A mask function, shared by a client, its clones and its batchers.
#[derive(Clone)]
pub struct Mask(Arc<MaskFn>);

impl Mask {
    /// Wrap a mask function.
    pub fn new(f: impl Fn(Value) -> Value + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    /// Apply the function to one value, replacing it with [`MASK_FAILED`] if the function panics.
    pub fn apply(&self, value: Value) -> Value {
        match catch_unwind(AssertUnwindSafe(|| (self.0)(value))) {
            Ok(masked) => masked,
            Err(_) => {
                tracing::warn!("the Langfuse mask function panicked; the value is replaced");
                Value::String(MASK_FAILED.to_owned())
            }
        }
    }

    fn apply_opt(&self, value: &mut Option<Value>) {
        if let Some(inner) = value.take() {
            *value = Some(self.apply(inner));
        }
    }

    fn apply_double(&self, value: &mut Option<Option<Value>>) {
        if let Some(inner) = value.as_mut() {
            self.apply_opt(inner);
        }
    }

    /// Apply the function to the input, output and metadata of an ingestion event.
    pub fn apply_to_event(&self, event: &mut IngestionEvent) {
        use IngestionEvent as E;
        match event {
            E::TraceEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::ScoreEvent(e) => self.apply_double(&mut e.body.metadata),
            E::CreateSpanEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::UpdateSpanEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::CreateGenerationEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::UpdateGenerationEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::CreateEventEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::CreateObservationEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
            E::UpdateObservationEvent(e) => {
                self.apply_double(&mut e.body.input);
                self.apply_double(&mut e.body.output);
                self.apply_double(&mut e.body.metadata);
            }
        }
    }
}

impl fmt::Debug for Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Mask(..)")
    }
}

impl<F> From<F> for Mask
where
    F: Fn(Value) -> Value + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Self::new(f)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic)]
    use super::*;
    use langfuse_client_base::models::{
        ingestion_event_create_span_event::Type, CreateSpanBody, IngestionEventCreateSpanEvent,
    };
    use serde_json::json;

    fn span(input: Value) -> IngestionEvent {
        IngestionEvent::CreateSpanEvent(IngestionEventCreateSpanEvent::new(
            "e".into(),
            "t".into(),
            CreateSpanBody {
                input: Some(Some(input)),
                output: Some(None),
                metadata: Some(Some(json!({"k": "secret"}))),
                ..CreateSpanBody::default()
            },
            Type::SpanCreate,
        ))
    }

    #[test]
    fn masks_input_and_metadata_and_skips_absent_values() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = calls.clone();
        let mask = Mask::new(move |_| {
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            json!("***")
        });
        let mut event = span(json!("hello"));
        mask.apply_to_event(&mut event);
        let IngestionEvent::CreateSpanEvent(e) = event else {
            panic!()
        };
        assert_eq!(e.body.input, Some(Some(json!("***"))));
        assert_eq!(e.body.metadata, Some(Some(json!("***"))));
        assert_eq!(e.body.output, Some(None));
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 2);
    }

    #[test]
    fn a_panicking_mask_never_leaks_the_value() {
        let mask = Mask::new(|_| panic!("boom"));
        let mut event = span(json!("secret"));
        mask.apply_to_event(&mut event);
        let IngestionEvent::CreateSpanEvent(e) = event else {
            panic!()
        };
        assert_eq!(e.body.input, Some(Some(json!(MASK_FAILED))));
    }
}
