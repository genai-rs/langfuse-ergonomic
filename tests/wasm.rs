//! WebAssembly: the batcher's background task, its timers and retries in a JavaScript host, against a scripted
//! HTTP service (a tower service that is not `Send`, in place of a server).
//!
//! `cargo test --target wasm32-unknown-unknown --no-default-features --test wasm`, with `wasm-bindgen-test-runner`
//! as the target's runner (it runs the tests in Node.js); see docs/webassembly.md.
#![cfg(target_family = "wasm")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use langfuse_client_base::models::{
    ingestion_event_trace_event::Type, IngestionEvent, IngestionEventTraceEvent, TraceBody,
};
use langfuse_ergonomic::{ApiVersion, Batcher, ClientBuilder, LangfuseClient};
use timvw_rust_runtime::{Body, BoxError, Client};
use wasm_bindgen_test::wasm_bindgen_test;
use web_time::Instant;

/// A request the scripted service received: its path and body.
#[derive(Debug, Clone)]
struct Seen {
    path: String,
    body: serde_json::Value,
}

/// A client whose HTTP service answers with `script`, one status per call (then `200`), recording each request.
fn scripted(api_version: ApiVersion, script: Vec<u16>) -> (LangfuseClient, Rc<RefCell<Vec<Seen>>>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let log = seen.clone();
    let service = tower::service_fn(move |request: http::Request<Body>| {
        let log = log.clone();
        let status = script.get(log.borrow().len()).copied().unwrap_or(200);
        async move {
            let path = request.uri().path().to_owned();
            let bytes = request.into_body().collect_bytes().await?;
            let body = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
            log.borrow_mut().push(Seen { path, body });
            Ok::<_, BoxError>(
                http::Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))?,
            )
        }
    });
    let client = ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url("https://langfuse.invalid")
        .api_version(api_version)
        .http_client(Client::from_service(service))
        .build()
        .unwrap();
    (client, seen)
}

fn trace_event(id: &str) -> IngestionEvent {
    IngestionEvent::TraceEvent(IngestionEventTraceEvent {
        body: TraceBody {
            id: Some(Some(format!("trace-{id}"))),
            name: Some(Some("wasm".to_owned())),
            ..TraceBody::default()
        },
        id: id.to_owned(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: Type::TraceCreate,
    })
}

async fn sleep(duration: Duration) {
    gloo_timers::future::sleep(duration).await;
}

#[wasm_bindgen_test]
async fn flush_sends_one_v3_batch() {
    let (client, seen) = scripted(ApiVersion::V3, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    batcher.add(trace_event("a")).await.unwrap();
    batcher.add(trace_event("b")).await.unwrap();

    let response = batcher.flush().await.unwrap();
    assert_eq!(response.success_count, 2);
    let seen = seen.borrow().clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/api/public/ingestion");
    assert_eq!(seen[0].body["batch"].as_array().unwrap().len(), 2);
    assert_eq!(batcher.metrics().flushed, 2);
}

#[wasm_bindgen_test]
async fn the_background_task_flushes_on_its_interval() {
    let (client, seen) = scripted(ApiVersion::V3, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_millis(100))
        .build()
        .await;
    batcher.add(trace_event("a")).await.unwrap();

    let started = Instant::now();
    while seen.borrow().is_empty() {
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "the interval did not flush"
        );
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(batcher.metrics().flushed, 1);
}

#[wasm_bindgen_test]
async fn the_background_task_flushes_a_full_batch() {
    let (client, seen) = scripted(ApiVersion::V3, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .max_events(2)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    batcher.add(trace_event("a")).await.unwrap();
    batcher.add(trace_event("b")).await.unwrap();

    let started = Instant::now();
    while seen.borrow().is_empty() {
        assert!(started.elapsed() < Duration::from_secs(2), "no flush");
        sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(seen.borrow()[0].body["batch"].as_array().unwrap().len(), 2);
}

#[wasm_bindgen_test]
async fn retries_back_off_with_the_host_timer() {
    // 429s without `Retry-After` back off from `initial_retry_delay` (a 5xx waits at least 5 s).
    let (client, seen) = scripted(ApiVersion::V3, vec![429, 429]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .retry_jitter(false)
        .initial_retry_delay(Duration::from_millis(100))
        .build()
        .await;
    batcher.add(trace_event("a")).await.unwrap();

    let started = Instant::now();
    let response = batcher.flush().await.unwrap();
    let waited = started.elapsed();
    assert_eq!(response.success_count, 1);
    assert_eq!(seen.borrow().len(), 3);
    // 50 ms for `flush` to collect the queue, then 100 ms and 200 ms of backoff.
    assert!(
        (Duration::from_millis(350)..Duration::from_secs(3)).contains(&waited),
        "{waited:?}"
    );
    assert_eq!(batcher.metrics().retries, 2);
}

#[wasm_bindgen_test]
async fn shutdown_sends_what_is_queued() {
    let (client, seen) = scripted(ApiVersion::V4, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    batcher.add(trace_event("a")).await.unwrap();

    let response = batcher.shutdown().await.unwrap();
    assert_eq!(response.success_count, 1);
    let seen = seen.borrow().clone();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path, "/api/public/otel/v1/traces");
}

#[wasm_bindgen_test]
async fn a_trace_without_a_batcher() {
    let (client, seen) = scripted(ApiVersion::V4, vec![]);
    let trace = client.trace().name("wasm").call().await.unwrap();
    // A random v4 UUID (`crypto.getRandomValues`), as an OpenTelemetry trace id: 32 hex digits.
    assert_eq!(trace.id.len(), 32);
    assert!(trace.id.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(seen.borrow().len(), 1);
}

#[wasm_bindgen_test]
async fn validate_checks_the_health_endpoint() {
    let (client, seen) = scripted(ApiVersion::V4, vec![]);
    assert!(client.validate().await.unwrap());
    assert_eq!(seen.borrow()[0].path, "/api/public/health");
}

#[wasm_bindgen_test]
async fn validate_times_out_with_the_host_timer() {
    let service = tower::service_fn(|_request: http::Request<Body>| {
        std::future::pending::<Result<http::Response<Body>, BoxError>>()
    });
    let client = ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .http_client(Client::from_service(service))
        .build()
        .unwrap();
    let started = Instant::now();
    let error = client.validate().await.unwrap_err();
    assert!(error.to_string().contains("timed out"), "{error}");
    assert!(started.elapsed() >= Duration::from_secs(5));
}

// ----- The observation context (runtime-agnostic: entered on every poll) -----

use langfuse_ergonomic::observe::{self, Observation, ObservationType, ObserveFutureExt};

fn observations(seen: &[Seen]) -> Vec<serde_json::Value> {
    seen.iter()
        .filter(|s| s.path == "/api/public/ingestion")
        .flat_map(|s| s.body["batch"].as_array().cloned().unwrap_or_default())
        .filter(|e| e["type"] != "trace-create")
        .map(|e| e["body"].clone())
        .collect()
}

#[wasm_bindgen_test]
async fn observations_nest_across_await_and_spawn_local() {
    let (client, seen) = scripted(ApiVersion::V3, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;

    let root = Observation::builder("root")
        .as_type(ObservationType::Agent)
        .batcher(batcher.clone())
        .start();
    let root_id = root.id().unwrap().to_owned();
    root.clone()
        .instrument(async {
            sleep(Duration::from_millis(5)).await;
            // Started after an await, still inside the root.
            let step = Observation::builder("step").start();
            assert_eq!(step.parent_id(), observe::current().unwrap().id());
            step.end();
            // A task spawned on the host's event loop, carrying the observation.
            let (done_tx, done_rx) = tokio::sync::oneshot::channel();
            wasm_bindgen_futures::spawn_local(
                async move {
                    sleep(Duration::from_millis(1)).await;
                    Observation::builder("spawned")
                        .as_type(ObservationType::Tool)
                        .start()
                        .end();
                    let _ = done_tx.send(());
                }
                .in_current_observation(),
            );
            done_rx.await.unwrap();
        })
        .await;
    assert!(observe::current().is_none());
    batcher.flush().await.unwrap();

    let observations = observations(&seen.borrow());
    assert_eq!(observations.len(), 3, "{observations:#?}");
    for name in ["step", "spawned"] {
        let child = observations.iter().find(|o| o["name"] == name).unwrap();
        assert_eq!(child["parentObservationId"], root_id.as_str());
    }
    let root = observations.iter().find(|o| o["name"] == "root").unwrap();
    assert_eq!(root["type"], "AGENT");
}

#[wasm_bindgen_test]
async fn a_cancelled_observation_still_ends() {
    let (client, seen) = scripted(ApiVersion::V4, vec![]);
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    let observation = Observation::builder("cancelled")
        .batcher(batcher.clone())
        .start();
    let future = observation.instrument(sleep(Duration::from_secs(60)));
    drop(future);
    batcher.flush().await.unwrap();
    let seen = seen.borrow();
    let spans = &seen[0].body["resourceSpans"][0]["scopeSpans"][0]["spans"];
    assert_eq!(spans[0]["name"], "cancelled");
    assert!(spans[0]["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |a| a["key"] == "langfuse.observation.level" && a["value"]["stringValue"] == "WARNING"
        ));
}

#[cfg(feature = "macros")]
mod macros {
    use super::*;
    use langfuse_ergonomic::observe;

    #[observe(as_type = "generation")]
    async fn answer(question: &str) -> Result<String, String> {
        sleep(Duration::from_millis(1)).await;
        Ok(format!("echo {question}"))
    }

    #[observe]
    async fn turn(question: String) -> Result<String, String> {
        answer(&question).await
    }

    #[wasm_bindgen_test]
    async fn the_observe_attribute_works_in_a_javascript_host() {
        let (client, seen) = scripted(ApiVersion::V3, vec![]);
        let batcher = Batcher::builder()
            .client(client)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        observe::install_batcher(batcher.clone());
        assert_eq!(turn("hi".into()).await.unwrap(), "echo hi");
        observe::uninstall();
        batcher.flush().await.unwrap();

        let observations = observations(&seen.borrow());
        let turn = observations.iter().find(|o| o["name"] == "turn").unwrap();
        let answer = observations.iter().find(|o| o["name"] == "answer").unwrap();
        assert_eq!(answer["parentObservationId"], turn["id"]);
        assert_eq!(answer["output"], "echo hi");
        assert_eq!(turn["input"], serde_json::json!({"question": "hi"}));
    }
}
