//! HTTP middleware with retry support using tower layers.
//!
//! The HTTP client is a `timvw_rust_runtime::Client`: a tower service (reqwest by default)
//! that takes tower layers. This example adds `tower::retry::RetryLayer` with the runtime's
//! `RetryPolicy` (429, 408, 5xx and failed connections, exponential backoff, `Retry-After`)
//! and a `TimeoutLayer`.
//!
//! The layer approach allows you to:
//! - Automatically retry transient errors (network failures, 429, 5xx)
//! - Add timeouts, rate limits, logging (`TraceLayer`, feature `tracing-http`) or metrics
//! - Compose multiple layers
//!
//! Run with: `cargo run --example http_middleware_retry --features retry`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::{ClientBuilder, Result};
use serde_json::json;
use timvw_rust_runtime::{retry::RetryPolicy, Client, ReqwestService};
use tower::{retry::RetryLayer, timeout::TimeoutLayer};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== HTTP Middleware with Retry Example ===\n");

    // Example 1: Basic client with retry middleware
    println!("1. Creating Langfuse client with retry middleware\n");

    // Retry transient errors up to 3 times with exponential delays
    let http_client = Client::from_service(ReqwestService::new(reqwest::Client::new()))
        .layer(RetryLayer::new(RetryPolicy::new(3)));

    // Create Langfuse client with custom HTTP client
    let client = ClientBuilder::from_env()?
        .http_client(http_client)
        .build()?;

    // Use the client normally - retries are handled automatically
    println!("Creating trace (retries are automatic)...");

    match client
        .trace()
        .name("middleware-example")
        .input(json!({"query": "Hello, world!"}))
        .output(json!({"response": "Hi there!"}))
        .tags(vec!["middleware".to_string(), "retry".to_string()])
        .call()
        .await
    {
        Ok(trace) => {
            println!("\nSuccess! Trace created: {}", trace.id);
        }
        Err(e) => {
            eprintln!("\nError after retries: {e}");
        }
    }

    // Example 2: Custom retry policy with more retries and custom delays
    println!("\n2. Creating client with custom retry policy\n");

    // Up to 5 retries, each attempt limited to 20 seconds (a layer added later sees the request
    // first, so the timeout applies to each attempt).
    let custom_http_client = Client::from_service(ReqwestService::new(reqwest::Client::new()))
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(20)))
        .layer(RetryLayer::new(RetryPolicy::new(5)));

    let custom_client = ClientBuilder::from_env()?
        .http_client(custom_http_client)
        .build()?;

    println!("Creating trace with custom retry policy (up to 5 retries)...");

    match custom_client
        .trace()
        .name("custom-retry-example")
        .input(json!({"query": "Test with custom retry policy"}))
        .metadata(json!({
            "retry_config": {
                "max_retries": 5,
                "attempt_timeout_sec": 20
            }
        }))
        .call()
        .await
    {
        Ok(trace) => {
            println!("\nSuccess! Trace created: {}", trace.id);
        }
        Err(e) => {
            eprintln!("\nError after retries: {e}");
        }
    }

    // Example 3: Validate client credentials (will retry on transient failures)
    println!("\n3. Validating client credentials\n");

    match client.validate().await {
        Ok(true) => println!("Credentials validated successfully!"),
        Ok(false) => println!("Credentials validation failed"),
        Err(e) => eprintln!("Error validating credentials: {e}"),
    }

    println!("\n=== Example Complete ===");
    println!("\nKey takeaways:");
    println!("- Tower layers add transparent retry logic");
    println!("- Exponential backoff prevents overwhelming the server");
    println!("- Custom policies can be tailored to your needs");
    println!("- All API calls automatically benefit from retries");

    Ok(())
}
