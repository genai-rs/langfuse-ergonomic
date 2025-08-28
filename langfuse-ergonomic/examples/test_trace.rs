use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Create client from environment variables
    println!("Creating Langfuse client from environment...");
    let client = LangfuseClient::from_env()?;
    
    println!("✅ Client created successfully");
    
    // Create a simple trace
    println!("Creating trace...");
    let trace_response = client.trace()
        .name("test-trace-from-rust")
        .input(json!({
            "prompt": "What is the capital of France?",
            "max_tokens": 50
        }))
        .output(json!({
            "response": "The capital of France is Paris.",
            "tokens_used": 10
        }))
        .tags(["test", "rust-client", "example"])
        .user_id("test-user-123")
        .session_id("test-session-456")
        .metadata(json!({
            "environment": "testing",
            "sdk": "langfuse-rs",
            "version": "0.1.0"
        }))
        .send()
        .await?;
    
    println!("✅ Trace created successfully!");
    println!("   Trace ID: {}", trace_response.id);
    println!("\n🎉 Check your Langfuse dashboard at https://cloud.langfuse.com");
    println!("   You should see a trace named 'test-trace-from-rust'");
    
    Ok(())
}