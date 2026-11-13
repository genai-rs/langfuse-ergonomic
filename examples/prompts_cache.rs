//! Cached prompts with a fallback, compiled, and linked to a generation.
//!
//! `cargo run --example prompts_cache`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use langfuse_ergonomic::{ClientBuilder, PromptFallback};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.build()?;

    let name = format!("cache-demo-{}", uuid::Uuid::new_v4().simple());
    client
        .create_prompt()
        .name(&name)
        .prompt("Translate to {{language}}: {{text}}")
        .labels(vec!["production".to_owned()])
        .call()
        .await?;

    for attempt in 1..=2 {
        let prompt = client
            .prompt(&name)
            .cache_ttl(Duration::from_secs(60))
            .fallback(PromptFallback::text("Translate to {{language}}: {{text}}"))
            .call()
            .await?;
        let compiled = prompt.compile([("language", "Dutch"), ("text", "Good morning")]);
        println!(
            "attempt {attempt}: {:?} from {:?} (version {:?})",
            compiled.as_text(),
            prompt.source,
            prompt.version()
        );

        if attempt == 2 {
            let trace = client.trace().name("prompt-linked").call().await?;
            client
                .generation()
                .trace_id(&trace.id)
                .name("translate")
                .prompt_name(prompt.name())
                .maybe_prompt_version(prompt.version())
                .call()
                .await?;
        }
    }

    // A prompt Langfuse does not have: the fallback.
    let missing = client
        .prompt("does-not-exist")
        .fallback(PromptFallback::chat([("system", "You are {{persona}}.")]))
        .call()
        .await?;
    println!(
        "fallback: {:?}",
        missing.compile([("persona", "helpful")]).as_messages()
    );
    Ok(())
}
