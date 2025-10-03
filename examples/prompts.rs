//! Example demonstrating prompt management functionality

use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Create client from environment variables
    let client = ClientBuilder::from_env()?.build()?;

    println!(" Prompt Management Example");
    println!("============================");

    // Note: Create functionality is currently a placeholder
    println!("\n1. Creating a prompt (placeholder implementation)...");
    match client
        .create_prompt()
        .name("example-prompt")
        .prompt("You are a helpful AI assistant. Help the user with their query: {{input}}")
        .is_active(true)
        .config(json!({
            "temperature": 0.7,
            "max_tokens": 100
        }))
        .labels(vec!["assistant".to_string(), "helpful".to_string()])
        .tags(vec!["production".to_string()])
        .call()
        .await
    {
        Ok(prompt) => {
            println!(" Prompt result: {}", serde_json::to_string_pretty(&prompt)?);
        }
        Err(e) => {
            println!("  Prompt creation (placeholder): {}", e);
        }
    }

    // Get a specific prompt
    println!("\n2. Getting a prompt by name...");
    match client.get_prompt("example-prompt", None, None).await {
        Ok(prompt) => {
            println!(
                " Retrieved prompt: {}",
                serde_json::to_string_pretty(&prompt)?
            );
        }
        Err(e) => {
            println!("  Could not retrieve prompt: {}", e);
        }
    }

    // List prompts with filters
    println!("\n3. Listing prompts with filters...");
    match client
        .list_prompts()
        .page(1)
        .limit("10".to_string())
        .name("example-prompt")
        .call()
        .await
    {
        Ok(prompts) => {
            println!(
                " Listed prompts: {}",
                serde_json::to_string_pretty(&prompts)?
            );
        }
        Err(e) => {
            println!("  Could not list prompts: {}", e);
        }
    }

    // List all prompts
    println!("\n4. Listing all prompts...");
    match client.list_prompts().call().await {
        Ok(prompts) => {
            println!(" All prompts: {}", serde_json::to_string_pretty(&prompts)?);
        }
        Err(e) => {
            println!("  Could not list all prompts: {}", e);
        }
    }

    println!("\n Prompt management example completed!");
    println!(" Note: Prompt creation is currently using a placeholder implementation.");
    println!("   The actual create functionality depends on the correct API implementation.");

    Ok(())
}
