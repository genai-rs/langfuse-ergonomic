//! Basic usage example for the Langfuse Rust client

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;
use std::env;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();
    
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Get credentials from environment
    let public_key = env::var("LANGFUSE_PUBLIC_KEY")
        .expect("LANGFUSE_PUBLIC_KEY environment variable not set");
    let secret_key = env::var("LANGFUSE_SECRET_KEY")
        .expect("LANGFUSE_SECRET_KEY environment variable not set");
    let base_url = env::var("LANGFUSE_BASE_URL")
        .unwrap_or_else(|_| "https://cloud.langfuse.com".to_string());
    
    // Create client using builder pattern
    let client = LangfuseClient::new()
        .public_key(public_key)
        .secret_key(secret_key)
        .base_url(base_url)
        .call();
    
    info!("Created Langfuse client");
    
    // Validate credentials
    if client.validate().await? {
        info!("✅ Credentials validated successfully");
    }
    
    // Example 1: Create a simple trace
    let trace_response = client.trace()
        .name("example-trace")
        .input(json!({
            "prompt": "What is the capital of France?",
            "max_tokens": 50
        }))
        .output(json!({
            "response": "The capital of France is Paris.",
            "tokens_used": 10
        }))
        .tags(["example", "question-answering"])
        .user_id("user-123")
        .session_id("session-456")
        .metadata(json!({
            "environment": "development",
            "version": "1.0.0"
        }))
        .send()
        .await?;
    
    info!("Created trace with ID: {}", trace_response.id);
    let trace_id = trace_response.id.clone();
    
    // Example 2: Create a span within the trace
    let span_id = "span-001";
    client.span(&trace_id)
        .id(span_id)
        .name("preprocessing")
        .input(json!({
            "raw_prompt": "What is the capital of France?"
        }))
        .output(json!({
            "processed_prompt": "Question: What is the capital of France? Answer:"
        }))
        .level("INFO")
        .send()
        .await?;
    
    info!("Created span: {}", span_id);
    
    // Example 3: Create a generation observation
    client.generation(&trace_id)
        .parent_observation_id(span_id)
        .name("llm-generation")
        .model("gpt-3.5-turbo")
        .model_parameters(json!({
            "temperature": 0.7,
            "max_tokens": 50
        }))
        .input(json!({
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is the capital of France?"}
            ]
        }))
        .output(json!({
            "content": "The capital of France is Paris.",
            "finish_reason": "stop"
        }))
        .tokens(15, 10)  // prompt tokens, completion tokens
        .send()
        .await?;
    
    info!("Created generation observation");
    
    // Example 4: Add scores to evaluate the trace
    
    // Binary score for correctness
    client.binary_score(&trace_id, "correctness", true)
        .comment("The answer is factually correct")
        .send()
        .await?;
    
    // Rating score for quality
    client.rating_score(&trace_id, "quality", 4, 5)
        .comment("Good answer, could be more detailed")
        .send()
        .await?;
    
    // Categorical score for language
    client.categorical_score(&trace_id, "detected_language", "English")
        .send()
        .await?;
    
    info!("Added scores to trace");
    
    // Example 5: Create an event
    client.event(&trace_id)
        .name("cache-hit")
        .level("INFO")
        .metadata(json!({
            "cache_key": "capital_france",
            "ttl": 3600
        }))
        .send()
        .await?;
    
    info!("Created event");
    
    println!("\n✅ All examples completed successfully!");
    println!("Check your Langfuse dashboard to see the created traces and observations.");
    
    Ok(())
}