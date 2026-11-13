//! Experiments against a mock Langfuse: tasks traced, run items linked, evaluators scored.
#![cfg(all(feature = "macros", not(target_family = "wasm")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod recorder;

use langfuse_ergonomic::experiments::{
    evaluator_fn, Evaluation, Evaluator, EvaluatorContext, EvaluatorError,
};
use langfuse_ergonomic::{observe, ApiVersion};
use mockito::Matcher;
use recorder::Recorder;
use serde_json::{json, Value};

const DATASET: &str = r#"{"id":"ds-1","name":"capitals","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"proj-1","createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-01T00:00:00Z"}"#;

fn item(id: &str, input: &str, expected: &str, status: &str) -> Value {
    json!({
        "id": id, "status": status, "input": input, "expectedOutput": expected,
        "datasetId": "ds-1", "datasetName": "capitals", "metadata": null, "sourceTraceId": null, "sourceObservationId": null,
        "createdAt": "2026-11-01T00:00:00Z", "updatedAt": "2026-11-01T00:00:00Z",
        "mediaReferences": []
    })
}

/// An evaluator implemented as a type.
struct Length;

impl Evaluator for Length {
    async fn evaluate(
        &self,
        context: &EvaluatorContext,
    ) -> Result<Vec<Evaluation>, EvaluatorError> {
        let length = context.output.as_str().map_or(0, str::len);
        Ok(vec![Evaluation::new(
            "length",
            f64::from(u32::try_from(length)?),
        )])
    }
}

#[observe(as_type = "generation")]
async fn lookup(country: String) -> Result<String, String> {
    match country.as_str() {
        "France" => Ok("Paris".into()),
        "Belgium" => Ok("Brussels".into()),
        other => Err(format!("unknown country {other}")),
    }
}

#[tokio::test]
async fn experiments_trace_link_and_score_every_item() {
    let mut recorder = Recorder::start().await;
    let json_header = ("content-type", "application/json");
    recorder
        .server
        .mock("GET", "/api/public/v2/datasets/capitals")
        .with_header(json_header.0, json_header.1)
        .with_body(DATASET)
        .create_async()
        .await;
    let page = |page: &str, items: Vec<Value>| {
        json!({"data": items, "meta": {"page": page.parse::<i64>().unwrap(), "limit": 100, "totalItems": 4, "totalPages": 2}})
            .to_string()
    };
    recorder
        .server
        .mock("GET", "/api/public/dataset-items")
        .match_query(Matcher::UrlEncoded("page".into(), "1".into()))
        .with_header(json_header.0, json_header.1)
        .with_body(page(
            "1",
            vec![
                item("i1", "France", "Paris", "ACTIVE"),
                item("i2", "Belgium", "Ghent", "ACTIVE"),
            ],
        ))
        .create_async()
        .await;
    recorder
        .server
        .mock("GET", "/api/public/dataset-items")
        .match_query(Matcher::UrlEncoded("page".into(), "2".into()))
        .with_header(json_header.0, json_header.1)
        .with_body(page(
            "2",
            vec![
                item("i3", "Atlantis", "?", "ACTIVE"),
                item("i4", "Old", "?", "ARCHIVED"),
            ],
        ))
        .create_async()
        .await;
    let run_items = recorder
        .server
        .mock("POST", "/api/public/dataset-run-items")
        .match_body(Matcher::PartialJson(json!({"runName": "run-1"})))
        .with_header(json_header.0, json_header.1)
        .with_body(r#"{"id":"dri","datasetRunId":"run-id-1","datasetRunName":"run-1","datasetItemId":"x","traceId":"t","observationId":null,"createdAt":"2026-11-01T00:00:00Z","updatedAt":"2026-11-01T00:00:00Z"}"#)
        .expect(3)
        .create_async()
        .await;

    let client = recorder.client(ApiVersion::V3);
    let result = client
        .run_experiment("capitals", |item| async move {
            let country = item.input.unwrap().as_str().unwrap().to_owned();
            lookup(country).await
        })
        .run_name("run-1")
        .evaluator(evaluator_fn(|ctx: EvaluatorContext| async move {
            Ok(vec![Evaluation::new(
                "exact_match",
                Some(&ctx.output) == ctx.expected_output.as_ref(),
            )
            .with_comment("compared with the expected output")])
        }))
        .evaluator(Length)
        .concurrency(2)
        .run()
        .await
        .unwrap();

    run_items.assert_async().await;
    assert_eq!(result.items.len(), 3);
    assert_eq!(result.dataset_run_id.as_deref(), Some("run-id-1"));
    assert_eq!(
        result.run_url.as_deref(),
        Some(
            format!(
                "{}/project/proj-1/datasets/ds-1/runs/run-id-1",
                recorder.server.url()
            )
            .as_str()
        )
    );
    assert_eq!(result.items[0].output, Some(json!("Paris")));
    assert_eq!(
        result.items[2].error.as_deref(),
        Some("unknown country Atlantis")
    );
    assert_eq!(result.failed_items().count(), 1);
    let exact = &result.aggregates["exact_match"];
    assert_eq!((exact.count, exact.mean), (2, 0.5));
    assert_eq!(result.aggregates["length"].max, 8.0);

    // Each item is a trace: the run's root and the task's `#[observe]` generation under it.
    let roots: Vec<_> = recorder
        .v3_observations()
        .into_iter()
        .filter(|o| o["name"] == "experiment-item-run")
        .collect();
    assert_eq!(roots.len(), 3);
    for (root, item) in roots.iter().map(|r| {
        let id = r["metadata"]["dataset_item_id"]
            .as_str()
            .unwrap()
            .to_owned();
        (r, id)
    }) {
        let result = result.items.iter().find(|r| r.item.id == item).unwrap();
        assert_eq!(root["traceId"].as_str(), result.trace_id.as_deref());
        let child = recorder
            .v3_observations()
            .into_iter()
            .find(|o| o["name"] == "lookup" && o["traceId"] == root["traceId"])
            .unwrap();
        assert_eq!(child["parentObservationId"], root["id"]);
    }
    let failed = roots.iter().find(|r| r["level"] == "ERROR").unwrap();
    assert_eq!(failed["statusMessage"], "unknown country Atlantis");
    let traces = recorder.v3_traces();
    assert!(traces.iter().all(|t| t["name"] == "run-1"));

    // Two evaluators on two successful items: four trace scores.
    let scores = recorder.v3_scores();
    assert_eq!(scores.len(), 4, "{scores:#?}");
    assert!(scores.iter().any(|s| s["name"] == "exact_match"
        && s["dataType"] == "BOOLEAN"
        && s["comment"].is_string()));
}
