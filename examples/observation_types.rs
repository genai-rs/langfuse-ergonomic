//! Every observation type with the builders: agent, tool, chain, retriever, evaluator,
//! embedding, guardrail, and a generation with usage, cost and a prompt link.
//!
//! `cargo run --example observation_types`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;

use chrono::{Duration, Utc};
use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.build()?;

    let trace = client
        .trace()
        .name("observation-types")
        .user_id("user-1")
        .call()
        .await?;

    let agent = client
        .agent()
        .trace_id(&trace.id)
        .name("planner")
        .input(json!({"goal": "answer a question"}))
        .output(json!({"plan": ["retrieve", "answer", "check"]}))
        .call()
        .await?;

    let chain = client
        .chain()
        .trace_id(&trace.id)
        .parent_observation_id(&agent)
        .name("rag-chain")
        .call()
        .await?;

    client
        .embedding()
        .trace_id(&trace.id)
        .parent_observation_id(&chain)
        .name("embed-query")
        .model("text-embedding-3-small")
        .usage_details(HashMap::from([("input".to_owned(), 9)]))
        .call()
        .await?;

    client
        .retriever()
        .trace_id(&trace.id)
        .parent_observation_id(&chain)
        .name("vector-search")
        .input(json!({"top_k": 3}))
        .output(json!({"documents": 3}))
        .call()
        .await?;

    let start = Utc::now();
    client
        .generation()
        .trace_id(&trace.id)
        .parent_observation_id(&chain)
        .name("answer")
        .model("gpt-4o")
        .start_time(start)
        .completion_start_time(start + Duration::milliseconds(200))
        .end_time(start + Duration::milliseconds(800))
        .usage_details(HashMap::from([
            ("input".to_owned(), 320),
            ("output".to_owned(), 48),
            ("cache_read_input_tokens".to_owned(), 256),
        ]))
        .cost_details(HashMap::from([("total".to_owned(), 0.0011)]))
        .prompt_name("rag-answer")
        .prompt_version(1)
        .call()
        .await?;

    client
        .tool()
        .trace_id(&trace.id)
        .parent_observation_id(&agent)
        .name("calculator")
        .input(json!({"expression": "6 * 7"}))
        .output(json!(42))
        .call()
        .await?;

    client
        .evaluator()
        .trace_id(&trace.id)
        .parent_observation_id(&agent)
        .name("faithfulness-judge")
        .output(json!({"score": 0.9}))
        .call()
        .await?;

    client
        .guardrail()
        .trace_id(&trace.id)
        .parent_observation_id(&agent)
        .name("pii-filter")
        .output(json!({"passed": true}))
        .call()
        .await?;

    println!("Trace: {}", trace.url());
    Ok(())
}
