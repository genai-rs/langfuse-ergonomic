//! Interoperability with OpenTelemetry (feature `otel-context`, Langfuse v4 only).
//!
//! - While an observation is current, the current OpenTelemetry [`Context`] holds a remote
//!   [`SpanContext`] with the observation's trace and span id, so spans started from
//!   `Context::current()` (openai-ergonomic's `LangfuseInterceptor`, opentelemetry-langfuse, any
//!   tracer) become its children. A trace that is not sampled is attached as not sampled, so a
//!   parent-based sampler drops those spans too.
//! - A root observation started inside an active OpenTelemetry span joins that span's trace as
//!   its child, and follows its sampling decision.

use opentelemetry::trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState};
use opentelemetry::{Context, ContextGuard};

use super::Observation;
use crate::ApiVersion;

/// Attach the OpenTelemetry context of `observation` (Langfuse v4 sinks only).
pub(super) fn attach(observation: &Observation) -> Option<ContextGuard> {
    let span_context = match &observation.inner {
        Some(inner) => {
            if inner.sink.client().api_version() != ApiVersion::V4 {
                return None;
            }
            SpanContext::new(
                TraceId::from_hex(&inner.trace_id).ok()?,
                SpanId::from_hex(&inner.id).ok()?,
                TraceFlags::SAMPLED,
                true,
                TraceState::default(),
            )
        }
        // Not sampled: make OpenTelemetry children unsampled as well.
        None => SpanContext::new(
            TraceId::from_hex(&super::new_trace_id()).ok()?,
            SpanId::from_hex(&super::new_observation_id()).ok()?,
            TraceFlags::default(),
            true,
            TraceState::default(),
        ),
    };
    Some(
        Context::current()
            .with_remote_span_context(span_context)
            .attach(),
    )
}

/// The active OpenTelemetry span a new root observation adopts.
pub(super) struct RemoteParent {
    pub(super) trace_id: String,
    pub(super) span_id: String,
    pub(super) sampled: bool,
}

pub(super) fn current_span() -> Option<RemoteParent> {
    let context = Context::current();
    let span = context.span();
    let span_context = span.span_context();
    span_context.is_valid().then(|| RemoteParent {
        trace_id: format!("{:032x}", span_context.trace_id()),
        span_id: format!("{:016x}", span_context.span_id()),
        sampled: span_context.is_sampled(),
    })
}
