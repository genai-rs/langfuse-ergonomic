//! `#[observe]`: an agent with a retriever, a tool and an LLM call, traced automatically.
//!
//! `cargo run --example observe` (needs `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::Duration;

use langfuse_ergonomic::observe::{ObservationType, ObserveFutureExt};
use langfuse_ergonomic::{observe, ClientBuilder};

#[derive(Debug, serde::Serialize)]
struct Document {
    title: String,
    score: f32,
}

#[observe(as_type = "retriever")]
async fn search(query: &str) -> Vec<Document> {
    tokio::time::sleep(Duration::from_millis(20)).await;
    vec![Document {
        title: format!("All about {query}"),
        score: 0.92,
    }]
}

#[observe(as_type = "tool")]
async fn weather(city: String) -> Result<String, String> {
    match city.as_str() {
        "Ghent" => Ok("12°C, cloudy".to_owned()),
        other => Err(format!("no weather for {other}")),
    }
}

#[observe(as_type = "generation", model = "gpt-4o-mini", skip(api_key))]
async fn llm(api_key: &str, prompt: String) -> Result<String, std::io::Error> {
    let _ = api_key; // never recorded
    tokio::time::sleep(Duration::from_millis(50)).await;
    observe::update_current(|o| {
        o.set_model_parameters(serde_json::json!({"temperature": 0.2}));
        o.set_usage_details([("input", 42), ("output", 17)]);
    });
    Ok(format!("Answer based on: {prompt}"))
}

#[observe(
    as_type = "agent",
    name = "travel-agent",
    user_id = "user-42",
    session_id = "demo"
)]
async fn agent(question: String) -> Result<String, std::io::Error> {
    let documents = search(&question).await;
    // A spawned task stays in the trace with in_current_observation().
    let forecast = tokio::spawn(weather("Ghent".to_owned()).in_current_observation())
        .await
        .expect("the task panicked");
    // An error in a tool is recorded (level ERROR) without failing the agent.
    let _ = weather("Atlantis".to_owned()).await;

    let prompt = format!("{question} | {:?} | {forecast:?}", documents);
    let answer = llm("sk-secret", prompt).await?;
    observe::score_current("helpfulness", 0.9);
    Ok(answer)
}

#[observe]
fn summarize(text: &str) -> usize {
    // Synchronous functions work too.
    text.split_whitespace().count()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.build()?;
    let batcher = client.batcher().build().await;
    observe::install_batcher(batcher.clone());

    let answer = agent("Should I bring an umbrella to Ghent?".to_owned()).await?;
    println!("{answer} ({} words)", summarize(&answer));

    // A manual observation, outside any function.
    let check = observe::Observation::builder("post-check")
        .as_type(ObservationType::Guardrail)
        .input(&answer)
        .start();
    check.end_with_output(serde_json::json!({"passed": true}));
    if let Some(url) = check.trace_url() {
        println!("Trace: {url}");
    }

    batcher.shutdown().await?;
    Ok(())
}
