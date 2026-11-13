//! Natively, the futures of the 0.15 APIs are `Send`, so they can be spawned on a multi-threaded
//! runtime (on WebAssembly nothing needs to be `Send`).
#![cfg(not(target_family = "wasm"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod recorder;

use std::future::Future;

use langfuse_ergonomic::experiments::{
    evaluator_fn, Evaluation, Evaluator, EvaluatorContext, EvaluatorError,
};
use langfuse_ergonomic::observe::{self, Observation, ObserveFutureExt};
use langfuse_ergonomic::{ApiVersion, ClientBuilder, LangfuseClient, PromptFallback};
use mockito::Matcher;
use recorder::Recorder;
use serde_json::json;
use timvw_rust_runtime::Upload;

fn assert_send<T: Send>(_: &T) {}

struct Length;

impl Evaluator for Length {
    async fn evaluate(&self, ctx: &EvaluatorContext) -> Result<Vec<Evaluation>, EvaluatorError> {
        tokio::task::yield_now().await;
        Ok(vec![Evaluation::new(
            "length",
            f64::from(u32::try_from(ctx.output.to_string().len())?),
        )])
    }
}

async fn task(item: langfuse_ergonomic::DatasetItem) -> Result<String, String> {
    tokio::task::yield_now().await;
    Ok(format!("{:?}", item.input))
}

/// Compile-time checks only: the futures are built, checked and dropped.
#[allow(dead_code)]
fn the_new_futures_are_send(client: &LangfuseClient) {
    let experiment = client
        .run_experiment("dataset", task)
        .evaluator(Length)
        .evaluator(evaluator_fn(|_ctx: EvaluatorContext| async move {
            Ok(vec![Evaluation::new("ok", true)])
        }))
        .concurrency(4)
        .run();
    assert_send(&experiment);

    assert_send(
        &client
            .prompt("p")
            .fallback(PromptFallback::text("x"))
            .call(),
    );
    assert_send(&client.prefetch_prompts([("p", None, Some("production"))]));
    assert_send(
        &client
            .upload_media()
            .upload(Upload::bytes(vec![1u8]))
            .trace_id("t")
            .call(),
    );
    assert_send(&client.get_media("m"));
    assert_send(&client.download_media("m"));
    assert_send(
        &client
            .create_dataset_run_item()
            .run_name("r")
            .dataset_item_id("i")
            .call(),
    );
    assert_send(&client.agent().trace_id("t").call());
    assert_send(&observe::flush());
    let observation = Observation::builder("x").start();
    assert_send(&observation);
    assert_send(&observation.clone().instrument(async {}));
    assert_send(&async {}.in_current_observation());
    let batcher_add = async move {
        let batcher = client.batcher().build().await;
        batcher.flush().await
    };
    assert_send(&batcher_add);
}

fn spawn_send<F: Future + Send + 'static>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F::Output: Send + 'static,
{
    tokio::spawn(future)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_experiment_can_be_spawned() {
    let mut recorder = Recorder::start().await;
    recorder
        .server
        .mock("GET", "/api/public/v2/datasets/ds")
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"ds-1","name":"ds","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"p","createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-01T00:00:00Z"}"#)
        .create_async()
        .await;
    recorder
        .server
        .mock("GET", "/api/public/dataset-items")
        .match_query(Matcher::Any)
        .with_header("content-type", "application/json")
        .with_body(
            json!({"data": [{
                "id": "i1", "status": "ACTIVE", "input": "x", "expectedOutput": null,
                "metadata": null, "sourceTraceId": null, "sourceObservationId": null,
                "datasetId": "ds-1", "datasetName": "ds",
                "createdAt": "2026-11-01T00:00:00Z", "updatedAt": "2026-11-01T00:00:00Z",
                "mediaReferences": []
            }], "meta": {"page": 1, "limit": 100, "totalItems": 1, "totalPages": 1}})
            .to_string(),
        )
        .create_async()
        .await;
    recorder
        .server
        .mock("POST", "/api/public/dataset-run-items")
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"dri","datasetRunId":"run","datasetRunName":"r","datasetItemId":"i1","traceId":"t","observationId":null,"createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-01T00:00:00Z"}"#)
        .create_async()
        .await;

    let client = recorder.client(ApiVersion::V3);
    let result = spawn_send(async move {
        client
            .run_experiment("ds", task)
            .run_name("r")
            .evaluator(Length)
            .concurrency(2)
            .run()
            .await
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].output, Some(json!("Some(String(\"x\"))")));
    assert_eq!(result.aggregates["length"].count, 1);
    assert_eq!(result.dataset_run_id.as_deref(), Some("run"));

    // The other new futures spawn too.
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(recorder.server.url())
        .build()
        .unwrap();
    let prompt = spawn_send(async move {
        client
            .prompt("missing")
            .fallback(PromptFallback::text("hi"))
            .call()
            .await
    });
    assert!(prompt.await.unwrap().unwrap().is_fallback());
    spawn_send(observe::flush()).await.unwrap().unwrap();
}
