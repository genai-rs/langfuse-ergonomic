//! Example demonstrating how to read traces back with the Langfuse v4 observations API
//!
//! In Langfuse v4 a trace is read through its observations: the root observation carries the
//! trace name, input and output. `trace_observations()` replaces `get_trace()`, and
//! `observations().is_root_observation(true)` replaces `list_traces()`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    println!(" Trace Fetching Example");
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
            "response": "Read traces through the v4 observations API"
        }))
        .user_id("example-user")
        .tags(vec!["example".to_string(), "fetch-demo".to_string()])
        .call()
        .await?;
    client
        .span()
        .trace_id(&trace.id)
        .name("lookup")
        .call()
        .await?;

    println!(" Created trace with ID: {}", trace.id);

    // Get every observation of the trace
    println!("\n2. Fetching the trace's observations...");
    match client.trace_observations(&trace.id).await {
        Ok(observations) => {
            for observation in &observations {
                println!(
                    "   {} {} {:?} root={:?}",
                    observation.id,
                    observation.r#type,
                    observation.name,
                    observation.is_root_observation
                );
            }
        }
        Err(e) => {
            println!("  Could not fetch observations: {e}");
        }
    }

    // List traces (root observations) with filters
    println!("\n3. Listing traces with filters...");
    let page = client
        .observations()
        .is_root_observation(true)
        .user_id("example-user")
        .name("fetch-example-trace")
        .from_start_time(chrono::Utc::now() - chrono::Duration::hours(1))
        .limit(5)
        .call()
        .await?;
    for root in &page.data {
        println!("   trace {:?}: {:?}", root.trace_id, root.name);
    }

    // Page through recent traces with the cursor
    println!("\n4. Paging through recent traces...");
    let mut cursor = None;
    for page_number in 1..=3 {
        let page = client
            .observations()
            .is_root_observation(true)
            .from_start_time(chrono::Utc::now() - chrono::Duration::days(1))
            .limit(3)
            .maybe_cursor(cursor)
            .call()
            .await?;
        println!("   page {page_number}: {} traces", page.data.len());
        match page.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }

    // Example of trace deletion (commented out to preserve data)
    println!("\n5. Trace deletion example (not executed):");
    println!("   client.delete_trace(\"{}\").await?;", trace.id);
    println!(
        "   client.delete_multiple_traces(vec![\"{}\".to_string()]).await?;",
        trace.id
    );

    println!("\n Trace fetching example completed!");
    Ok(())
}
