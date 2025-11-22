//! Example demonstrating batch ingestion with 207 Multi-Status handling
//!
//! This example shows how to:
//! - Use the batcher for efficient event ingestion
//! - Handle partial failures (207 responses)
//! - Configure batch size, flush intervals, and backpressure
//! - Monitor metrics (queued, flushed, failed, dropped)
//! - Graceful shutdown with guarantees

use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};
use langfuse_ergonomic::{BackpressurePolicy, Batcher, ClientBuilder};
use serde_json::json;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize from environment variables
    dotenvy::dotenv().ok();

    let client = ClientBuilder::from_env()?.build()?;

    println!(" Starting batch ingestion example with advanced features...\n");

    // Create a batcher with comprehensive configuration
    let batcher = Batcher::builder()
        .client(client)
        .max_events(10) // Batch up to 10 events
        .max_bytes(2_000_000) // Or up to 2MB
        .flush_interval(Duration::from_secs(3)) // Auto-flush every 3 seconds
        .max_retries(3) // Retry failed events up to 3 times
        .fail_fast(false) // Continue on partial failures
        .max_queue_size(100) // Queue up to 100 events
        .backpressure_policy(BackpressurePolicy::Block) // Block when queue is full
        .build()
        .await;

    println!(" Batcher Configuration:");
    println!("  - Max events per batch: 10");
    println!("  - Max batch size: 2MB");
    println!("  - Auto-flush interval: 3 seconds");
    println!("  - Max retries: 3");
    println!("  - Backpressure: Block when full");
    println!("  - Max queue size: 100 events\n");

    // Simulate sending multiple events
    println!(" Adding events to batch...");
    for i in 1..=15 {
        let trace = TraceBody {
            id: Some(Some(format!("batch-trace-{}", i))),
            name: Some(Some(format!("Batch Test Trace {}", i))),
            input: Some(Some(json!({
                "batch_index": i,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))),
            output: Some(Some(json!({
                "processed": true,
                "batch_mode": "efficient"
            }))),
            metadata: Some(Some(json!({
                "source": "batch_example",
                "version": env!("CARGO_PKG_VERSION"),
                "batch_features": {
                    "207_handling": true,
                    "auto_chunking": true,
                    "retry_logic": true,
                    "metrics": true
                }
            }))),
            user_id: Some(Some("batch-user".to_string())),
            session_id: Some(Some(format!("batch-session-{}", i % 3))),
            tags: Some(Some(vec![
                "batch".to_string(),
                format!("group-{}", i % 2),
                "207-example".to_string(),
            ])),
            ..Default::default()
        };

        let event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf::new(
            format!("event-{}", i),
            chrono::Utc::now().to_rfc3339(),
            trace,
            langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        )));

        // Add to batch
        match batcher.add(event).await {
            Ok(_) => {
                println!("   Added event {} to batch", i);

                // Show metrics periodically
                if i % 5 == 0 {
                    let metrics = batcher.metrics();
                    println!(
                        "     Current metrics - Queued: {}, Flushed: {}, Failed: {}, Dropped: {}",
                        metrics.queued, metrics.flushed, metrics.failed, metrics.dropped
                    );
                }

                // Trigger auto-flush at event 10 (max_events)
                if i == 10 {
                    println!("\n   Auto-flush triggered (reached max_events)...");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let metrics = batcher.metrics();
                    println!(
                        "     After auto-flush - Flushed: {}, Queued: {}",
                        metrics.flushed, metrics.queued
                    );
                }
            }
            Err(e) => eprintln!("   Failed to add event {}: {}", i, e),
        }

        // Small delay to simulate real-world event generation
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("\n  Waiting for timer-based auto-flush (3 seconds)...");
    tokio::time::sleep(Duration::from_secs(4)).await;

    let metrics = batcher.metrics();
    println!(" Metrics after auto-flush:");
    println!("  - Events flushed: {}", metrics.flushed);
    println!("  - Events queued: {}", metrics.queued);
    println!("  - Events failed: {}", metrics.failed);
    println!("  - Events dropped: {}", metrics.dropped);
    println!("  - Retry attempts: {}", metrics.retries);

    // Demonstrate manual flush
    println!("\n Adding more events and performing manual flush...");
    for i in 16..=20 {
        let trace = TraceBody {
            id: Some(Some(format!("manual-trace-{}", i))),
            name: Some(Some(format!("Manual Flush Trace {}", i))),
            metadata: Some(Some(json!({
                "flush_type": "manual",
                "example_feature": "207_multi_status"
            }))),
            ..Default::default()
        };

        let event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf::new(
            format!("event-{}", i),
            chrono::Utc::now().to_rfc3339(),
            trace,
            langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        )));
        batcher.add(event).await?;
        println!("   Added event {}", i);
    }

    println!("\n Performing manual flush...");
    match batcher.flush().await {
        Ok(response) => {
            println!(" Manual flush successful!");
            println!("  - Successfully flushed: {}", response.success_count);
            println!("  - Failed: {}", response.failure_count);

            if response.failure_count > 0 {
                println!("\n    Some events failed:");
                for error in response.failures.iter().take(3) {
                    println!("    - Event {}: {}", error.event_id, error.message);
                    if error.retryable {
                        println!("      (Will be retried automatically)");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!(" Manual flush failed: {}", e);

            // Check if it's a partial failure (207 response)
            if let langfuse_ergonomic::Error::PartialFailure {
                success_count,
                failure_count,
                errors,
                ..
            } = &e
            {
                println!("\n  Partial failure (207 Multi-Status):");
                println!("   Successful: {}", success_count);
                println!("   Failed: {}", failure_count);

                // Note: request_id and retry_after metadata is available
                // in other error types like RateLimit and Client errors

                println!("\n  Failed events will be retried:");
                for error in errors.iter().take(3) {
                    println!("    - {}: {}", error.event_id, error.message);
                    if error.retryable {
                        println!("      Status: Retryable ");
                    } else {
                        println!("      Status: Not retryable ");
                    }
                }
            } else if let langfuse_ergonomic::Error::RateLimit {
                retry_after,
                request_id,
            } = &e
            {
                println!("\n  Rate limited:");
                if let Some(req_id) = request_id {
                    println!("   Request ID: {}", req_id);
                }
                if let Some(retry) = retry_after {
                    println!("    Retry after: {} seconds", retry.as_secs());
                }
            } else if let langfuse_ergonomic::Error::Client {
                request_id: Some(req_id),
                ..
            } = &e
            {
                println!("   Request ID: {}", req_id);
            }
        }
    }

    // Demonstrate backpressure handling
    println!("\n Testing backpressure handling...");
    println!("  Creating a new batcher with DropNew policy and small queue...");

    let client2 = ClientBuilder::from_env()?.build()?;
    let backpressure_batcher = Batcher::builder()
        .client(client2)
        .max_queue_size(3)
        .backpressure_policy(BackpressurePolicy::DropNew)
        .build()
        .await;

    for i in 1..=5 {
        let trace = TraceBody {
            id: Some(Some(format!("backpressure-trace-{}", i))),
            name: Some(Some(format!("Backpressure Test {}", i))),
            ..Default::default()
        };

        let event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf::new(
            format!("event-{}", i),
            chrono::Utc::now().to_rfc3339(),
            trace,
            langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        )));
        match backpressure_batcher.add(event).await {
            Ok(_) => println!("   Event {} queued", i),
            Err(e) => println!("    Event {} dropped: {}", i, e),
        }
    }

    let bp_metrics = backpressure_batcher.metrics();
    println!(
        "   Backpressure test - Queued: {}, Dropped: {}",
        bp_metrics.queued, bp_metrics.dropped
    );

    // Graceful shutdown
    println!("\n Shutting down batchers gracefully...");

    // Get final metrics before shutdown (shutdown consumes self)
    let final_metrics = batcher.metrics();

    // Shutdown main batcher
    match batcher.shutdown().await {
        Ok(response) => {
            println!(" Main batcher shutdown complete:");
            println!("  - Final flush successful: {}", response.success_count);
            println!("  - Final flush failed: {}", response.failure_count);

            println!("\n Final metrics:");
            println!("  - Total flushed: {}", final_metrics.flushed);
            println!("  - Total failed: {}", final_metrics.failed);
            println!("  - Total dropped: {}", final_metrics.dropped);
            println!("  - Total retries: {}", final_metrics.retries);
            if final_metrics.last_error_ts > 0 {
                println!(
                    "  - Last error: {} seconds ago",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - final_metrics.last_error_ts
                );
            }
        }
        Err(e) => eprintln!(" Shutdown failed: {}", e),
    }

    // Shutdown backpressure test batcher
    let _ = backpressure_batcher.shutdown().await;

    println!("\n Batch ingestion example complete!");
    println!("   This example demonstrated:");
    println!("    207 Multi-Status handling for partial failures");
    println!("    Automatic retry with exponential backoff");
    println!("    Size and count-based auto-chunking");
    println!("    Backpressure policies (Block, DropNew, DropOldest)");
    println!("    Comprehensive metrics tracking");
    println!("    Graceful shutdown with guarantees");

    Ok(())
}
