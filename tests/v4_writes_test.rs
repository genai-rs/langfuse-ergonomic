//! Mock-server tests for the Langfuse v4 write path (OTLP spans and the scores API), and for
//! the version setting.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use langfuse_client_base::models::{
    ingestion_event_create_span_event, ingestion_event_score_event, ingestion_event_trace_event,
    CreateScoreValue, CreateSpanBody, IngestionEvent, IngestionEventCreateSpanEvent,
    IngestionEventScoreEvent, IngestionEventTraceEvent, ScoreBody, TraceBody,
};
use langfuse_ergonomic::{otel, ApiVersion, Batcher, ClientBuilder, Error, LangfuseClient};
use mockito::{Mock, Server, ServerGuard};
use serde_json::{json, Value};

const OTLP_PATH: &str = "/api/public/otel/v1/traces";

fn v4_client(server: &ServerGuard) -> LangfuseClient {
    ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .unwrap()
}

/// Requests recorded by a capturing mock: headers we care about plus the JSON body.
#[derive(Clone, Default)]
struct Recorded(Arc<Mutex<Vec<RecordedRequest>>>);

type RecordedRequest = (HashMap<String, String>, Value);

impl Recorded {
    fn bodies(&self) -> Vec<Value> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .map(|(_, b)| b.clone())
            .collect()
    }

    fn headers(&self, index: usize) -> HashMap<String, String> {
        self.0.lock().unwrap()[index].0.clone()
    }
}

async fn capture(
    server: &mut ServerGuard,
    path: &str,
    status: usize,
    response: &'static str,
    hits: usize,
) -> (Mock, Recorded) {
    let recorded = Recorded::default();
    let sink = recorded.clone();
    let mock = server
        .mock("POST", path)
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body_from_request(move |request| {
            let headers = [
                "content-type",
                "x-langfuse-ingestion-version",
                "authorization",
            ]
            .into_iter()
            .filter_map(|name| {
                request
                    .header(name)
                    .first()
                    .map(|v| (name.to_owned(), v.to_str().unwrap().to_owned()))
            })
            .collect();
            let body = serde_json::from_slice(request.body().unwrap()).unwrap();
            sink.0.lock().unwrap().push((headers, body));
            response.as_bytes().to_vec()
        })
        .expect(hits)
        .create_async()
        .await;
    (mock, recorded)
}

fn spans(body: &Value) -> Vec<Value> {
    body["resourceSpans"][0]["scopeSpans"][0]["spans"]
        .as_array()
        .unwrap()
        .clone()
}

fn attr<'a>(span: &'a Value, key: &str) -> Option<&'a Value> {
    span["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["key"] == key)
        .map(|a| &a["value"])
}

fn string_attr(span: &Value, key: &str) -> String {
    attr(span, key).unwrap_or_else(|| panic!("missing {key} in {span}"))["stringValue"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn default_api_version_is_v4() {
    let client = ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .build()
        .unwrap();
    assert_eq!(client.api_version(), ApiVersion::V4);
    let v3 = client.with_api_version(ApiVersion::V3);
    assert_eq!(v3.api_version(), ApiVersion::V3);
}

#[test]
fn api_version_from_env() {
    // The only test in this binary that touches the environment.
    std::env::set_var("LANGFUSE_PUBLIC_KEY", "pk");
    std::env::set_var("LANGFUSE_SECRET_KEY", "sk");

    std::env::set_var("LANGFUSE_API_VERSION", "v3");
    let client = ClientBuilder::from_env().unwrap().build().unwrap();
    assert_eq!(client.api_version(), ApiVersion::V3);

    // An explicit builder setting overrides the environment.
    let client = ClientBuilder::from_env()
        .unwrap()
        .api_version(ApiVersion::V4)
        .build()
        .unwrap();
    assert_eq!(client.api_version(), ApiVersion::V4);

    std::env::set_var("LANGFUSE_API_VERSION", "v9");
    assert!(matches!(
        ClientBuilder::from_env(),
        Err(Error::Configuration(_))
    ));

    std::env::remove_var("LANGFUSE_API_VERSION");
    let client = ClientBuilder::from_env().unwrap().build().unwrap();
    assert_eq!(client.api_version(), ApiVersion::V4);

    std::env::remove_var("LANGFUSE_PUBLIC_KEY");
    std::env::remove_var("LANGFUSE_SECRET_KEY");
}

#[tokio::test]
async fn trace_is_sent_as_otlp_root_span_with_v4_header() {
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 1).await;
    let ingestion = server
        .mock("POST", "/api/public/ingestion")
        .expect(0)
        .create_async()
        .await;

    let client = v4_client(&server);
    let trace = client
        .trace()
        .id("my-trace")
        .name("chat")
        .user_id("user-1")
        .session_id("session-1")
        .tags(vec!["a".to_owned()])
        .input(json!({"q": "hi"}))
        .metadata(json!({"team": "x"}))
        .release("r1")
        .version("v1")
        .call()
        .await
        .unwrap();

    assert_eq!(trace.id, otel::trace_id("my-trace"));
    assert_eq!(trace.url(), format!("{}/trace/{}", server.url(), trace.id));
    otlp.assert_async().await;
    ingestion.assert_async().await;

    let headers = recorded.headers(0);
    assert_eq!(headers["x-langfuse-ingestion-version"], "4");
    assert_eq!(headers["content-type"], "application/json");
    assert!(headers["authorization"].starts_with("Basic "));

    let body = &recorded.bodies()[0];
    assert_eq!(
        body["resourceSpans"][0]["scopeSpans"][0]["scope"]["name"],
        "langfuse-ergonomic"
    );
    let spans = spans(body);
    assert_eq!(spans.len(), 1);
    let root = &spans[0];
    assert_eq!(root["traceId"], trace.id);
    assert_eq!(root["spanId"], otel::trace_root_span_id("my-trace"));
    assert!(root.get("parentSpanId").is_none());
    assert_eq!(string_attr(root, "langfuse.trace.name"), "chat");
    assert_eq!(string_attr(root, "user.id"), "user-1");
    assert_eq!(string_attr(root, "session.id"), "session-1");
    assert_eq!(string_attr(root, "langfuse.trace.metadata.team"), "x");
    assert_eq!(
        string_attr(root, "langfuse.observation.input"),
        r#"{"q":"hi"}"#
    );
    assert_eq!(string_attr(root, "langfuse.release"), "r1");
    assert_eq!(string_attr(root, "langfuse.version"), "v1");
    assert_eq!(
        attr(root, "langfuse.trace.tags").unwrap()["arrayValue"]["values"][0]["stringValue"],
        "a"
    );
}

#[tokio::test]
async fn observations_link_to_parent_and_unknown_trace_root() {
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 3).await;
    let client = v4_client(&server);

    let span_id = client
        .span()
        .trace_id("my-trace")
        .id("retrieval")
        .name("retrieve")
        .call()
        .await
        .unwrap();
    let generation_id = client
        .generation()
        .trace_id("my-trace")
        .parent_observation_id("retrieval")
        .name("llm")
        .model("gpt-4o")
        .model_parameters(json!({"temperature": 0.2, "max_tokens": 100}))
        .prompt_tokens(10)
        .completion_tokens(5)
        .call()
        .await
        .unwrap();
    let event_id = client
        .event()
        .trace_id("my-trace")
        .parent_observation_id(span_id.clone())
        .name("cache-hit")
        .level("WARNING")
        .call()
        .await
        .unwrap();

    otlp.assert_async().await;
    assert_eq!(span_id, otel::observation_id("retrieval"));
    assert_eq!(generation_id.len(), 16);
    assert_eq!(event_id.len(), 16);

    let bodies = recorded.bodies();
    let span = &spans(&bodies[0])[0];
    let generation = &spans(&bodies[1])[0];
    let event = &spans(&bodies[2])[0];

    let trace_hex = otel::trace_id("my-trace");
    for s in [span, generation, event] {
        assert_eq!(s["traceId"], trace_hex);
    }
    assert_eq!(span["spanId"], span_id);
    // This client never called trace() for "my-trace", so the first parentless
    // observation becomes the root.
    assert!(span.get("parentSpanId").is_none());
    assert_eq!(string_attr(span, "langfuse.observation.type"), "span");

    assert_eq!(generation["spanId"], generation_id);
    // The caller's original parent id and the returned mapped id resolve to the same span.
    assert_eq!(generation["parentSpanId"], span_id);
    assert_eq!(event["parentSpanId"], span_id);
    assert_eq!(
        string_attr(generation, "langfuse.observation.type"),
        "generation"
    );
    assert_eq!(
        string_attr(generation, "langfuse.observation.model.name"),
        "gpt-4o"
    );
    let usage: Value = serde_json::from_str(&string_attr(
        generation,
        "langfuse.observation.usage_details",
    ))
    .unwrap();
    assert_eq!(usage, json!({"input": 10, "output": 5}));
    let params: Value = serde_json::from_str(&string_attr(
        generation,
        "langfuse.observation.model.parameters",
    ))
    .unwrap();
    assert_eq!(params, json!({"temperature": 0.2, "max_tokens": 100}));

    assert_eq!(string_attr(event, "langfuse.observation.type"), "event");
    assert_eq!(string_attr(event, "langfuse.observation.level"), "WARNING");
    assert_eq!(event["startTimeUnixNano"], event["endTimeUnixNano"]);
}

#[tokio::test]
async fn score_uses_scores_api_with_mapped_ids() {
    let mut server = Server::new_async().await;
    let (scores, recorded) =
        capture(&mut server, "/api/public/scores", 200, r#"{"id":"s"}"#, 1).await;
    let ingestion = server
        .mock("POST", "/api/public/ingestion")
        .expect(0)
        .create_async()
        .await;
    let client = v4_client(&server);

    let score_id = client
        .score()
        .trace_id("my-trace")
        .observation_id("retrieval")
        .name("quality")
        .value(0.8)
        .comment("good")
        .call()
        .await
        .unwrap();

    scores.assert_async().await;
    ingestion.assert_async().await;
    let body = &recorded.bodies()[0];
    assert_eq!(body["id"], score_id);
    assert_eq!(body["traceId"], otel::trace_id("my-trace"));
    assert_eq!(body["observationId"], otel::observation_id("retrieval"));
    assert_eq!(body["name"], "quality");
    assert_eq!(body["value"], 0.8);
    assert_eq!(body["dataType"], "NUMERIC");
    assert_eq!(body["comment"], "good");
}

#[tokio::test]
async fn v4_errors_keep_status_and_retry_after() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("POST", OTLP_PATH)
        .with_status(429)
        .with_header("retry-after", "7")
        .with_header("x-request-id", "req-1")
        .create_async()
        .await;
    let client = v4_client(&server);
    let err = client
        .trace()
        .name("t")
        .call()
        .await
        .err()
        .expect("429 must fail");
    match err {
        Error::RateLimit {
            retry_after,
            request_id,
        } => {
            assert_eq!(retry_after, Some(Duration::from_secs(7)));
            assert_eq!(request_id.as_deref(), Some("req-1"));
        }
        other => panic!("unexpected error {other:?}"),
    }
}

#[tokio::test]
async fn v3_client_keeps_using_ingestion() {
    let mut server = Server::new_async().await;
    let ingestion = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes":[],"errors":[]}"#)
        .expect(1)
        .create_async()
        .await;
    let otlp = server
        .mock("POST", OTLP_PATH)
        .expect(0)
        .create_async()
        .await;
    let client = v4_client(&server).with_api_version(ApiVersion::V3);
    let trace = client.trace().id("my-trace").call().await.unwrap();
    assert_eq!(trace.id, "my-trace");
    ingestion.assert_async().await;
    otlp.assert_async().await;
}

fn trace_event(id: &str) -> IngestionEvent {
    IngestionEvent::TraceEvent(IngestionEventTraceEvent::new(
        format!("ev-{id}"),
        "2026-11-10T12:00:00.000Z".to_owned(),
        TraceBody {
            id: Some(Some(id.to_owned())),
            name: Some(Some(format!("trace {id}"))),
            ..TraceBody::default()
        },
        ingestion_event_trace_event::Type::TraceCreate,
    ))
}

fn span_event(id: &str, trace: &str) -> IngestionEvent {
    IngestionEvent::CreateSpanEvent(IngestionEventCreateSpanEvent::new(
        format!("ev-{id}"),
        "2026-11-10T12:00:00.000Z".to_owned(),
        CreateSpanBody {
            id: Some(Some(id.to_owned())),
            trace_id: Some(Some(trace.to_owned())),
            ..CreateSpanBody::default()
        },
        ingestion_event_create_span_event::Type::SpanCreate,
    ))
}

fn score_event(id: &str, trace: &str) -> IngestionEvent {
    let mut body = ScoreBody::new("quality".to_owned(), CreateScoreValue::DoubleNumber(1.0));
    body.id = Some(Some(id.to_owned()));
    body.trace_id = Some(Some(trace.to_owned()));
    IngestionEvent::ScoreEvent(IngestionEventScoreEvent::new(
        format!("ev-{id}"),
        "2026-11-10T12:00:00.000Z".to_owned(),
        body,
        ingestion_event_score_event::Type::ScoreCreate,
    ))
}

async fn v4_batcher(server: &ServerGuard, max_retries: u32) -> Batcher {
    Batcher::builder()
        .client(v4_client(server))
        .max_retries(max_retries)
        .initial_retry_delay(Duration::from_millis(1))
        .max_retry_delay(Duration::from_millis(5))
        .retry_jitter(false)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await
}

#[tokio::test]
async fn batcher_sends_spans_in_one_export_and_scores_separately() {
    let mut server = Server::new_async().await;
    let (otlp, otlp_recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 1).await;
    let (scores, score_recorded) =
        capture(&mut server, "/api/public/scores", 200, r#"{"id":"x"}"#, 1).await;

    let batcher = v4_batcher(&server, 1).await;
    batcher.add(trace_event("t1")).await.unwrap();
    batcher.add(span_event("s1", "t1")).await.unwrap();
    batcher.add(score_event("sc1", "t1")).await.unwrap();
    let result = batcher.shutdown().await.unwrap();

    assert_eq!(result.failure_count, 0);
    assert_eq!(result.success_count, 3);
    otlp.assert_async().await;
    scores.assert_async().await;

    let exported = spans(&otlp_recorded.bodies()[0]);
    assert_eq!(exported.len(), 2);
    assert_eq!(exported[1]["parentSpanId"], exported[0]["spanId"]);
    assert_eq!(
        otlp_recorded.headers(0)["x-langfuse-ingestion-version"],
        "4"
    );
    assert_eq!(score_recorded.bodies()[0]["traceId"], otel::trace_id("t1"));
}

#[tokio::test]
async fn batcher_retries_otlp_after_rate_limit() {
    let mut server = Server::new_async().await;
    let limited = server
        .mock("POST", OTLP_PATH)
        .with_status(429)
        .with_header("retry-after", "0")
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("POST", OTLP_PATH)
        .with_status(200)
        .with_body("{}")
        .expect(1)
        .create_async()
        .await;

    let batcher = v4_batcher(&server, 2).await;
    batcher.add(trace_event("t1")).await.unwrap();
    let result = batcher.shutdown().await.unwrap();

    assert_eq!(result.success_count, 1);
    assert_eq!(result.failure_count, 0);
    limited.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn batcher_splits_otlp_export_on_413() {
    let mut server = Server::new_async().await;
    let sizes = Arc::new(Mutex::new(Vec::new()));
    let sink = sizes.clone();
    let mock = server
        .mock("POST", OTLP_PATH)
        .with_status_code_from_request(move |request| {
            let body: Value = serde_json::from_slice(request.body().unwrap()).unwrap();
            let count = spans(&body).len();
            sink.lock().unwrap().push(count);
            if count > 1 {
                413
            } else {
                200
            }
        })
        .expect(3)
        .create_async()
        .await;

    let batcher = v4_batcher(&server, 1).await;
    batcher.add(trace_event("t1")).await.unwrap();
    batcher.add(trace_event("t2")).await.unwrap();
    let result = batcher.shutdown().await.unwrap();

    assert_eq!(result.success_count, 2);
    assert_eq!(result.failure_count, 0);
    assert_eq!(*sizes.lock().unwrap(), vec![2, 1, 1]);
    mock.assert_async().await;
}

#[tokio::test]
async fn batcher_reports_failed_scores_per_event() {
    let mut server = Server::new_async().await;
    let _scores = server
        .mock("POST", "/api/public/scores")
        .with_status(400)
        .with_body(r#"{"message":"bad score"}"#)
        .expect(1)
        .create_async()
        .await;

    let batcher = v4_batcher(&server, 1).await;
    batcher.add(score_event("sc1", "t1")).await.unwrap();
    let result = batcher.shutdown().await.unwrap();

    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 1);
    assert_eq!(result.failures[0].event_id, "ev-sc1");
    assert_eq!(result.failures[0].code.as_deref(), Some("400"));
    assert!(!result.failures[0].retryable);
}

#[tokio::test]
async fn batcher_rejects_unencodable_events_in_v4() {
    let server = Server::new_async().await;
    let batcher = v4_batcher(&server, 1).await;
    let mut event = span_event("s1", "t1");
    if let IngestionEvent::CreateSpanEvent(e) = &mut event {
        e.body.trace_id = None;
    }
    assert!(matches!(
        batcher.add(event).await,
        Err(Error::Validation(_))
    ));
    batcher.shutdown().await.unwrap();
}

#[tokio::test]
async fn update_resends_the_full_span_with_the_original_start() {
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 3).await;
    let client = v4_client(&server);
    let start = chrono::DateTime::parse_from_rfc3339("2026-11-10T12:00:00.000Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    client.trace().id("t1").name("chat").call().await.unwrap();
    let span_id = client
        .span()
        .trace_id("t1")
        .id("s1")
        .name("retrieve")
        .input(json!({"q": "hi"}))
        .start_time(start)
        .call()
        .await
        .unwrap();
    let updated_id = client
        .update_span()
        .id("s1")
        .trace_id("t1")
        .output(json!({"done": true}))
        .end_time(start + chrono::Duration::seconds(2))
        .call()
        .await
        .unwrap();
    otlp.assert_async().await;
    assert_eq!(updated_id, span_id);

    let bodies = recorded.bodies();
    let created = &spans(&bodies[1])[0];
    let updated = &spans(&bodies[2])[0];
    assert_eq!(updated["spanId"], span_id);
    assert_eq!(updated["startTimeUnixNano"], created["startTimeUnixNano"]);
    assert_eq!(updated["startTimeUnixNano"], "1794312000000000000");
    assert_eq!(updated["endTimeUnixNano"], "1794312002000000000");
    assert_eq!(updated["name"], "retrieve");
    assert_eq!(updated["parentSpanId"], created["parentSpanId"]);
    assert_eq!(
        string_attr(updated, "langfuse.observation.input"),
        r#"{"q":"hi"}"#
    );
    assert_eq!(
        string_attr(updated, "langfuse.observation.output"),
        r#"{"done":true}"#
    );
}

#[tokio::test]
async fn update_of_an_unknown_span_fails_without_a_request() {
    let mut server = Server::new_async().await;
    let otlp = server
        .mock("POST", OTLP_PATH)
        .expect(0)
        .create_async()
        .await;
    let err = v4_client(&server)
        .update_span()
        .id("never-created")
        .trace_id("t1")
        .output(json!("x"))
        .call()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "{err:?}");
    otlp.assert_async().await;
}

#[tokio::test]
async fn trace_attributes_are_stamped_on_child_spans() {
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 3).await;
    let client = v4_client(&server);

    client
        .trace()
        .id("t1")
        .user_id("user-1")
        .session_id("session-1")
        .tags(vec!["a".to_owned()])
        .release("r1")
        .call()
        .await
        .unwrap();
    client
        .span()
        .trace_id("t1")
        .name("child")
        .call()
        .await
        .unwrap();
    // A trace created elsewhere: the builder setters supply the trace attributes.
    client
        .generation()
        .trace_id("other-trace")
        .user_id("user-2")
        .session_id("session-2")
        .tags(vec!["b".to_owned()])
        .call()
        .await
        .unwrap();
    otlp.assert_async().await;

    let bodies = recorded.bodies();
    let root = &spans(&bodies[0])[0];
    let child = &spans(&bodies[1])[0];
    assert_eq!(child["parentSpanId"], root["spanId"]);
    assert_eq!(string_attr(child, "user.id"), "user-1");
    assert_eq!(string_attr(child, "session.id"), "session-1");
    assert_eq!(string_attr(child, "langfuse.release"), "r1");
    assert_eq!(
        attr(child, "langfuse.trace.tags").unwrap()["arrayValue"]["values"][0]["stringValue"],
        "a"
    );
    let other = &spans(&bodies[2])[0];
    assert_eq!(string_attr(other, "user.id"), "user-2");
    assert_eq!(string_attr(other, "session.id"), "session-2");
    assert_eq!(
        attr(other, "langfuse.trace.tags").unwrap()["arrayValue"]["values"][0]["stringValue"],
        "b"
    );
}

#[tokio::test]
async fn v3_observation_trace_setters_upsert_the_trace() {
    let mut server = Server::new_async().await;
    let (ingestion, recorded) = capture(
        &mut server,
        "/api/public/ingestion",
        200,
        r#"{"successes":[],"errors":[]}"#,
        1,
    )
    .await;
    let client = v4_client(&server).with_api_version(ApiVersion::V3);
    client
        .span()
        .trace_id("t1")
        .user_id("user-1")
        .call()
        .await
        .unwrap();
    ingestion.assert_async().await;
    let batch = &recorded.bodies()[0]["batch"];
    assert_eq!(batch[0]["type"], "trace-create");
    assert_eq!(batch[0]["body"]["id"], "t1");
    assert_eq!(batch[0]["body"]["userId"], "user-1");
    assert_eq!(batch[1]["type"], "span-create");
}

#[tokio::test]
async fn batcher_sends_one_merged_span_for_create_and_update() {
    use langfuse_client_base::models::{
        ingestion_event_update_span_event, IngestionEventUpdateSpanEvent, UpdateSpanBody,
    };
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 1).await;
    let batcher = v4_batcher(&server, 1).await;

    batcher.add(span_event("s1", "t1")).await.unwrap();
    let mut update = UpdateSpanBody::new("s1".to_owned());
    update.trace_id = Some(Some("t1".to_owned()));
    update.output = Some(Some(json!("done")));
    batcher
        .add(IngestionEvent::UpdateSpanEvent(
            IngestionEventUpdateSpanEvent::new(
                "ev-s1-update".to_owned(),
                "2026-11-10T12:00:05.000Z".to_owned(),
                update,
                ingestion_event_update_span_event::Type::SpanUpdate,
            ),
        ))
        .await
        .unwrap();
    let result = batcher.shutdown().await.unwrap();
    assert_eq!(result.success_count, 2);
    otlp.assert_async().await;

    let exported = spans(&recorded.bodies()[0]);
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0]["startTimeUnixNano"], "1794312000000000000");
    assert_eq!(
        string_attr(&exported[0], "langfuse.observation.output"),
        "done"
    );
}

#[tokio::test]
async fn batcher_waits_for_retry_after_on_score_rate_limit() {
    let mut server = Server::new_async().await;
    let limited = server
        .mock("POST", "/api/public/scores")
        .with_status(429)
        .with_header("retry-after", "1")
        .expect(1)
        .create_async()
        .await;
    let ok = server
        .mock("POST", "/api/public/scores")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"x"}"#)
        .expect(2)
        .create_async()
        .await;

    let batcher = v4_batcher(&server, 2).await;
    batcher.add(score_event("sc1", "t1")).await.unwrap();
    batcher.add(score_event("sc2", "t1")).await.unwrap();
    let started = std::time::Instant::now();
    let result = batcher.shutdown().await.unwrap();

    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "Retry-After ignored"
    );
    assert_eq!(result.success_count, 2);
    assert_eq!(result.failure_count, 0);
    limited.assert_async().await;
    ok.assert_async().await;
}

#[tokio::test]
async fn batcher_stops_scores_when_still_rate_limited() {
    let mut server = Server::new_async().await;
    let limited = server
        .mock("POST", "/api/public/scores")
        .with_status(429)
        .with_header("retry-after", "0")
        .expect(1)
        .create_async()
        .await;

    // max_retries(0): no retry, so the remaining scores are not sent at all.
    let batcher = v4_batcher(&server, 0).await;
    batcher.add(score_event("sc1", "t1")).await.unwrap();
    batcher.add(score_event("sc2", "t1")).await.unwrap();
    let result = batcher.shutdown().await.unwrap();

    assert_eq!(result.failure_count, 2);
    assert!(result.failures.iter().all(|f| f.retryable));
    assert!(result
        .failures
        .iter()
        .all(|f| f.code.as_deref() == Some("429")));
    limited.assert_async().await;
}

#[tokio::test]
async fn batcher_fails_fast_on_score_auth_error() {
    let mut server = Server::new_async().await;
    let denied = server
        .mock("POST", "/api/public/scores")
        .with_status(401)
        .with_body(r#"{"message":"invalid key"}"#)
        .expect(1)
        .create_async()
        .await;

    let batcher = v4_batcher(&server, 2).await;
    batcher.add(score_event("sc1", "t1")).await.unwrap();
    batcher.add(score_event("sc2", "t1")).await.unwrap();
    let err = batcher.shutdown().await.unwrap_err();

    assert!(matches!(err, Error::Auth { .. }), "{err:?}");
    denied.assert_async().await;
}

#[tokio::test]
async fn batcher_does_not_record_rejected_events() {
    let mut server = Server::new_async().await;
    let (otlp, recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 1).await;
    let batcher = Batcher::builder()
        .client(v4_client(&server))
        .max_bytes(2_000)
        .flush_interval(Duration::from_secs(3600))
        .build()
        .await;

    // Too large: rejected before it is queued, so it must not become the trace root.
    let mut big = span_event("big", "t1");
    if let IngestionEvent::CreateSpanEvent(e) = &mut big {
        e.body.input = Some(Some(json!("x".repeat(5_000))));
    }
    assert!(matches!(
        batcher.add(big).await,
        Err(Error::BatchSizeExceeded { .. })
    ));
    batcher.add(span_event("small", "t1")).await.unwrap();
    batcher.shutdown().await.unwrap();
    otlp.assert_async().await;

    let exported = spans(&recorded.bodies()[0]);
    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0]["spanId"], otel::observation_id("small"));
    assert!(exported[0].get("parentSpanId").is_none());
}

#[tokio::test]
async fn evicted_spans_cannot_be_updated() {
    let mut server = Server::new_async().await;
    let (_otlp, _recorded) = capture(&mut server, OTLP_PATH, 200, "{}", 2).await;
    let client = ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .v4_state_capacity(1)
        .build()
        .unwrap();
    client.span().trace_id("t1").id("s1").call().await.unwrap();
    client.span().trace_id("t1").id("s2").call().await.unwrap();
    let err = client
        .update_span()
        .id("s1")
        .trace_id("t1")
        .output(json!("late"))
        .call()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Validation(_)), "{err:?}");
}
