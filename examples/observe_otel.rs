//! Feature `otel-context`: OpenTelemetry spans nest under `#[observe]` observations.
//!
//! openai-ergonomic's `LangfuseInterceptor` starts its spans from `Context::current()`, like the
//! "instrumented library" below. Here the OpenTelemetry spans go to an in-memory exporter, so
//! the example shows the ids without a Langfuse server; in an application they go to Langfuse
//! (opentelemetry-langfuse or openai-ergonomic's exporter) and appear inside the observation.
//!
//! `cargo run --example observe_otel --features otel-context`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::{observe, ApiVersion, ClientBuilder};
use opentelemetry::trace::{Span, Tracer, TracerProvider};
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};

/// What an OpenTelemetry instrumentation does for an LLM call.
fn instrumented_llm_call(provider: &SdkTracerProvider) {
    let mut span = provider.tracer("openai-ergonomic").start("chat gpt-4o");
    span.end();
}

#[observe(as_type = "agent")]
async fn agent(provider: &SdkTracerProvider) -> String {
    let observation = observe::current().expect("a sink is installed");
    instrumented_llm_call(provider);
    format!(
        "trace {} / observation {}",
        observation.trace_id().unwrap(),
        observation.id().unwrap()
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();

    // The context is attached for Langfuse v4 sinks; this one sends nowhere useful, the
    // batcher is never flushed.
    let client = ClientBuilder::new()
        .public_key("pk-lf-example")
        .secret_key("sk-lf-example")
        .base_url("http://localhost:3000")
        .api_version(ApiVersion::V4)
        .build()?;
    let batcher = client.batcher().build().await;
    observe::install_batcher(batcher);

    let ids = agent(&provider).await;
    println!("observation: {ids}");
    for span in exporter.get_finished_spans()? {
        println!(
            "OpenTelemetry span '{}': trace {:032x}, parent {:016x}",
            span.name,
            span.span_context.trace_id(),
            span.parent_span_id
        );
    }
    observe::uninstall();
    Ok(())
}
