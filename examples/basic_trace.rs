//! Basic example showing minimal trace creation

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Create client from environment variables
    let client = LangfuseClient::from_env()?;
    println!("✅ Connected to Langfuse");

    // Create a minimal trace
    let trace = client.trace().name("minimal-trace").send().await?;

    println!("Created minimal trace: {}", trace.id);

    // Create a trace with input/output
    let trace_with_io = client
        .trace()
        .name("trace-with-io")
        .input(json!({
            "question": "What is 2 + 2?"
        }))
        .output(json!({
            "answer": 4,
            "confidence": 1.0
        }))
        .send()
        .await?;

    println!("Created trace with I/O: {}", trace_with_io.id);

    Ok(())
}
