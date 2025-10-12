//! HTTP middleware with retry support using reqwest-middleware.
//!
//! This example demonstrates how to use `reqwest-middleware` with `reqwest-retry`
//! to add automatic retry capabilities with exponential backoff to the Langfuse client.
//!
//! The middleware approach allows you to:
//! - Automatically retry transient errors (network failures, timeouts, 5xx errors)
//! - Configure exponential backoff strategies
//! - Add custom middleware for logging, metrics, or other cross-cutting concerns
//! - Compose multiple middleware layers
//!
//! Run with: `cargo run --example http_middleware_retry`

use langfuse_ergonomic::{ClientBuilder, Result};
use reqwest_middleware::ClientBuilder as MiddlewareClientBuilder;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== HTTP Middleware with Retry Example ===\n");

    // Example 1: Basic client with retry middleware
    println!("1. Creating Langfuse client with retry middleware\n");

    // Create a retry policy with exponential backoff
    // This will retry transient errors up to 3 times with exponential delays
    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);

    // Build an HTTP client with retry middleware
    let http_client = MiddlewareClientBuilder::new(reqwest::Client::new())
        .with(RetryTransientMiddleware::new_with_policy(retry_policy))
        .build();

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
            println!("\n✅ Success! Trace created: {}", trace.id);
        }
        Err(e) => {
            eprintln!("\n❌ Error after retries: {e}");
        }
    }

    // Example 2: Custom retry policy with more retries and custom delays
    println!("\n2. Creating client with custom retry policy\n");

    let custom_retry_policy = ExponentialBackoff::builder()
        .retry_bounds(
            std::time::Duration::from_millis(100), // minimum delay
            std::time::Duration::from_secs(30),    // maximum delay
        )
        .build_with_max_retries(5); // up to 5 retries

    let custom_http_client = MiddlewareClientBuilder::new(
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("Failed to build reqwest client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        custom_retry_policy,
    ))
    .build();

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
                "min_delay_ms": 100,
                "max_delay_sec": 30
            }
        }))
        .call()
        .await
    {
        Ok(trace) => {
            println!("\n✅ Success! Trace created: {}", trace.id);
        }
        Err(e) => {
            eprintln!("\n❌ Error after retries: {e}");
        }
    }

    // Example 3: Validate client credentials (will retry on transient failures)
    println!("\n3. Validating client credentials\n");

    match client.validate().await {
        Ok(true) => println!("✅ Credentials validated successfully!"),
        Ok(false) => println!("❌ Credentials validation failed"),
        Err(e) => eprintln!("❌ Error validating credentials: {e}"),
    }

    println!("\n=== Example Complete ===");
    println!("\nKey takeaways:");
    println!("- Middleware allows transparent retry logic");
    println!("- Exponential backoff prevents overwhelming the server");
    println!("- Custom policies can be tailored to your needs");
    println!("- All API calls automatically benefit from retries");

    Ok(())
}
