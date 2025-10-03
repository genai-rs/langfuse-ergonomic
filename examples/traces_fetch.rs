//! Example demonstrating trace fetching functionality

use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    println!("🔍 Trace Fetching Example");
    println!("========================");

    // First, create a trace to work with
    println!("\n1. Creating a sample trace...");
    let trace = client
        .trace()
        .name("fetch-example-trace")
        .input(json!({
            "user_query": "How does trace fetching work?"
        }))
        .output(json!({
            "response": "You can fetch traces using get_trace and list_traces methods"
        }))
        .user_id("example-user")
        .tags(vec!["example".to_string(), "fetch-demo".to_string()])
        .call()
        .await?;

    println!("✅ Created trace with ID: {}", trace.id);

    // Get the specific trace
    println!("\n2. Fetching the trace by ID...");
    match client.get_trace(&trace.id).await {
        Ok(fetched_trace) => {
            println!(
                "🎯 Fetched trace: {}",
                serde_json::to_string_pretty(&fetched_trace)?
            );
        }
        Err(e) => {
            println!("⚠️  Could not fetch trace: {}", e);
        }
    }

    // List traces with filters
    println!("\n3. Listing traces with filters...");
    let traces = client
        .list_traces()
        .limit(5)
        .page(1)
        .user_id("example-user")
        .name("fetch-example-trace")
        .call()
        .await?;

    println!(
        "📋 Listed traces: {}",
        serde_json::to_string_pretty(&traces)?
    );

    // List recent traces
    println!("\n4. Listing recent traces...");
    let recent_traces = client
        .list_traces()
        .limit(3)
        .order_by("timestamp")
        .call()
        .await?;

    println!(
        "🕒 Recent traces: {}",
        serde_json::to_string_pretty(&recent_traces)?
    );

    // Example of trace deletion (commented out to preserve data)
    println!("\n5. Trace deletion example (not executed):");
    println!("   client.delete_trace(\"{}\").await?;", trace.id);
    println!(
        "   client.delete_multiple_traces(vec![\"{}\".to_string()]).await?;",
        trace.id
    );

    println!("\n✨ Trace fetching example completed!");
    Ok(())
}
