//! 0.15 additions to the builders and the batcher: observation types, generation details,
//! masking, `Batcher::try_add`, 5xx backoff, and the prompt cache.
#![cfg(not(target_family = "wasm"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod recorder;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use langfuse_ergonomic::prompts::{PromptFallback, PromptSource};
use langfuse_ergonomic::{
    ApiVersion, BackpressurePolicy, Batcher, ClientBuilder, Error, ObservationType,
};
use mockito::Matcher;
use recorder::{attr, Recorder};
use serde_json::{json, Value};

#[tokio::test]
async fn every_observation_type_is_sent_with_its_type_v3_and_v4() {
    let recorder = Recorder::start().await;
    for version in [ApiVersion::V3, ApiVersion::V4] {
        let client = recorder.client(version);
        client
            .agent()
            .trace_id("t-agent")
            .name("agent")
            .call()
            .await
            .unwrap();
        client
            .tool()
            .trace_id("t-agent")
            .name("tool")
            .call()
            .await
            .unwrap();
        client
            .chain()
            .trace_id("t-agent")
            .name("chain")
            .call()
            .await
            .unwrap();
        client
            .retriever()
            .trace_id("t-agent")
            .name("retriever")
            .call()
            .await
            .unwrap();
        client
            .evaluator()
            .trace_id("t-agent")
            .name("evaluator")
            .call()
            .await
            .unwrap();
        client
            .embedding()
            .trace_id("t-agent")
            .name("embedding")
            .model("text-embedding-3-small")
            .usage_details(HashMap::from([("input".to_owned(), 12)]))
            .call()
            .await
            .unwrap();
        client
            .guardrail()
            .trace_id("t-agent")
            .name("guardrail")
            .call()
            .await
            .unwrap();
        client
            .observation()
            .as_type(ObservationType::Span)
            .trace_id("t-agent")
            .name("plain-span")
            .call()
            .await
            .unwrap();
    }

    let v3 = recorder.v3_observations();
    for (name, kind) in [
        ("agent", "AGENT"),
        ("tool", "TOOL"),
        ("chain", "CHAIN"),
        ("retriever", "RETRIEVER"),
        ("evaluator", "EVALUATOR"),
        ("embedding", "EMBEDDING"),
        ("guardrail", "GUARDRAIL"),
    ] {
        let observation = recorder.v3_observation(name);
        // The legacy `observation-create` only takes SPAN, GENERATION and EVENT.
        assert_eq!(
            observation["kind"],
            format!("{}-create", kind.to_lowercase())
        );
        assert!(observation.get("type").is_none(), "{observation:#?}");
        let span = recorder.v4_span(name);
        assert_eq!(
            attr(&span, "langfuse.observation.type"),
            Some(json!(kind.to_lowercase()))
        );
    }
    assert_eq!(recorder.v3_observation("embedding")["usage"]["input"], 12);
    assert!(v3
        .iter()
        .any(|o| o["name"] == "plain-span" && o["kind"] == "span-create"));
}

/// Langfuse rejects an `observation-create` of a new type with a per-event 400 in its 207
/// response; such rejections fail the builder call and show up in the batcher's results.
#[tokio::test]
async fn v3_ingestion_rejections_are_reported() {
    let rejected = |id: &str| {
        json!({"successes": [], "errors": [{
            "id": id,
            "status": 400,
            "message": "Invalid request data",
            "error": "Invalid enum value at body.type"
        }]})
        .to_string()
    };
    let mut server = mockito::Server::new_async().await;
    let client = ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .api_version(ApiVersion::V3)
        .build()
        .unwrap();

    // The builder: the event type and body Langfuse accepts, and its rejection as an error.
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .match_body(Matcher::PartialJson(json!({"batch": [{
            "type": "agent-create",
            "body": {"traceId": "t-reject", "name": "agent"}
        }]})))
        .with_status(207)
        .with_body(rejected("any"))
        .create_async()
        .await;
    let error = client
        .agent()
        .trace_id("t-reject")
        .name("agent")
        .call()
        .await
        .unwrap_err();
    mock.assert_async().await;
    // The builders wrap the error with what they were creating.
    let message = error.to_string();
    assert!(
        message.contains("Partial batch failure: 0 succeeded, 1 failed")
            && message
                .contains("Invalid request data: Invalid enum value at body.type (code: 400)"),
        "{message}"
    );

    // The batcher (here through `observe`): the rejected event is in the shutdown response and
    // the metrics.
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .match_body(Matcher::PartialJson(json!({"batch": [{
            "type": "tool-create",
            "body": {"name": "tool"}
        }]})))
        .with_status(207)
        .with_body(rejected("any"))
        .create_async()
        .await;
    let batcher = Batcher::builder()
        .client(client)
        .flush_interval(Duration::from_secs(3600))
        .max_retries(0)
        .build()
        .await;
    langfuse_ergonomic::observe::Observation::builder("tool")
        .as_type(ObservationType::Tool)
        .batcher(batcher.clone())
        .start()
        .end();
    let metrics = batcher.clone();
    let response = batcher.shutdown().await.unwrap();
    mock.assert_async().await;
    assert_eq!(response.failure_count, 1, "{response:?}");
    assert_eq!(
        response.failures[0].message,
        "Invalid request data: Invalid enum value at body.type"
    );
    assert_eq!(metrics.metrics().failed, 1);
}

#[tokio::test]
async fn generations_send_usage_cost_completion_start_and_prompt() {
    let recorder = Recorder::start().await;
    let start = chrono::Utc::now();
    let first_token = start + chrono::Duration::milliseconds(150);
    for version in [ApiVersion::V3, ApiVersion::V4] {
        recorder
            .client(version)
            .generation()
            .trace_id("t-gen")
            .name("gen")
            .model("gpt-4o")
            .start_time(start)
            .completion_start_time(first_token)
            .usage_details(HashMap::from([
                ("input".to_owned(), 10),
                ("cache_read_input_tokens".to_owned(), 4),
            ]))
            .completion_tokens(20)
            .cost_details(HashMap::from([("total".to_owned(), 0.002)]))
            .prompt_name("support")
            .prompt_version(3)
            .call()
            .await
            .unwrap();
    }
    let generation = recorder.v3_observation("gen");
    assert_eq!(
        generation["usageDetails"],
        json!({"input": 10, "cache_read_input_tokens": 4, "output": 20})
    );
    assert_eq!(generation["costDetails"], json!({"total": 0.002}));
    assert_eq!(generation["promptName"], "support");
    assert_eq!(generation["promptVersion"], 3);
    assert!(generation["completionStartTime"].is_string());

    let span = recorder.v4_span("gen");
    assert_eq!(
        attr(&span, "langfuse.observation.prompt.name"),
        Some(json!("support"))
    );
    assert!(attr(&span, "langfuse.observation.cost_details").is_some());
    assert!(attr(&span, "langfuse.observation.completion_start_time").is_some());
}

#[tokio::test]
async fn the_mask_applies_to_builders_and_batches() {
    let recorder = Recorder::start().await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(recorder.server.url())
        .api_version(ApiVersion::V3)
        .mask(|value| match value {
            Value::Object(mut map) => {
                if map.contains_key("password") {
                    map.insert("password".into(), json!("***"));
                }
                Value::Object(map)
            }
            other => other,
        })
        .build()
        .unwrap();
    client
        .span()
        .trace_id("t-mask")
        .name("login")
        .input(json!({"user": "ada", "password": "hunter2"}))
        .call()
        .await
        .unwrap();
    client
        .trace()
        .name("trace-mask")
        .metadata(json!({"password": "x"}))
        .call()
        .await
        .unwrap();
    assert_eq!(
        recorder.v3_observation("login")["input"],
        json!({"user": "ada", "password": "***"})
    );
    assert_eq!(recorder.v3_traces()[0]["metadata"]["password"], "***");
}

#[tokio::test]
async fn try_add_never_waits_and_reports_a_full_queue() {
    let server = mockito::Server::new_async().await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(server.url())
        .api_version(ApiVersion::V3)
        .build()
        .unwrap();
    let batcher = Batcher::builder()
        .client(client)
        .max_queue_size(1)
        .flush_interval(Duration::from_secs(3600))
        .backpressure_policy(BackpressurePolicy::Block)
        .build()
        .await;
    let event = |id: &str| {
        use langfuse_client_base::models::{
            ingestion_event_trace_event::Type, IngestionEvent, IngestionEventTraceEvent, TraceBody,
        };
        IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
            id.into(),
            "2026-11-01T00:00:00.000Z".into(),
            TraceBody::default(),
            Type::TraceCreate,
        ))
    };
    // The background task drains the channel into the buffer; fill it faster than that.
    let mut rejected = 0;
    for i in 0..200 {
        match batcher.try_add(event(&format!("e{i}"))) {
            Ok(()) => {}
            Err(Error::Backpressure { policy, .. }) => {
                assert_eq!(policy, BackpressurePolicy::Block);
                rejected += 1;
            }
            Err(other) => panic!("{other}"),
        }
    }
    assert!(rejected > 0);
    assert_eq!(batcher.metrics().dropped, rejected);
    // A clone shares the queue and the shutdown.
    let clone = batcher.clone();
    batcher.shutdown().await.ok();
    assert!(matches!(clone.try_add(event("late")), Err(Error::Api(_))));
}

#[tokio::test]
async fn server_errors_back_off_exponentially_and_honour_retry_after() {
    let mut server = mockito::Server::new_async().await;
    let failing = server
        .mock("POST", "/api/public/ingestion")
        .with_status(503)
        .with_header("retry-after", "1")
        .expect(2)
        .create_async()
        .await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(server.url())
        .api_version(ApiVersion::V3)
        .build()
        .unwrap();
    let batcher = Batcher::builder()
        .client(client)
        .max_retries(1)
        .initial_retry_delay(Duration::from_millis(10))
        .retry_jitter(false)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    let event = {
        use langfuse_client_base::models::{
            ingestion_event_trace_event::Type, IngestionEvent, IngestionEventTraceEvent, TraceBody,
        };
        IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
            "e".into(),
            "2026-11-01T00:00:00.000Z".into(),
            TraceBody::default(),
            Type::TraceCreate,
        ))
    };
    batcher.add(event).await.unwrap();
    let started = Instant::now();
    let response = batcher.flush().await.unwrap();
    // The 503's Retry-After (1 s) wins over the 10 ms backoff.
    assert!(
        started.elapsed() >= Duration::from_millis(950),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(response.failure_count, 1);
    failing.assert_async().await;

    // Without Retry-After a 5xx no longer waits a fixed 5 s.
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", "/api/public/ingestion")
        .with_status(500)
        .expect(3)
        .create_async()
        .await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(server.url())
        .api_version(ApiVersion::V3)
        .build()
        .unwrap();
    let batcher = Batcher::builder()
        .client(client)
        .max_retries(2)
        .initial_retry_delay(Duration::from_millis(10))
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;
    let event = {
        use langfuse_client_base::models::{
            ingestion_event_trace_event::Type, IngestionEvent, IngestionEventTraceEvent, TraceBody,
        };
        IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
            "e2".into(),
            "2026-11-01T00:00:00.000Z".into(),
            TraceBody::default(),
            Type::TraceCreate,
        ))
    };
    batcher.add(event).await.unwrap();
    let started = Instant::now();
    batcher.flush().await.unwrap();
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
}

fn text_prompt(version: i64, text: &str) -> String {
    json!({
        "type": "text", "name": "greeting", "version": version, "prompt": text,
        "config": {"temperature": 0.1}, "labels": ["production"], "tags": []
    })
    .to_string()
}

#[tokio::test]
async fn prompts_are_cached_served_stale_on_error_and_fall_back() {
    let mut server = mockito::Server::new_async().await;
    let ok = server
        .mock("GET", "/api/public/v2/prompts/greeting")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(text_prompt(2, "Hello {{name}}"))
        .expect(1)
        .create_async()
        .await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(server.url())
        .build()
        .unwrap();

    let first = client.prompt("greeting").call().await.unwrap();
    assert_eq!(first.source, PromptSource::Api);
    assert_eq!(first.version(), Some(2));
    assert_eq!(
        first.compile([("name", "Ada")]).as_text(),
        Some("Hello Ada")
    );
    assert_eq!(first.config(), Some(&json!({"temperature": 0.1})));
    // Within the TTL: no request (the mock expects exactly one).
    let cached = client
        .clone()
        .prompt("greeting")
        .label("production")
        .call()
        .await
        .unwrap();
    assert_eq!(cached.source, PromptSource::Cache);
    ok.assert_async().await;
    ok.remove_async().await;

    // Expired and the server fails: the stale prompt.
    let failing = server
        .mock("GET", "/api/public/v2/prompts/greeting")
        .match_query(Matcher::Any)
        .with_status(500)
        .expect(3)
        .create_async()
        .await;
    let stale = client
        .prompt("greeting")
        .cache_ttl(Duration::from_nanos(1))
        .call()
        .await
        .unwrap();
    assert_eq!(stale.source, PromptSource::StaleCache);
    assert_eq!(stale.version(), Some(2));

    // Not cached and failing: the fallback, or the error.
    client.clear_prompt_cache();
    let fallback = client
        .prompt("greeting")
        .fallback(PromptFallback::text("Hi {{name}}"))
        .call()
        .await
        .unwrap();
    assert!(fallback.is_fallback());
    assert_eq!(
        fallback.compile([("name", "Bob")]).as_text(),
        Some("Hi Bob")
    );
    assert!(client.prompt("greeting").call().await.is_err());
    failing.assert_async().await;
}

#[tokio::test]
async fn updating_labels_sends_only_the_set_lists_and_drops_cached_entries() {
    let mut server = mockito::Server::new_async().await;
    let get = server
        .mock("GET", "/api/public/v2/prompts/greeting")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(text_prompt(2, "Hello {{name}}"))
        .expect(2)
        .create_async()
        .await;
    let patch = server
        .mock("PATCH", "/api/public/v2/prompts/greeting/versions/3")
        .match_body(Matcher::Json(
            json!({"newLabels": ["production"], "removeLabels": ["staging"]}),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(text_prompt(3, "Hi {{name}}"))
        .expect(1)
        .create_async()
        .await;
    let remove_only = server
        .mock("PATCH", "/api/public/v2/prompts/greeting/versions/1")
        .match_body(Matcher::Json(json!({"removeLabels": ["old"]})))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(text_prompt(1, "Yo"))
        .expect(1)
        .create_async()
        .await;
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(server.url())
        .build()
        .unwrap();

    assert_eq!(
        client.prompt("greeting").call().await.unwrap().source,
        PromptSource::Api
    );
    client
        .update_prompt_version()
        .name("greeting")
        .version(3)
        .labels(vec!["production".to_string()])
        .remove_labels(vec!["staging".to_string()])
        .call()
        .await
        .unwrap();
    // `production` may now resolve to another version: fetched again, not from the cache.
    assert_eq!(
        client.prompt("greeting").call().await.unwrap().source,
        PromptSource::Api
    );
    client
        .update_prompt_version()
        .name("greeting")
        .version(1)
        .remove_labels(vec!["old".to_string()])
        .call()
        .await
        .unwrap();

    get.assert_async().await;
    patch.assert_async().await;
    remove_only.assert_async().await;
}
