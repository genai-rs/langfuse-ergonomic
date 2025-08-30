//! Example showing how to connect to a self-hosted Langfuse instance

use chrono::Utc;
use langfuse_ergonomic::LangfuseClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    // Example 1: Connect to self-hosted instance with custom configuration
    let client = LangfuseClient::builder()
        .public_key("your-public-key")
        .secret_key("your-secret-key")
        .base_url("https://langfuse.your-domain.com".to_string()) // Your self-hosted URL
        .timeout(Duration::from_secs(30)) // Custom timeout for slower connections
        .connect_timeout(Duration::from_secs(5)) // Custom connection timeout
        .user_agent("my-app/1.0.0".to_string()) // Custom user agent
        .build();

    // Validate the connection
    match client.validate().await {
        Ok(true) => println!("Successfully connected to self-hosted Langfuse"),
        Ok(false) => println!("Connection established but validation failed"),
        Err(e) => println!("Failed to connect: {}", e),
    }

    // Example 2: Using environment variables for self-hosted instance
    // Set these environment variables:
    // LANGFUSE_PUBLIC_KEY=your-public-key
    // LANGFUSE_SECRET_KEY=your-secret-key
    // LANGFUSE_BASE_URL=https://langfuse.your-domain.com

    let env_client = match LangfuseClient::from_env() {
        Ok(client) => {
            println!("Successfully created client from environment variables");
            client
        }
        Err(e) => {
            println!("Failed to create client from env: {}", e);
            println!("Using default cloud instance for demo");
            LangfuseClient::builder()
                .public_key("demo-public-key")
                .secret_key("demo-secret-key")
                .build()
        }
    };

    // Create a trace to test the connection
    let trace_response = env_client
        .trace()
        .name("self-hosted-test")
        .metadata(serde_json::json!({
            "instance_type": "self-hosted",
            "test_timestamp": Utc::now()
        }))
        .tags(vec![
            "self-hosted".to_string(),
            "connection-test".to_string(),
        ])
        .input(serde_json::json!({
            "test": "Testing connection to self-hosted instance"
        }))
        .output(serde_json::json!({
            "status": "success",
            "message": "Connection established"
        }))
        .call()
        .await?;

    println!("Created trace: {}", trace_response.id);

    // Example 3: Using a batcher with self-hosted instance
    use langfuse_ergonomic::Batcher;

    let batcher = Batcher::builder()
        .client(env_client)
        .max_events(20) // Lower limit for self-hosted instances
        .flush_interval(Duration::from_secs(10)) // Longer interval
        .max_retries(5) // More retries for potentially unstable connections
        .build()
        .await;

    println!("Batcher configured for self-hosted instance");

    // Add events to the batcher
    use langfuse_client_base::models::{
        ingestion_event_one_of::Type, IngestionEvent, IngestionEventOneOf, TraceBody,
    };
    use uuid::Uuid;

    for i in 0..5 {
        let trace_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();

        let trace_event = IngestionEvent::IngestionEventOneOf(Box::new(IngestionEventOneOf {
            id: trace_id.clone(),
            timestamp: timestamp.clone(),
            r#type: Type::TraceCreate,
            body: Box::new(TraceBody {
                id: Some(Some(trace_id.clone())),
                timestamp: Some(Some(timestamp)),
                name: Some(Some(format!("self-hosted-trace-{}", i))),
                metadata: Some(Some(serde_json::json!({
                    "index": i,
                    "instance": "self-hosted"
                }))),
                tags: Some(Some(vec!["batch".to_string(), "self-hosted".to_string()])),
                environment: None,
                ..Default::default()
            }),
            metadata: None,
        }));

        batcher.add(trace_event).await?;
    }

    println!("Added 5 traces to batcher");

    // Flush and shutdown
    let response = batcher.flush().await?;
    println!(
        "Flush complete: {} succeeded, {} failed",
        response.success_count, response.failure_count
    );

    let final_response = batcher.shutdown().await?;
    println!(
        "Batcher shutdown: {} total succeeded, {} total failed",
        final_response.success_count, final_response.failure_count
    );

    println!("\nSelf-hosted configuration tips:");
    println!("1. Ensure your Langfuse instance is accessible from this network");
    println!("2. Use HTTPS in production for security");
    println!("3. Configure appropriate timeouts based on your network latency");
    println!("4. Monitor batch sizes if your instance has different limits");
    println!("5. Consider implementing health checks for high-availability setups");

    Ok(())
}
