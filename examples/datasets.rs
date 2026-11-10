//! Example demonstrating dataset management functionality
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::{ApiVersion, ClientBuilder};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    println!("  Dataset Management Example");
    println!("=============================");

    // Create a dataset
    println!("\n1. Creating a dataset...");
    let dataset = client
        .create_dataset()
        .name("example-dataset")
        .description("An example dataset for testing")
        .metadata(json!({
            "created_by": "example",
            "purpose": "testing"
        }))
        .input_schema(json!({
            "type": "object",
            "properties": {
                "prompt": {"type": "string"},
                "context": {"type": "string"}
            }
        }))
        .expected_output_schema(json!({
            "type": "object",
            "properties": {
                "answer": {"type": "string"},
                "confidence": {"type": "number"}
            }
        }))
        .call()
        .await?;

    println!(
        " Created dataset: {}",
        serde_json::to_string_pretty(&dataset)?
    );

    // List all datasets
    println!("\n2. Listing datasets...");
    let datasets = client.list_datasets().page(1).limit(10).call().await?;

    println!(" Datasets: {}", serde_json::to_string_pretty(&datasets)?);

    // Get a specific dataset
    println!("\n3. Getting specific dataset...");
    match client.get_dataset("example-dataset").await {
        Ok(dataset) => {
            println!(
                " Dataset details: {}",
                serde_json::to_string_pretty(&dataset)?
            );
        }
        Err(e) => {
            println!("  Could not retrieve dataset: {e}");
        }
    }

    // Get dataset runs. Langfuse v4 replaces dataset runs with experiments, so this endpoint
    // only exists on Langfuse v3 servers.
    println!("\n4. Getting dataset runs...");
    if client.api_version() == ApiVersion::V3 {
        #[allow(deprecated)]
        let runs = client.get_dataset_runs("example-dataset").await;
        match runs {
            Ok(runs) => {
                println!(" Dataset runs: {}", serde_json::to_string_pretty(&runs)?);
            }
            Err(e) => {
                println!("  Could not retrieve dataset runs: {e}");
            }
        }
    } else {
        println!("  Skipped: dataset runs are a Langfuse v3 feature (v4 uses experiments)");
    }

    println!("\n Dataset management example completed!");
    Ok(())
}
