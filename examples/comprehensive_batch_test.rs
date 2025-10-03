//! Comprehensive test demonstrating batching, metrics, and error handling

use chrono::Utc;
use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};
use langfuse_ergonomic::{BackpressurePolicy, Batcher, ClientBuilder};
use std::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    println!("🚀 Starting comprehensive batch test");
    println!("{}", "=".repeat(50));

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;
    println!("✅ Connected to Langfuse");

    // Create a batcher with specific configuration to test various scenarios
    let batcher = Batcher::builder()
        .client(client)
        .max_events(10) // Small batch size to trigger multiple flushes
        .max_bytes(50_000) // Small byte limit to test chunking
        .flush_interval(Duration::from_secs(2))
        .max_retries(3)
        .max_queue_size(100)
        .backpressure_policy(BackpressurePolicy::Block)
        .retry_jitter(true)
        .build()
        .await;

    println!("📊 Batcher configured with:");
    println!("  - Max events per batch: 10");
    println!("  - Max bytes per batch: 50,000");
    println!("  - Flush interval: 2 seconds");
    println!("  - Max retries: 3");
    println!("  - Max queue size: 100");
    println!("  - Backpressure policy: Block");
    println!("  - Retry jitter: Enabled");

    // Get initial metrics
    let initial_metrics = batcher.metrics();
    println!("\n📈 Initial metrics:");
    println!("  - Events queued: {}", initial_metrics.queued);
    println!("  - Events flushed: {}", initial_metrics.flushed);
    println!("  - Events failed: {}", initial_metrics.failed);
    println!("  - Events dropped: {}", initial_metrics.dropped);

    println!("\n{}", "=".repeat(50));
    println!("Creating test events...\n");

    // Note: session_id and user_id are NOT secrets in Langfuse context
    // They are identifiers for organizing/filtering traces, not authentication credentials
    let session_id = format!("batch-test-session-{}", Uuid::new_v4());
    let user_id = "test-user-batch";

    // Safe to log: these are trace organization identifiers, not secrets
    // codeql[rust/cleartext-logging] - False positive: session_id is not a secret
    println!("📁 Session ID: {}", session_id);
    // codeql[rust/cleartext-logging] - False positive: user_id is not a secret
    println!("👤 User ID: {}\n", user_id);

    // Create multiple traces to test batching
    let mut trace_ids = Vec::new();

    for i in 0..25 {
        let trace_id = Uuid::new_v4().to_string();
        trace_ids.push(trace_id.clone());

        // Create different types of traces to test various scenarios
        let trace_type = match i % 5 {
            0 => "llm-chat",
            1 => "api-call",
            2 => "database-query",
            3 => "background-job",
            _ => "user-interaction",
        };

        let trace_event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf {
            id: trace_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
            body: Box::new(TraceBody {
                id: Some(Some(trace_id.clone())),
                timestamp: Some(Some(Utc::now().to_rfc3339())),
                name: Some(Some(format!("{}-trace-{}", trace_type, i))),
                user_id: Some(Some(user_id.to_string())),
                session_id: Some(Some(session_id.clone())),
                metadata: Some(Some(serde_json::json!({
                    "test_type": "comprehensive_batch",
                    "trace_index": i,
                    "trace_type": trace_type,
                    "batch_test": true,
                    "features_tested": ["batching", "chunking", "retries", "metrics"],
                    "timestamp": Utc::now().to_rfc3339(),
                    "test_data": {
                        "importance": if i < 5 { "critical" } else if i < 15 { "normal" } else { "low" },
                        "processing_time_ms": 100 + i * 10,
                        "data_size_bytes": 1024 * (i + 1)
                    }
                }))),
                release: Some(Some("v2.0.0".to_string())),
                version: Some(Some("2.0.0".to_string())),
                tags: Some(Some(vec![
                    "batch-test".to_string(),
                    "comprehensive".to_string(),
                    trace_type.to_string(),
                    format!(
                        "priority-{}",
                        if i < 5 {
                            "high"
                        } else if i < 15 {
                            "medium"
                        } else {
                            "low"
                        }
                    ),
                ])),
                input: Some(Some(serde_json::json!({
                    "query": format!("Test query for {} #{}", trace_type, i),
                    "params": {
                        "index": i,
                        "type": trace_type,
                        "batch_size": 10,
                        "test_mode": true
                    },
                    "context": {
                        "session": session_id.clone(),
                        "user": user_id,
                        "environment": "test"
                    }
                }))),
                output: Some(Some(serde_json::json!({
                    "status": "success",
                    "result": format!("Processed {} request #{}", trace_type, i),
                    "metrics": {
                        "duration_ms": 100 + i * 10,
                        "tokens_used": if trace_type == "llm-chat" { Some(150 + i * 5) } else { None },
                        "cache_hit": i % 3 == 0
                    },
                    "data": {
                        "response_size": 2048 * (i % 5 + 1),
                        "items_processed": i * 2 + 1
                    }
                }))),
                public: Some(Some(false)),
                environment: None,
            }),
            metadata: None,
        }));

        batcher.add(trace_event).await?;

        // Print progress for every 5 traces
        if (i + 1) % 5 == 0 {
            println!("  ✅ Added {} traces (latest: {})", i + 1, trace_type);

            // Check metrics periodically
            let current_metrics = batcher.metrics();
            println!(
                "     📊 Current: queued={}, flushed={}",
                current_metrics.queued, current_metrics.flushed
            );
        }

        // Add small delays to simulate real-world timing
        if i % 3 == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("📊 Checking metrics after adding all events...");

    let after_add_metrics = batcher.metrics();
    println!("  - Total events queued: {}", after_add_metrics.queued);
    println!("  - Events flushed so far: {}", after_add_metrics.flushed);
    println!("  - Events failed: {}", after_add_metrics.failed);
    println!("  - Events dropped: {}", after_add_metrics.dropped);
    println!("  - Total retries: {}", after_add_metrics.retries);

    // Wait for automatic flush
    println!("\n⏳ Waiting for automatic flush (3 seconds)...");
    tokio::time::sleep(Duration::from_secs(4)).await;

    let after_auto_flush_metrics = batcher.metrics();
    println!("📊 Metrics after automatic flush:");
    println!(
        "  - Total events queued: {}",
        after_auto_flush_metrics.queued
    );
    println!("  - Events flushed: {}", after_auto_flush_metrics.flushed);
    println!("  - Events failed: {}", after_auto_flush_metrics.failed);
    println!("  - Events dropped: {}", after_auto_flush_metrics.dropped);
    println!("  - Total retries: {}", after_auto_flush_metrics.retries);

    // Manual flush to ensure all events are sent
    println!("\n🔄 Performing manual flush...");
    let flush_response = batcher.flush().await?;
    println!("✅ Manual flush complete:");
    println!("  - Succeeded: {}", flush_response.success_count);
    println!("  - Failed: {}", flush_response.failure_count);

    if !flush_response.failures.is_empty() {
        println!("⚠️  Failures detected:");
        for failure in &flush_response.failures {
            println!(
                "    - Event {}: {} (retryable: {})",
                failure.event_id, failure.message, failure.retryable
            );
        }
    }

    // Test adding more events to see if buffer continues working
    println!("\n📝 Adding 5 more traces to test continued operation...");
    for i in 25..30 {
        let trace_id = Uuid::new_v4().to_string();
        let trace_event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf {
            id: trace_id.clone(),
            timestamp: Utc::now().to_rfc3339(),
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
            body: Box::new(TraceBody {
                id: Some(Some(trace_id.clone())),
                timestamp: Some(Some(Utc::now().to_rfc3339())),
                name: Some(Some(format!("additional-trace-{}", i))),
                user_id: Some(Some(user_id.to_string())),
                session_id: Some(Some(session_id.clone())),
                metadata: Some(Some(serde_json::json!({
                    "test_phase": "additional",
                    "trace_index": i
                }))),
                tags: Some(Some(vec![
                    "additional".to_string(),
                    "post-flush".to_string(),
                ])),
                input: Some(Some(serde_json::json!({ "additional": true, "index": i }))),
                output: Some(Some(serde_json::json!({ "processed": true }))),
                release: Some(Some("v2.0.0".to_string())),
                version: Some(Some("2.0.0".to_string())),
                public: Some(Some(false)),
                environment: None,
            }),
            metadata: None,
        }));
        batcher.add(trace_event).await?;
    }
    println!("  ✅ Added 5 additional traces");

    // Get final metrics before shutdown (shutdown consumes the batcher)
    let final_metrics = batcher.metrics();

    // Graceful shutdown
    println!("\n🛑 Shutting down batcher...");
    let shutdown_response = batcher.shutdown().await?;
    println!("✅ Shutdown complete:");
    println!("  - Total succeeded: {}", shutdown_response.success_count);
    println!("  - Total failed: {}", shutdown_response.failure_count);
    println!("\n📊 Final metrics summary:");
    println!("  - Total events queued: {}", final_metrics.queued);
    println!("  - Total events flushed: {}", final_metrics.flushed);
    println!("  - Total events failed: {}", final_metrics.failed);
    println!("  - Total events dropped: {}", final_metrics.dropped);
    println!("  - Total retries: {}", final_metrics.retries);

    let success_rate = if final_metrics.queued > 0 {
        (final_metrics.flushed as f64 / final_metrics.queued as f64) * 100.0
    } else {
        0.0
    };
    println!("  - Success rate: {:.2}%", success_rate);

    if final_metrics.last_error_ts > 0 {
        let error_time = std::time::UNIX_EPOCH + Duration::from_secs(final_metrics.last_error_ts);
        if let Ok(error_time) = error_time.duration_since(std::time::UNIX_EPOCH) {
            println!("  - Last error: {} seconds ago", error_time.as_secs());
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("🎉 Test complete!");
    println!("\n📌 View results at:");
    println!(
        "   Session: https://cloud.langfuse.com/sessions/{}",
        session_id
    );
    if !trace_ids.is_empty() {
        println!(
            "   First trace: https://cloud.langfuse.com/trace/{}",
            trace_ids[0]
        );
        println!(
            "   Last trace: https://cloud.langfuse.com/trace/{}",
            trace_ids[trace_ids.len() - 1]
        );
    }
    println!("\n💡 Login with:");
    println!("   Email: langfuse@timvw.be");
    println!("   URL: https://cloud.langfuse.com");

    Ok(())
}
