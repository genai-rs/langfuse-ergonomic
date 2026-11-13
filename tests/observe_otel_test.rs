//! Feature `otel-context`: OpenTelemetry spans nest under observations (as openai-ergonomic's
//! `LangfuseInterceptor` spans do: it starts them from `Context::current()`), and root
//! observations join the active OpenTelemetry span.
#![cfg(all(
    feature = "otel-context",
    feature = "macros",
    not(target_family = "wasm")
))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod recorder;

use langfuse_ergonomic::{observe, ApiVersion};
use opentelemetry::context::FutureExt as _;
use opentelemetry::trace::{Span, TraceContextExt, Tracer, TracerProvider};
use opentelemetry::Context;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use recorder::{Recorder, GLOBAL_SINK};
use serde_json::json;

fn tracer() -> (SdkTracerProvider, InMemorySpanExporter) {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    (provider, exporter)
}

#[observe(name = "agent-step", as_type = "agent")]
async fn agent_step(provider: &SdkTracerProvider) -> u8 {
    tokio::task::yield_now().await;
    // What an OpenTelemetry instrumentation (e.g. LangfuseInterceptor) does for an LLM call.
    let tracer = provider.tracer("openai-ergonomic");
    let mut span = tracer.start("chat gpt-4o");
    span.end();
    1
}

#[tokio::test]
async fn otel_spans_nest_under_the_current_observation() {
    let _guard = GLOBAL_SINK.lock().await;
    let recorder = Recorder::start().await;
    let batcher = recorder.batcher(recorder.client(ApiVersion::V4)).await;
    observe::install_batcher(batcher.clone());
    let (provider, exporter) = tracer();

    agent_step(&provider).await;
    observe::uninstall();
    batcher.flush().await.unwrap();

    let observation = recorder.v4_span("agent-step");
    let otel = exporter.get_finished_spans().unwrap();
    assert_eq!(otel.len(), 1);
    let child = &otel[0];
    assert_eq!(
        format!("{:032x}", child.span_context.trace_id()),
        observation["traceId"].as_str().unwrap()
    );
    assert_eq!(
        format!("{:016x}", child.parent_span_id),
        observation["spanId"].as_str().unwrap()
    );
    assert!(child.span_context.is_sampled());
}

#[tokio::test]
async fn root_observations_join_the_active_otel_span() {
    let _guard = GLOBAL_SINK.lock().await;
    let recorder = Recorder::start().await;
    let batcher = recorder.batcher(recorder.client(ApiVersion::V4)).await;
    observe::install_batcher(batcher.clone());
    let (provider, exporter) = tracer();

    let tracer = provider.tracer("app");
    let outer = tracer.start("http-request");
    let context = Context::current_with_span(outer);
    agent_step(&provider).with_context(context.clone()).await;
    context.span().end();
    observe::uninstall();
    batcher.flush().await.unwrap();

    let outer = exporter
        .get_finished_spans()
        .unwrap()
        .into_iter()
        .find(|s| s.name == "http-request")
        .unwrap();
    let observation = recorder.v4_span("agent-step");
    assert_eq!(
        observation["traceId"],
        json!(format!("{:032x}", outer.span_context.trace_id()))
    );
    assert_eq!(
        observation["parentSpanId"],
        json!(format!("{:016x}", outer.span_context.span_id()))
    );
}

#[tokio::test]
async fn unsampled_traces_make_otel_children_unsampled() {
    let _guard = GLOBAL_SINK.lock().await;
    let recorder = Recorder::start().await;
    let batcher = recorder.batcher(recorder.client(ApiVersion::V4)).await;
    observe::install_batcher(batcher.clone());
    observe::set_sample_rate(0.0);
    let (provider, exporter) = tracer();

    agent_step(&provider).await;
    observe::set_sample_rate(1.0);
    observe::uninstall();
    batcher.flush().await.unwrap();

    assert!(recorder.v4_spans().is_empty());
    // The default sampler is parent-based: an unsampled parent drops the span.
    assert!(exporter.get_finished_spans().unwrap().is_empty());
}
