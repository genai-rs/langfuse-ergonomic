//! Example demonstrating batch ingestion with automatic chunking and retries

use chrono::Utc;
use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};
use langfuse_ergonomic::{Batcher, ClientBuilder};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    // Create a batcher with custom configuration
    let batcher = Batcher::builder()
        .client(client)
        .max_events(50) // Lower limit for demonstration
        .flush_interval(std::time::Duration::from_secs(2))
        .max_retries(3)
        .build()
        .await;

    // Create multiple traces
    for i in 0..20 {
        let trace_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        // Add trace event
        let trace_event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf {
            id: trace_id.clone(),
            timestamp: timestamp.clone(),
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
            body: Box::new(TraceBody {
                id: Some(Some(trace_id.clone())),
                timestamp: Some(Some(timestamp.clone())),
                name: Some(Some(format!("batch-trace-{}", i))),
                user_id: Some(Some("test-user".to_string())),
                metadata: Some(Some(serde_json::json!({
                    "batch_number": i,
                    "batch_test": true,
                    "timestamp": timestamp
                }))),
                release: Some(Some("v1.0.0".to_string())),
                version: Some(Some("1.0.0".to_string())),
                session_id: Some(Some("batch-session-001".to_string())),
                public: Some(Some(false)),
                tags: Some(Some(vec!["batch".to_string(), "test".to_string()])),
                input: Some(Some(serde_json::json!({
                    "test_input": format!("Input for trace {}", i)
                }))),
                output: Some(Some(serde_json::json!({
                    "test_output": format!("Output for trace {}", i)
                }))),
                environment: None,
            }),
            metadata: None,
        }));

        batcher.add(trace_event).await?;

        // Add a small delay to simulate real-world usage
        if i % 5 == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    println!("Added 20 trace events to the batcher");
    println!("Events will be automatically batched and sent...");

    // Wait a moment for automatic flush
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Manually flush any remaining events
    let response = batcher.flush().await?;
    println!(
        "Final flush: {} succeeded, {} failed",
        response.success_count, response.failure_count
    );

    if !response.failures.is_empty() {
        println!("Failures:");
        for failure in &response.failures {
            println!("  - {}: {}", failure.event_id, failure.message);
        }
    }

    // Shutdown the batcher
    let final_response = batcher.shutdown().await?;
    println!(
        "Shutdown complete: {} total succeeded, {} total failed",
        final_response.success_count, final_response.failure_count
    );

    println!("\nBatcher features demonstrated:");
    println!("- Automatic batching of events");
    println!("- Configurable batch size and flush interval");
    println!("- Automatic retry with exponential backoff");
    println!("- Graceful shutdown with final flush");

    Ok(())
}
