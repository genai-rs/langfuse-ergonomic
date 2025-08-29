//! Example demonstrating batch ingestion with 207 Multi-Status handling
//!
//! This example shows how to:
//! - Use the batcher for efficient event ingestion
//! - Handle partial failures (207 responses)
//! - Retry failed events automatically
//! - Configure batch size and flush intervals

use langfuse_ergonomic::{BatcherConfig, LangfuseClient, IngestionResponse};
use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize from environment variables
    dotenvy::dotenv().ok();
    
    let client = Arc::new(LangfuseClient::from_env()?);
    
    // Create a batcher with custom configuration
    let config = BatcherConfig {
        max_events: 50,                           // Batch up to 50 events
        max_bytes: 2_000_000,                     // Or up to 2MB
        flush_interval: Duration::from_secs(5),   // Auto-flush every 5 seconds
        max_retries: 3,                           // Retry failed events up to 3 times
        fail_fast: false,                         // Continue on partial failures
        ..Default::default()
    };
    let batcher = client.clone().create_batcher(Some(config));
    
    println!("🚀 Starting batch ingestion example...");
    
    // Simulate sending multiple events
    for i in 1..=10 {
        let trace_body = TraceBody {
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
                "version": env!("CARGO_PKG_VERSION")
            }))),
            user_id: Some(Some("batch-user".to_string())),
            session_id: Some(Some(format!("batch-session-{}", i % 3))),
            tags: Some(Some(vec![
                "batch".to_string(),
                format!("group-{}", i % 2)
            ])),
            ..Default::default()
        };
        
        let event = IngestionEventOneOf {
            body: Box::new(trace_body),
            id: format!("event-{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        };
        
        let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
        
        // Add to batch
        match batcher.add(ingestion_event).await {
            Ok(_) => println!("  ✅ Added event {} to batch", i),
            Err(e) => eprintln!("  ❌ Failed to add event {}: {}", i, e),
        }
        
        // Small delay to simulate real-world event generation
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    println!("\n📤 Flushing batch...");
    
    // Manually flush the batch (also happens automatically on interval)
    match batcher.flush().await {
        Ok(response) => {
            print_ingestion_response(&response);
        }
        Err(e) => {
            eprintln!("❌ Flush failed: {}", e);
            
            // Check if it's a partial failure
            if let langfuse_ergonomic::Error::PartialFailure { 
                success_count, 
                failure_count, 
                errors, 
                .. 
            } = e {
                println!("\n⚠️  Partial failure detected:");
                println!("  ✅ Successful: {}", success_count);
                println!("  ❌ Failed: {}", failure_count);
                
                if !errors.is_empty() {
                    println!("\n  Failed events:");
                    for error in errors.iter().take(5) {
                        println!("    - {}: {} {}", 
                            error.event_id, 
                            error.message,
                            if error.retryable { "[retryable]" } else { "" }
                        );
                    }
                    if errors.len() > 5 {
                        println!("    ... and {} more", errors.len() - 5);
                    }
                }
            }
        }
    }
    
    // Simulate more events being added while the batcher is running
    println!("\n🔄 Adding more events (will auto-flush)...");
    
    for i in 11..=15 {
        let trace_body = TraceBody {
            id: Some(Some(format!("auto-trace-{}", i))),
            name: Some(Some(format!("Auto-flush Trace {}", i))),
            ..Default::default()
        };
        
        let event = IngestionEventOneOf {
            body: Box::new(trace_body),
            id: format!("auto-event-{}", uuid::Uuid::new_v4()),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        };
        
        let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
        batcher.add(ingestion_event).await?;
        println!("  ➕ Added event {}", i);
    }
    
    // Wait for auto-flush
    println!("\n⏳ Waiting for auto-flush (5 seconds)...");
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    // Shutdown the batcher and get final results
    println!("\n🛑 Shutting down batcher...");
    match batcher.shutdown().await {
        Ok(response) => {
            println!("Final flush results:");
            print_ingestion_response(&response);
        }
        Err(e) => eprintln!("❌ Shutdown flush failed: {}", e),
    }
    
    println!("\n✨ Batch ingestion example complete!");
    
    Ok(())
}

/// Helper to print ingestion response details
fn print_ingestion_response(response: &IngestionResponse) {
    println!("\n📊 Ingestion Results:");
    println!("  ✅ Successful: {}", response.success_count);
    println!("  ❌ Failed: {}", response.failure_count);
    
    if response.success_count > 0 {
        println!("\n  Successfully ingested event IDs:");
        for id in response.success_ids.iter().take(5) {
            println!("    - {}", id);
        }
        if response.success_ids.len() > 5 {
            println!("    ... and {} more", response.success_ids.len() - 5);
        }
    }
    
    if response.failure_count > 0 {
        println!("\n  Failed events:");
        for error in response.failures.iter().take(5) {
            println!("    - {}: {} {}", 
                error.event_id, 
                error.message,
                if error.retryable { "[retryable]" } else { "" }
            );
        }
        if response.failures.len() > 5 {
            println!("    ... and {} more", response.failures.len() - 5);
        }
    }
}