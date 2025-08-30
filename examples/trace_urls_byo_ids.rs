//! Example demonstrating Trace URLs and Bring Your Own IDs
//!
//! This example shows how to:
//! - Get Langfuse URLs for created traces
//! - Use custom/deterministic IDs for traces and observations
//! - Generate reproducible IDs from seeds
//! - Create hierarchical ID structures

use langfuse_ergonomic::{IdGenerator, LangfuseClient};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize from environment variables
    dotenvy::dotenv().ok();

    let client = LangfuseClient::from_env()?;

    println!("🔗 Demonstrating Trace URLs and custom IDs...\n");

    // Example 1: Create a trace with auto-generated ID and get its URL
    println!("1️⃣ Creating trace with auto-generated ID:");
    let response = client
        .trace()
        .name("Auto ID Trace")
        .input(json!({
            "message": "This trace has an auto-generated ID"
        }))
        .tags(vec!["auto-id".to_string()])
        .call()
        .await?;

    println!("   Trace ID: {}", response.id);
    println!("   Trace URL: {}", response.url());
    println!();

    // Example 2: Create a trace with a custom ID
    println!("2️⃣ Creating trace with custom ID:");
    let custom_id = "my-custom-trace-12345";
    let response = client
        .trace()
        .id(custom_id.to_string())
        .name("Custom ID Trace")
        .input(json!({
            "message": "This trace has a custom ID"
        }))
        .tags(vec!["custom-id".to_string()])
        .call()
        .await?;

    println!("   Trace ID: {}", response.id);
    println!("   Trace URL: {}", response.url());
    println!();

    // Example 3: Generate deterministic IDs from seeds
    println!("3️⃣ Generating deterministic IDs from seeds:");

    let seed = "user-123:session-456:request-789";
    let deterministic_id = IdGenerator::from_seed(seed);

    println!("   Seed: {}", seed);
    println!("   Generated ID: {}", deterministic_id);

    // Create trace with deterministic ID
    let response = client
        .trace()
        .id(deterministic_id.clone())
        .name("Deterministic ID Trace")
        .input(json!({
            "seed": seed,
            "message": "This ID is reproducible from the seed"
        }))
        .tags(vec!["deterministic-id".to_string()])
        .call()
        .await?;

    println!("   Trace URL: {}", response.url());

    // Running this again with the same seed would produce the same ID
    let same_id = IdGenerator::from_seed(seed);
    println!(
        "   Verified: Same seed produces same ID: {}",
        deterministic_id == same_id
    );
    println!();

    // Example 4: Hierarchical IDs for related observations
    println!("4️⃣ Creating hierarchical IDs for related observations:");

    let base_seed = "workflow-execution-2024";
    let trace_id = IdGenerator::from_components(&[base_seed, "trace"]);

    let response = client
        .trace()
        .id(trace_id.clone())
        .name("Workflow Trace")
        .input(json!({
            "workflow": "data-processing",
            "run_id": base_seed
        }))
        .call()
        .await?;

    println!("   Trace ID: {}", trace_id);
    println!("   Trace URL: {}", response.url());

    // Create related spans with hierarchical IDs
    let span1_id = IdGenerator::from_components(&[base_seed, "span", "data-fetch"]);
    let span2_id = IdGenerator::from_components(&[base_seed, "span", "data-process"]);

    let span1 = client
        .span()
        .trace_id(trace_id.clone())
        .id(span1_id.clone())
        .name("Fetch Data")
        .input(json!({ "source": "database" }))
        .call()
        .await?;

    println!("   └─ Span 1 ID: {}", span1);

    let span2 = client
        .span()
        .trace_id(trace_id.clone())
        .id(span2_id.clone())
        .parent_observation_id(span1_id.clone())
        .name("Process Data")
        .input(json!({ "operation": "transform" }))
        .call()
        .await?;

    println!("      └─ Span 2 ID: {}", span2);
    println!();

    // Example 5: Hash-based IDs for simple cases
    println!("5️⃣ Using hash-based IDs:");

    let hash_seed = "simple-seed-123";
    let hash_id = IdGenerator::from_hash(hash_seed);

    println!("   Hash seed: {}", hash_seed);
    println!("   Hash ID: {}", hash_id);

    // This is useful when you want shorter, simpler IDs
    let response = client
        .trace()
        .id(hash_id.clone())
        .name("Hash ID Trace")
        .metadata(json!({
            "id_type": "hash",
            "seed": hash_seed
        }))
        .call()
        .await?;

    println!("   Trace URL: {}", response.url());
    println!();

    // Example 6: Idempotent trace creation
    println!("6️⃣ Demonstrating idempotent trace creation:");

    let idempotent_seed = format!("daily-report:{}", chrono::Utc::now().format("%Y-%m-%d"));
    let idempotent_id = IdGenerator::from_seed(&idempotent_seed);

    println!("   Creating trace with ID from seed: {}", idempotent_seed);

    // First creation
    let response1 = client
        .trace()
        .id(idempotent_id.clone())
        .name("Daily Report")
        .metadata(json!({
            "created_at": chrono::Utc::now().to_rfc3339(),
            "attempt": 1
        }))
        .call()
        .await?;

    println!("   First creation - URL: {}", response1.url());

    // Second creation with same ID (will update the existing trace)
    let response2 = client
        .trace()
        .id(idempotent_id.clone())
        .name("Daily Report (Updated)")
        .metadata(json!({
            "updated_at": chrono::Utc::now().to_rfc3339(),
            "attempt": 2
        }))
        .call()
        .await?;

    println!("   Second creation - URL: {}", response2.url());
    println!("   Same URL: {}", response1.url() == response2.url());
    println!();

    println!("✅ All examples completed successfully!");
    println!("\n📝 Summary:");
    println!("   - Traces automatically provide their Langfuse URLs via .url()");
    println!("   - Custom IDs enable deterministic and idempotent trace creation");
    println!("   - Seed-based IDs ensure reproducibility across runs");
    println!("   - Hierarchical IDs help organize related observations");

    Ok(())
}
