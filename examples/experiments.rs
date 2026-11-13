//! An experiment: a task over a dataset, scored by two evaluators.
//!
//! `cargo run --example experiments`
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::experiments::{
    evaluator_fn, Evaluation, Evaluator, EvaluatorContext, EvaluatorError,
};
use langfuse_ergonomic::{observe, ClientBuilder};
use serde_json::json;

/// The system under test.
#[observe(as_type = "generation", model = "rules-v1")]
async fn capital_of(country: String) -> Result<String, String> {
    match country.as_str() {
        "Belgium" => Ok("Brussels".to_owned()),
        "France" => Ok("Paris".to_owned()),
        other => Err(format!("I do not know {other}")),
    }
}

/// An evaluator as a type.
struct ShortAnswer;

impl Evaluator for ShortAnswer {
    async fn evaluate(&self, ctx: &EvaluatorContext) -> Result<Vec<Evaluation>, EvaluatorError> {
        let short = ctx.output.as_str().is_some_and(|answer| answer.len() < 20);
        Ok(vec![Evaluation::new("short", short)])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let client = ClientBuilder::from_env()?.build()?;

    let dataset = format!("capitals-{}", uuid::Uuid::new_v4().simple());
    client.create_dataset().name(&dataset).call().await?;
    for (country, capital) in [("Belgium", "Brussels"), ("France", "Paris"), ("Mars", "-")] {
        client
            .create_dataset_item()
            .dataset_name(&dataset)
            .input(json!(country))
            .expected_output(json!(capital))
            .call()
            .await?;
    }

    let result = client
        .run_experiment(&dataset, |item| async move {
            capital_of(item.input.unwrap().as_str().unwrap().to_owned()).await
        })
        .run_name("rules-v1")
        .evaluator(evaluator_fn(|ctx: EvaluatorContext| async move {
            let exact = Some(&ctx.output) == ctx.expected_output.as_ref();
            Ok(vec![Evaluation::new("exact_match", exact)])
        }))
        .evaluator(ShortAnswer)
        .concurrency(2)
        .run()
        .await?;

    for item in &result.items {
        println!(
            "{:?} -> {:?} {:?}",
            item.item.input, item.output, item.error
        );
    }
    for (name, aggregate) in &result.aggregates {
        println!("{name}: {:.2} (n = {})", aggregate.mean, aggregate.count);
    }
    println!("Run: {}", result.run_url.unwrap_or_default());
    Ok(())
}
