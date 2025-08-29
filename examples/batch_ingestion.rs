//! Example demonstrating batch ingestion with automatic chunking and retries

use chrono::Utc;
use langfuse_client_base::models::{
    CreateGenerationEvent, CreateSpanEvent, CreateTraceEvent, IngestionEvent, TraceLevel,
};
use langfuse_ergonomic::{Batcher, LangfuseClient};
use std::collections::HashMap;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = LangfuseClient::from_env()?;

    // Create a batcher with custom configuration
    let batcher = Batcher::builder()
        .client(client)
        .max_events(50) // Lower limit for demonstration
        .flush_interval(std::time::Duration::from_secs(2))
        .max_retries(3)
        .build();

    // Create multiple traces with spans and generations
    for i in 0..10 {
        let trace_id = Uuid::new_v4().to_string();

        // Add trace event
        let trace_event = IngestionEvent::IngestionEventOneOf(Box::new(CreateTraceEvent {
            id: trace_id.clone(),
            timestamp: Some(Utc::now()),
            name: Some(format!("batch-trace-{}", i)),
            user_id: Some("test-user".to_string()),
            metadata: Some(HashMap::from([
                ("batch_number".to_string(), serde_json::json!(i)),
                ("batch_test".to_string(), serde_json::json!(true)),
            ])),
            release: Some("v1.0.0".to_string()),
            version: Some("1.0.0".to_string()),
            session_id: Some("batch-session-001".to_string()),
            public: Some(false),
            tags: Some(vec!["batch".to_string(), "test".to_string()]),
            input: None,
            output: None,
            level: Some(TraceLevel::Default),
        }));

        batcher.add(trace_event).await?;

        // Add span events
        for j in 0..3 {
            let span_id = Uuid::new_v4().to_string();
            let span_event = IngestionEvent::IngestionEventOneOf1(Box::new(CreateSpanEvent {
                id: span_id.clone(),
                trace_id: Some(trace_id.clone()),
                parent_observation_id: None,
                name: Some(format!("span-{}-{}", i, j)),
                start_time: Some(Utc::now()),
                end_time: Some(Utc::now()),
                metadata: Some(HashMap::from([(
                    "span_index".to_string(),
                    serde_json::json!(j),
                )])),
                level: Some(TraceLevel::Default),
                status_message: None,
                input: Some(serde_json::json!({
                    "prompt": format!("Process item {} in batch {}", j, i)
                })),
                output: Some(serde_json::json!({
                    "result": format!("Completed processing item {}", j)
                })),
                version: None,
            }));

            batcher.add(span_event).await?;

            // Add generation event
            let gen_event = IngestionEvent::IngestionEventOneOf2(Box::new(CreateGenerationEvent {
                id: Uuid::new_v4().to_string(),
                trace_id: Some(trace_id.clone()),
                parent_observation_id: Some(span_id),
                name: Some(format!("generation-{}-{}", i, j)),
                start_time: Some(Utc::now()),
                completion_start_time: Some(Utc::now()),
                end_time: Some(Utc::now()),
                model: Some("gpt-4".to_string()),
                model_parameters: Some(HashMap::from([
                    ("temperature".to_string(), serde_json::json!(0.7)),
                    ("max_tokens".to_string(), serde_json::json!(100)),
                ])),
                prompt: Some(serde_json::json!({
                    "messages": [
                        {"role": "system", "content": "You are a helpful assistant."},
                        {"role": "user", "content": format!("Process item {}", j)}
                    ]
                })),
                completion: Some(serde_json::json!({
                    "content": format!("Item {} has been processed successfully", j)
                })),
                usage: Some(langfuse_client_base::models::IngestionUsage {
                    input: Some(20),
                    output: Some(15),
                    total: Some(35),
                    unit: Some("TOKENS".to_string()),
                    input_cost: Some(0.0006),
                    output_cost: Some(0.0012),
                    total_cost: Some(0.0018),
                }),
                metadata: None,
                level: Some(TraceLevel::Default),
                status_message: None,
                version: None,
                top_p: None,
                frequency_penalty: None,
                presence_penalty: None,
                max_tokens: Some(100),
                temperature: Some(0.7),
                seed: None,
                function_call: None,
                functions: None,
                response_format: None,
                tool_choice: None,
                tools: None,
            }));

            batcher.add(gen_event).await?;
        }
    }

    println!("Added 40 events to the batcher (10 traces + 30 spans + 30 generations)");
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

    Ok(())
}
