//! Example showing trace creation with full metadata

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let client = LangfuseClient::from_env()?;
    println!("Connected to Langfuse");

    // Create a trace with all available metadata fields
    let trace = client
        .trace()
        .name("fully-annotated-trace")
        .input(json!({
            "system": "You are a helpful assistant",
            "user_message": "Explain quantum computing in simple terms"
        }))
        .output(json!({
            "response": "Quantum computing uses quantum bits (qubits) that can be both 0 and 1 simultaneously...",
            "model": "gpt-4",
            "latency_ms": 1250
        }))
        .user_id("user-42")
        .session_id("session-2024-001")
        .tags(["quantum", "education", "explanation"])
        .metadata(json!({
            "client_version": "0.1.0",
            "environment": "production",
            "feature_flags": {
                "streaming": false,
                "cache_enabled": true
            },
            "request_source": "web_app"
        }))
        .release("v1.2.3")
        .version("2024.01.15")
        .public(false)
        .send()
        .await?;

    println!("✅ Created trace with full metadata");
    println!("   Trace ID: {}", trace.id);
    println!("   View at: https://cloud.langfuse.com/traces/{}", trace.id);

    Ok(())
}
