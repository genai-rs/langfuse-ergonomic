//! Example demonstrating advanced features like prompt management, dataset items, and observation updates

use anyhow::Result;
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let client = LangfuseClient::from_env()?;

    // ===== PROMPT MANAGEMENT =====
    println!("Testing Prompt Management...");

    // Create a text prompt
    let text_prompt = client
        .create_prompt()
        .name("greeting-prompt")
        .prompt("Hello {{name}}! Welcome to {{place}}.")
        .config(json!({
            "temperature": 0.7,
            "max_tokens": 100
        }))
        .labels(vec!["production".to_string()])
        .tags(vec!["greeting".to_string(), "welcome".to_string()])
        .call()
        .await?;
    println!("Created text prompt: {:?}", text_prompt);

    // Create a chat prompt
    let chat_prompt = client
        .create_chat_prompt()
        .name("chat-assistant")
        .messages(vec![
            json!({
                "role": "system",
                "content": "You are a helpful assistant."
            }),
            json!({
                "role": "user",
                "content": "{{user_message}}"
            }),
        ])
        .config(json!({
            "model": "gpt-4",
            "temperature": 0.5
        }))
        .labels(vec!["v2".to_string()])
        .tags(vec!["chat".to_string()])
        .call()
        .await?;
    println!("Created chat prompt: {:?}", chat_prompt);

    // Update prompt version labels
    let updated = client
        .update_prompt_version()
        .name("greeting-prompt")
        .version(1)
        .labels(vec!["production".to_string(), "stable".to_string()])
        .call()
        .await?;
    println!("Updated prompt version: {:?}", updated);

    // ===== DATASET ITEM OPERATIONS =====
    println!("\nTesting Dataset Items...");

    // First create a dataset
    let dataset_name = format!("test-dataset-{}", Uuid::new_v4());
    let dataset = client
        .create_dataset()
        .name(&dataset_name)
        .description("Test dataset for examples")
        .metadata(json!({
            "source": "example",
            "version": "1.0"
        }))
        .call()
        .await?;
    println!("Created dataset: {:?}", dataset);

    // Add items to the dataset
    let item1 = client
        .create_dataset_item()
        .dataset_name(&dataset_name)
        .input(json!({
            "question": "What is the capital of France?"
        }))
        .expected_output(json!({
            "answer": "Paris"
        }))
        .metadata(json!({
            "difficulty": "easy",
            "category": "geography"
        }))
        .call()
        .await?;
    println!("Created dataset item 1: {:?}", item1);

    let item2 = client
        .create_dataset_item()
        .dataset_name(&dataset_name)
        .input(json!({
            "question": "Explain quantum computing"
        }))
        .expected_output(json!({
            "answer": "Quantum computing uses quantum mechanical phenomena..."
        }))
        .metadata(json!({
            "difficulty": "hard",
            "category": "physics"
        }))
        .call()
        .await?;
    println!("Created dataset item 2: {:?}", item2);

    // List dataset items
    let items = client
        .list_dataset_items()
        .dataset_name(&dataset_name)
        .limit(10)
        .call()
        .await?;
    println!("Listed dataset items: {:?}", items);

    // ===== OBSERVATION UPDATES =====
    println!("\nTesting Observation Updates...");

    // First create a trace with observations
    let trace_id = Uuid::new_v4().to_string();
    let trace = client
        .trace()
        .id(&trace_id)
        .name("update-example")
        .input(json!({"test": "data"}))
        .call()
        .await?;
    println!("Created trace: {}", trace.id);

    // Create a span
    let span_id = Uuid::new_v4().to_string();
    let span = client
        .span()
        .id(&span_id)
        .trace_id(&trace_id)
        .name("initial-span")
        .input(json!({"step": 1}))
        .call()
        .await?;
    println!("Created span: {}", span);

    // Update the span
    let updated_span = client
        .update_span()
        .id(&span_id)
        .trace_id(&trace_id)
        .name("updated-span")
        .output(json!({"result": "completed"}))
        .metadata(json!({"updated": true}))
        .status_message("Span successfully completed".to_string())
        .call()
        .await?;
    println!("Updated span: {}", updated_span);

    // Create a generation
    let gen_id = Uuid::new_v4().to_string();
    let generation = client
        .generation()
        .id(&gen_id)
        .trace_id(&trace_id)
        .name("initial-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Hello"}))
        .call()
        .await?;
    println!("Created generation: {}", generation);

    // Update the generation
    let updated_gen = client
        .update_generation()
        .id(&gen_id)
        .trace_id(&trace_id)
        .name("updated-generation")
        .output(json!({"response": "Hello! How can I help?"}))
        .metadata(json!({"tokens": 10}))
        .call()
        .await?;
    println!("Updated generation: {}", updated_gen);

    // Get observations
    let observations = client
        .get_observations()
        .trace_id(&trace_id)
        .limit(10)
        .call()
        .await?;
    println!("Retrieved observations: {:?}", observations);

    println!("\nAll advanced features tested successfully!");

    Ok(())
}
