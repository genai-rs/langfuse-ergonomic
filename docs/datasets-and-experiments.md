# Datasets and experiments

## Datasets

```rust,no_run
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient) -> Result<(), Box<dyn std::error::Error>> {
    client
        .create_dataset()
        .name("capitals")
        .description("Countries and their capitals")
        .call()
        .await?;
    client
        .create_dataset_item()
        .dataset_name("capitals")
        .input(json!("Belgium"))
        .expected_output(json!("Brussels"))
        .call()
        .await?;

    let items = client.list_dataset_items().dataset_name("capitals").call().await?;
    println!("{} items", items.data.len());
    Ok(())
}
```

`get_dataset`, `list_datasets`, `get_dataset_item` and `delete_dataset_item` complete the
set. `create_dataset_run_item()` links a trace to a dataset run and `list_dataset_run_items()`
lists a run's items; both work with Langfuse v3 and v4.

## Experiments

`client.run_experiment(dataset, task)` runs a task on every active item of a dataset and
records the run in Langfuse:

1. each item runs inside its own trace (a root observation named `experiment-item-run`, the
   trace named after the run, with the item's input), so [`#[observe]`](observe.md) functions
   the task calls nest under it;
2. the output is recorded, or the task's error (level `ERROR`; the item's evaluators are
   skipped);
3. the evaluators score the output; their evaluations become scores of the trace;
4. every trace is linked to the dataset run (`POST /api/public/dataset-run-items`).

It returns an `ExperimentResult` with per-item results (output or error, evaluations, trace
id), aggregate scores (count, mean, min, max per numeric or boolean score) and the run's URL.

```rust,no_run
use langfuse_ergonomic::experiments::{evaluator_fn, Evaluation, EvaluatorContext};
use langfuse_ergonomic::{observe, ClientBuilder};

#[observe(as_type = "generation", model = "my-model")]
async fn capital_of(country: String) -> Result<String, std::io::Error> {
    Ok(match country.as_str() {
        "Belgium" => "Brussels".to_owned(),
        other => format!("the capital of {other}"),
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.build()?;
    let result = client
        .run_experiment("capitals", |item| async move {
            let country = item.input.and_then(|input| input.as_str().map(str::to_owned));
            capital_of(country.unwrap_or_default()).await
        })
        .run_name("baseline")
        .description("No retrieval")
        .evaluator(evaluator_fn(|ctx: EvaluatorContext| async move {
            let exact = Some(&ctx.output) == ctx.expected_output.as_ref();
            Ok(vec![Evaluation::new("exact_match", exact)])
        }))
        .concurrency(4)
        .run()
        .await?;

    for (name, aggregate) in &result.aggregates {
        println!("{name}: mean {:.2} over {}", aggregate.mean, aggregate.count);
    }
    println!("{}", result.run_url.unwrap_or_default());
    Ok(())
}
```

### Evaluators

An evaluator implements `Evaluator` (a native `async fn evaluate`), or wraps a closure with
`evaluator_fn`. It gets an `EvaluatorContext` (item, input, output, expected output, metadata,
trace id) and returns `Evaluation`s: a name, a `ScoreValue` (number, boolean or category) and
an optional comment.

```rust
use langfuse_ergonomic::experiments::{Evaluation, Evaluator, EvaluatorContext, EvaluatorError};

struct AnswerLength;

impl Evaluator for AnswerLength {
    async fn evaluate(&self, ctx: &EvaluatorContext) -> Result<Vec<Evaluation>, EvaluatorError> {
        let length = ctx.output.as_str().map_or(0, str::len);
        Ok(vec![Evaluation::new("length", length as f64).with_comment("characters")])
    }
}
```

An LLM-as-a-judge evaluator calls a model inside `evaluate`; observe it with `#[observe(as_type
= "evaluator")]` to see the judge's calls in their own traces.

### Concurrency

`concurrency(n)` runs up to `n` items at once by polling their futures together
(`buffer_unordered`) in the calling task: there is no spawning, and it works on WebAssembly.
Natively the task and the evaluators must be `Sync` and their futures `Send`, and in turn the
`run()` future is `Send`: it can be `tokio::spawn`ed. On WebAssembly nothing needs to be
`Send`. Traces and scores go through a batcher of the experiment's own, which is
flushed before the traces are linked to the run.

See `examples/experiments.rs`.
