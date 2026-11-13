//! Experiments: run a task over a dataset, trace each run, link it to a dataset run and score it.
//!
//! [`LangfuseClient::run_experiment`] fetches the dataset's active items and, for each item (up
//! to `concurrency` at a time):
//!
//! 1. runs the task inside a new trace (a root observation named after the run, with the item's
//!    input; `#[observe]` functions the task calls nest under it);
//! 2. records the output, or the error (level `ERROR`);
//! 3. runs the evaluators on the output and sends their [`Evaluation`]s as scores of the trace.
//!
//! Then it sends everything, links each trace to the dataset run with
//! `POST /api/public/dataset-run-items`, and returns an [`ExperimentResult`]: per-item results,
//! aggregate scores and the run's URL.
//!
//! Concurrency comes from polling the item futures together (`buffer_unordered`) in the calling
//! task: no runtime is needed beyond the one running `run()`, which also works on WebAssembly.
//! Natively the task's and the evaluators' futures must be `Send` (and the task and evaluators
//! `Sync`), so that the `run()` future is `Send` and can be spawned; on WebAssembly nothing needs
//! to be `Send`.
//!
//! ```no_run
//! use langfuse_ergonomic::experiments::{evaluator_fn, Evaluation, EvaluatorContext};
//! use langfuse_ergonomic::ClientBuilder;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ClientBuilder::from_env()?.build()?;
//! let result = client
//!     .run_experiment("capital-cities", |item| async move {
//!         let country = item.input.as_ref().and_then(|i| i.as_str()).unwrap_or_default().to_owned();
//!         Ok::<_, std::io::Error>(format!("The capital of {country}"))
//!     })
//!     .run_name("baseline")
//!     .evaluator(evaluator_fn(|ctx: EvaluatorContext| async move {
//!         let exact = Some(&ctx.output) == ctx.expected_output.as_ref();
//!         Ok(vec![Evaluation::new("exact_match", exact)])
//!     }))
//!     .concurrency(4)
//!     .run()
//!     .await?;
//! println!("{:?} {:?}", result.aggregates.get("exact_match"), result.run_url);
//! # Ok(())
//! # }
//! ```

use std::collections::BTreeMap;
use std::fmt::Display;
use std::future::Future;

use std::pin::Pin;
use std::time::Duration;
use timvw_rust_runtime::{MaybeSend, MaybeSync};

use futures_util::StreamExt;
use langfuse_client_base::models::{DatasetItem, DatasetStatus};
use serde::Serialize;
use serde_json::Value;

use crate::client::LangfuseClient;
use crate::error::Result;
use crate::observe::{Observation, ObservationType, ObserveFutureExt};
use crate::scores::ScoreValue;
use crate::Batcher;

/// The error an evaluator returns.
pub type EvaluatorError = Box<dyn std::error::Error + Send + Sync>;

/// What an evaluator sees of one item's run.
#[derive(Debug, Clone)]
pub struct EvaluatorContext {
    /// The dataset item.
    pub item: DatasetItem,
    /// The item's input.
    pub input: Option<Value>,
    /// The task's output (serialised).
    pub output: Value,
    /// The item's expected output.
    pub expected_output: Option<Value>,
    /// The item's metadata.
    pub metadata: Option<Value>,
    /// The trace of the run.
    pub trace_id: String,
}

/// One evaluation, sent as a score of the item's trace.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// The score name.
    pub name: String,
    /// The value.
    pub value: ScoreValue,
    /// An optional comment (for example the judge's reasoning).
    pub comment: Option<String>,
}

impl Evaluation {
    /// An evaluation without a comment.
    pub fn new(name: impl Into<String>, value: impl Into<ScoreValue>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            comment: None,
        }
    }

    /// Add a comment.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }
}

/// Scores the output of one item. Implement it, or wrap a closure with [`evaluator_fn`].
///
/// The future must be `Send` natively (so that an experiment can be spawned), and need not be on
/// WebAssembly.
pub trait Evaluator {
    /// Evaluate one item's run.
    fn evaluate<'a>(
        &'a self,
        context: &'a EvaluatorContext,
    ) -> impl Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + MaybeSend;
}

/// An [`Evaluator`] made from a closure ([`evaluator_fn`]).
#[derive(Debug, Clone, Copy)]
pub struct FnEvaluator<F>(F);

/// Make an [`Evaluator`] from an async closure that takes the [`EvaluatorContext`] by value.
pub fn evaluator_fn<F, Fut>(f: F) -> FnEvaluator<F>
where
    F: Fn(EvaluatorContext) -> Fut,
    Fut: Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + MaybeSend,
{
    FnEvaluator(f)
}

impl<F, Fut> Evaluator for FnEvaluator<F>
where
    F: Fn(EvaluatorContext) -> Fut,
    Fut: Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + MaybeSend,
{
    fn evaluate<'a>(
        &'a self,
        context: &'a EvaluatorContext,
    ) -> impl Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + MaybeSend
    {
        (self.0)(context.clone())
    }
}

/// A boxed evaluation: `Send` natively, any future on WebAssembly.
#[cfg(not(target_family = "wasm"))]
type EvaluationFuture<'a> =
    Pin<Box<dyn Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + Send + 'a>>;
/// A boxed evaluation: `Send` natively, any future on WebAssembly.
#[cfg(target_family = "wasm")]
type EvaluationFuture<'a> =
    Pin<Box<dyn Future<Output = std::result::Result<Vec<Evaluation>, EvaluatorError>> + 'a>>;

/// A boxed evaluator: `Send + Sync` natively, any evaluator on WebAssembly.
#[cfg(not(target_family = "wasm"))]
type BoxedEvaluator<'c> = Box<dyn DynEvaluator + Send + Sync + 'c>;
/// A boxed evaluator: `Send + Sync` natively, any evaluator on WebAssembly.
#[cfg(target_family = "wasm")]
type BoxedEvaluator<'c> = Box<dyn DynEvaluator + 'c>;

/// An object-safe [`Evaluator`], so that evaluators of different types can be combined.
trait DynEvaluator {
    fn evaluate_dyn<'a>(&'a self, context: &'a EvaluatorContext) -> EvaluationFuture<'a>;
}

impl<E: Evaluator> DynEvaluator for E {
    fn evaluate_dyn<'a>(&'a self, context: &'a EvaluatorContext) -> EvaluationFuture<'a> {
        Box::pin(self.evaluate(context))
    }
}

/// The outcome of one dataset item.
#[derive(Debug, Clone)]
pub struct ExperimentItemResult {
    /// The dataset item.
    pub item: DatasetItem,
    /// The trace of its run.
    pub trace_id: Option<String>,
    /// The root observation of its run.
    pub observation_id: Option<String>,
    /// The task's output (serialised), when it succeeded.
    pub output: Option<Value>,
    /// The task's error, when it failed (evaluators are then skipped).
    pub error: Option<String>,
    /// The evaluations (sent as scores of the trace).
    pub evaluations: Vec<Evaluation>,
    /// Errors of evaluators that failed.
    pub evaluator_errors: Vec<String>,
    /// The dataset run item that links the trace to the run.
    pub dataset_run_item_id: Option<String>,
    /// Why the trace could not be linked to the run, if it could not.
    pub link_error: Option<String>,
}

/// Aggregate of one score over the items (numeric and boolean scores; booleans count as 1/0).
#[derive(Debug, Clone, PartialEq)]
pub struct ScoreAggregate {
    /// How many items have this score.
    pub count: usize,
    /// The mean.
    pub mean: f64,
    /// The smallest value.
    pub min: f64,
    /// The largest value.
    pub max: f64,
}

/// The outcome of an experiment.
#[derive(Debug, Clone)]
pub struct ExperimentResult {
    /// The run name.
    pub run_name: String,
    /// The dataset's id.
    pub dataset_id: String,
    /// The dataset run's id (once a run item was created).
    pub dataset_run_id: Option<String>,
    /// The run in the Langfuse UI.
    pub run_url: Option<String>,
    /// One result per item, in dataset order.
    pub items: Vec<ExperimentItemResult>,
    /// Aggregates per numeric or boolean score name.
    pub aggregates: BTreeMap<String, ScoreAggregate>,
}

impl ExperimentResult {
    /// The items whose task failed.
    pub fn failed_items(&self) -> impl Iterator<Item = &ExperimentItemResult> {
        self.items.iter().filter(|item| item.error.is_some())
    }
}

/// Configures and runs an experiment ([`LangfuseClient::run_experiment`]).
pub struct ExperimentBuilder<'c, F> {
    client: &'c LangfuseClient,
    dataset_name: String,
    task: F,
    run_name: Option<String>,
    description: Option<String>,
    metadata: Option<Value>,
    evaluators: Vec<BoxedEvaluator<'c>>,
    concurrency: usize,
}

impl<F> std::fmt::Debug for ExperimentBuilder<'_, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExperimentBuilder")
            .field("dataset_name", &self.dataset_name)
            .field("run_name", &self.run_name)
            .field("evaluators", &self.evaluators.len())
            .field("concurrency", &self.concurrency)
            .finish_non_exhaustive()
    }
}

impl LangfuseClient {
    /// Start an experiment: run `task` on every active item of the dataset `dataset_name`.
    ///
    /// `task` gets each [`DatasetItem`] and returns `Result<T, E>` (`T: Serialize`, `E:
    /// Display`). See [`crate::experiments`].
    pub fn run_experiment<F, Fut>(
        &self,
        dataset_name: impl Into<String>,
        task: F,
    ) -> ExperimentBuilder<'_, F>
    where
        F: Fn(DatasetItem) -> Fut,
    {
        ExperimentBuilder {
            client: self,
            dataset_name: dataset_name.into(),
            task,
            run_name: None,
            description: None,
            metadata: None,
            evaluators: Vec::new(),
            concurrency: 1,
        }
    }
}

impl<'c, F> ExperimentBuilder<'c, F> {
    /// The run name (default `<dataset>-<UTC timestamp>`). Runs with the same name share a
    /// dataset run.
    #[must_use]
    pub fn run_name(mut self, run_name: impl Into<String>) -> Self {
        self.run_name = Some(run_name.into());
        self
    }

    /// The run description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Metadata of the run (and of each item's trace).
    #[must_use]
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add an evaluator.
    #[must_use]
    pub fn evaluator(mut self, evaluator: impl Evaluator + MaybeSend + MaybeSync + 'c) -> Self {
        self.evaluators.push(Box::new(evaluator));
        self
    }

    /// How many items run at once (default 1).
    #[must_use]
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Run the experiment.
    pub async fn run<Fut, T, E>(self) -> Result<ExperimentResult>
    where
        F: Fn(DatasetItem) -> Fut + MaybeSync,
        Fut: Future<Output = std::result::Result<T, E>> + MaybeSend,
        T: Serialize + MaybeSend,
        E: Display + MaybeSend,
    {
        let client = self.client;
        let dataset = client.get_dataset(self.dataset_name.as_str()).await?;
        let items = self.items().await?;
        let run_name = self.run_name.clone().unwrap_or_else(|| {
            format!(
                "{}-{}",
                self.dataset_name,
                crate::time::now_utc().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            )
        });

        let batcher = Batcher::builder()
            .client(client.clone())
            .flush_interval(Duration::from_secs(1))
            .build()
            .await;

        // The futures are collected first (rather than mapped lazily with a closure over
        // borrows), which keeps the `run()` future `Send` natively.
        let item_runs: Vec<_> = items
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let batcher = batcher.clone();
                let run_name = run_name.as_str();
                let this = &self;
                async move { (index, this.run_item(item, batcher, run_name).await) }
            })
            .collect();
        let runs = futures_util::stream::iter(item_runs)
            .buffer_unordered(self.concurrency)
            .collect::<Vec<_>>()
            .await;
        let mut runs = runs;
        runs.sort_by_key(|(index, _)| *index);
        let mut results: Vec<ExperimentItemResult> =
            runs.into_iter().map(|(_, result)| result).collect();

        if let Err(error) = batcher.shutdown().await {
            tracing::warn!(%error, "sending the experiment's traces and scores failed");
        }

        // Link each trace to the dataset run.
        let mut dataset_run_id = None;
        let link_runs: Vec<_> = results
            .iter()
            .enumerate()
            .map(|(index, result)| {
                let run_name = run_name.as_str();
                let this = &self;
                async move { (index, this.link(result, run_name).await) }
            })
            .collect();
        let links = futures_util::stream::iter(link_runs)
            .buffer_unordered(self.concurrency)
            .collect::<Vec<_>>()
            .await;
        for (index, link) in links {
            match link {
                Ok(Some(run_item)) => {
                    dataset_run_id.get_or_insert(run_item.dataset_run_id.clone());
                    results[index].dataset_run_item_id = Some(run_item.id);
                }
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(%error, "linking an experiment trace to the dataset run failed");
                    results[index].link_error = Some(error.to_string());
                }
            }
        }

        let run_url = dataset_run_id.as_ref().map(|run_id| {
            format!(
                "{}/project/{}/datasets/{}/runs/{run_id}",
                client.web_base_url(),
                dataset.project_id,
                dataset.id
            )
        });

        Ok(ExperimentResult {
            run_name,
            dataset_id: dataset.id,
            dataset_run_id,
            run_url,
            aggregates: aggregate(&results),
            items: results,
        })
    }

    /// Every active item of the dataset, page by page.
    async fn items(&self) -> Result<Vec<DatasetItem>> {
        let mut items = Vec::new();
        let mut page = 1;
        loop {
            let response = self
                .client
                .list_dataset_items()
                .dataset_name(self.dataset_name.as_str())
                .page(page)
                .limit(100)
                .call()
                .await?;
            let count = response.data.len();
            items.extend(
                response
                    .data
                    .into_iter()
                    .filter(|item| item.status != DatasetStatus::Archived),
            );
            if count == 0 || i64::from(page) >= response.meta.total_pages {
                return Ok(items);
            }
            page += 1;
        }
    }

    async fn run_item<Fut, T, E>(
        &self,
        item: DatasetItem,
        batcher: Batcher,
        run_name: &str,
    ) -> ExperimentItemResult
    where
        F: Fn(DatasetItem) -> Fut + MaybeSync,
        Fut: Future<Output = std::result::Result<T, E>> + MaybeSend,
        T: Serialize + MaybeSend,
        E: Display + MaybeSend,
    {
        let mut metadata = serde_json::json!({
            "dataset_id": item.dataset_id,
            "dataset_item_id": item.id,
            "experiment_run_name": run_name,
        });
        if let (Some(Value::Object(extra)), Value::Object(map)) = (&self.metadata, &mut metadata) {
            map.extend(extra.clone());
        }
        let observation = Observation::builder("experiment-item-run")
            .as_type(ObservationType::Chain)
            .batcher(batcher)
            .metadata(&metadata)
            .start();
        if let Some(input) = &item.input {
            observation.set_input(input);
        }
        observation.set_trace_name(run_name.to_owned());

        let outcome = (self.task)(item.clone())
            .with_observation(observation.clone())
            .await;
        let mut result = ExperimentItemResult {
            trace_id: observation.trace_id().map(str::to_owned),
            observation_id: observation.id().map(str::to_owned),
            item,
            output: None,
            error: None,
            evaluations: Vec::new(),
            evaluator_errors: Vec::new(),
            dataset_run_item_id: None,
            link_error: None,
        };
        let output = match outcome {
            Ok(output) => serde_json::to_value(&output).unwrap_or(Value::Null),
            Err(error) => {
                let message = error.to_string();
                observation.set_error(message.clone());
                observation.end();
                result.error = Some(message);
                return result;
            }
        };
        observation.set_output(&output);
        observation.end();
        result.output = Some(output.clone());

        let context = EvaluatorContext {
            input: result.item.input.clone(),
            expected_output: result.item.expected_output.clone(),
            metadata: result.item.metadata.clone(),
            item: result.item.clone(),
            output,
            trace_id: result.trace_id.clone().unwrap_or_default(),
        };
        for evaluator in &self.evaluators {
            match evaluator.evaluate_dyn(&context).await {
                Ok(evaluations) => {
                    for evaluation in &evaluations {
                        observation.score_trace_with(
                            evaluation.name.clone(),
                            evaluation.value.clone(),
                            evaluation.comment.clone(),
                        );
                    }
                    result.evaluations.extend(evaluations);
                }
                Err(error) => result.evaluator_errors.push(error.to_string()),
            }
        }
        result
    }

    async fn link(
        &self,
        result: &ExperimentItemResult,
        run_name: &str,
    ) -> Result<Option<langfuse_client_base::models::DatasetRunItem>> {
        let Some(trace_id) = &result.trace_id else {
            return Ok(None);
        };
        self.client
            .create_dataset_run_item()
            .run_name(run_name)
            .dataset_item_id(result.item.id.as_str())
            .trace_id(trace_id.as_str())
            .maybe_run_description(self.description.clone())
            .maybe_metadata(self.metadata.clone())
            .call()
            .await
            .map(Some)
    }
}

fn aggregate(results: &[ExperimentItemResult]) -> BTreeMap<String, ScoreAggregate> {
    let mut values: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for evaluation in results.iter().flat_map(|r| &r.evaluations) {
        if let Some(value) = evaluation.value.as_f64() {
            values
                .entry(evaluation.name.clone())
                .or_default()
                .push(value);
        }
    }
    values
        .into_iter()
        .map(|(name, values)| {
            #[allow(clippy::cast_precision_loss)]
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (
                name,
                ScoreAggregate {
                    count: values.len(),
                    mean,
                    min,
                    max,
                },
            )
        })
        .collect()
}
