//! `#[observe]` and the observation context, against a mock Langfuse (v3 and v4).
#![cfg(all(feature = "macros", not(target_family = "wasm")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod recorder;

use std::time::Duration;

use futures_util::StreamExt;
use langfuse_ergonomic::observe::{ObservationType, ObserveFutureExt};
use langfuse_ergonomic::{observe, ApiVersion, Batcher};
use recorder::{attr, Recorder, GLOBAL_SINK};
use serde_json::{json, Value};

#[derive(Debug, serde::Serialize)]
struct Query {
    text: String,
}

/// Only `Debug`.
#[allow(dead_code)]
#[derive(Debug)]
struct Opaque(u8);

/// Neither `Debug` nor `Serialize`.
struct Hidden;

#[observe(as_type = "retriever")]
async fn retrieve(query: &Query, limit: usize) -> Vec<String> {
    (0..limit).map(|i| format!("{}-{i}", query.text)).collect()
}

#[observe(as_type = "generation", model = "gpt-test", name = "llm-call")]
async fn generate(prompt: String) -> Result<String, std::io::Error> {
    observe::update_current(|o| o.set_usage_details([("input", 3), ("output", 5)]));
    Ok(format!("answer to {prompt}"))
}

#[observe]
fn tokenize(text: &str) -> usize {
    text.split_whitespace().count()
}

#[observe(user_id = "user-7", session_id = format!("session-{}", 1), tags = ["a", "b"], metadata = json!({"run": 1}))]
async fn pipeline(question: String) -> Result<String, std::io::Error> {
    let tokens = tokenize(&question);
    let docs = retrieve(
        &Query {
            text: question.clone(),
        },
        tokens,
    )
    .await;
    tokio::time::sleep(Duration::from_millis(1)).await;
    let answer = generate(docs.join(",")).await?;
    observe::score_current("quality", 0.9);
    Ok(answer)
}

#[observe(err)]
async fn failing(fail: bool) -> Result<u32, String> {
    if fail {
        return Err("it broke".to_owned());
    }
    Ok(1)
}

#[observe]
async fn question_mark(input: &str) -> Result<u32, std::num::ParseIntError> {
    let value: u32 = input.parse()?;
    Ok(value + 1)
}

#[observe(skip(secret), capture_output = false)]
fn skipping(secret: &str, visible: u8, opaque: Opaque, hidden: Hidden) -> String {
    let _ = hidden;
    format!("{secret}{visible}{opaque:?}")
}

#[observe(capture_input = false, level = "DEBUG")]
fn no_input(value: u8) -> u8 {
    value
}

#[observe]
fn panics() -> u8 {
    panic!("boom")
}

#[observe]
async fn slow() {
    tokio::time::sleep(Duration::from_secs(60)).await;
}

#[observe]
async fn spawns_child() -> u32 {
    let handle = tokio::spawn(child_task().in_current_observation());
    handle.await.unwrap()
}

#[observe(as_type = "tool")]
async fn child_task() -> u32 {
    tokio::task::yield_now().await;
    7
}

#[observe(as_type = "generation", stream)]
async fn streaming(n: u32) -> impl futures_core::Stream<Item = Result<u32, String>> {
    futures_util::stream::iter((0..n).map(|i| {
        if i == 2 {
            Err("bad item".to_owned())
        } else {
            Ok(i)
        }
    }))
}

#[observe(stream = "iterator")]
fn counting(n: u32) -> impl Iterator<Item = u32> {
    0..n
}

struct Service {
    prefix: String,
}

impl Service {
    #[observe(name = "service-call")]
    async fn call(&self, value: u32) -> String {
        format!("{}{value}", self.prefix)
    }

    #[observe]
    fn borrowed(&self) -> &str {
        &self.prefix
    }
}

async fn setup(version: ApiVersion) -> (Recorder, Batcher) {
    let recorder = Recorder::start().await;
    let batcher = recorder.batcher(recorder.client(version)).await;
    observe::set_sample_rate(1.0);
    observe::install_batcher(batcher.clone());
    (recorder, batcher)
}

async fn finish(batcher: Batcher) {
    observe::uninstall();
    batcher.flush().await.unwrap();
}

fn by_name<'a>(observations: &'a [Value], name: &str) -> &'a Value {
    observations
        .iter()
        .find(|o| o["name"] == name)
        .unwrap_or_else(|| panic!("no {name} in {observations:#?}"))
}

#[tokio::test]
async fn nested_async_and_sync_observations_form_one_trace_v3() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    let answer = pipeline("what is rust".into()).await.unwrap();
    assert!(answer.starts_with("answer to"));
    finish(batcher).await;

    let observations = recorder.v3_observations();
    assert_eq!(observations.len(), 4, "{observations:#?}");
    let root = by_name(&observations, "pipeline");
    let tokenize = by_name(&observations, "tokenize");
    let retrieve = by_name(&observations, "retrieve");
    let generate = by_name(&observations, "llm-call");

    let trace_id = root["traceId"].as_str().unwrap();
    assert_eq!(trace_id.len(), 32);
    assert!(root.get("parentObservationId").is_none_or(Value::is_null));
    for child in [tokenize, retrieve, generate] {
        assert_eq!(child["traceId"], trace_id);
        assert_eq!(child["parentObservationId"], root["id"], "{child:#?}");
    }
    assert_eq!(root["kind"], "span-create");
    assert_eq!(retrieve["kind"], "observation-create");
    assert_eq!(retrieve["type"], "RETRIEVER");
    assert_eq!(generate["kind"], "generation-create");
    assert_eq!(generate["model"], "gpt-test");
    assert_eq!(generate["usageDetails"], json!({"input": 3, "output": 5}));

    // Inputs: an object keyed by argument name; outputs the return value (Ok unwrapped).
    assert_eq!(root["input"], json!({"question": "what is rust"}));
    assert_eq!(root["output"], json!(answer));
    assert_eq!(
        retrieve["input"],
        json!({"query": {"text": "what is rust"}, "limit": 3})
    );
    assert_eq!(
        retrieve["output"],
        json!(["what is rust-0", "what is rust-1", "what is rust-2"])
    );
    assert_eq!(tokenize["output"], json!(3));
    assert_eq!(root["metadata"], json!({"run": 1}));
    assert!(root["endTime"].is_string());

    // The root sends the trace with the trace attributes and its input and output.
    let traces = recorder.v3_traces();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0]["id"], trace_id);
    assert_eq!(traces[0]["name"], "pipeline");
    assert_eq!(traces[0]["userId"], "user-7");
    assert_eq!(traces[0]["sessionId"], "session-1");
    assert_eq!(traces[0]["tags"], json!(["a", "b"]));
    assert_eq!(traces[0]["output"], json!(answer));

    // score_current scores the current observation.
    let scores = recorder.v3_scores();
    assert_eq!(scores.len(), 1);
    assert_eq!(scores[0]["observationId"], root["id"]);
    assert_eq!(scores[0]["traceId"], trace_id);
    assert_eq!(scores[0]["value"], 0.9);
}

#[tokio::test]
async fn nested_observations_are_child_spans_v4() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V4).await;

    pipeline("hello world".into()).await.unwrap();
    finish(batcher).await;

    let root = recorder.v4_span("pipeline");
    let generate = recorder.v4_span("llm-call");
    let retrieve = recorder.v4_span("retrieve");
    assert!(root.get("parentSpanId").is_none(), "{root:#?}");
    for child in [&generate, &retrieve] {
        assert_eq!(child["traceId"], root["traceId"]);
        assert_eq!(child["parentSpanId"], root["spanId"]);
        // Trace attributes are stamped on children written before the root.
        assert_eq!(attr(child, "user.id"), Some(json!("user-7")));
        assert_eq!(attr(child, "session.id"), Some(json!("session-1")));
    }
    assert_eq!(
        attr(&generate, "langfuse.observation.type"),
        Some(json!("generation"))
    );
    assert_eq!(
        attr(&retrieve, "langfuse.observation.type"),
        Some(json!("retriever"))
    );
    assert_eq!(attr(&root, "langfuse.trace.name"), Some(json!("pipeline")));
    // One root: the trace event was merged into the root observation's span.
    let roots: Vec<_> = recorder
        .v4_spans()
        .into_iter()
        .filter(|s| s.get("parentSpanId").is_none())
        .map(|s| s["spanId"].clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(roots.len(), 1, "{:#?}", recorder.v4_spans());
}

#[tokio::test]
async fn errors_set_level_error_and_status_message() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    assert!(failing(true).await.is_err());
    assert_eq!(failing(false).await.unwrap(), 1);
    assert!(question_mark("nope").await.is_err());
    assert_eq!(question_mark("41").await.unwrap(), 42);
    finish(batcher).await;

    let observations = recorder.v3_observations();
    let failed: Vec<_> = observations
        .iter()
        .filter(|o| o["name"] == "failing")
        .collect();
    let err = failed.iter().find(|o| o["level"] == "ERROR").unwrap();
    assert_eq!(err["statusMessage"], "it broke");
    // `err`: the error is the output.
    assert_eq!(err["output"], "it broke");
    let ok = failed.iter().find(|o| o["level"] != "ERROR").unwrap();
    assert_eq!(ok["output"], 1);

    let parsed: Vec<_> = observations
        .iter()
        .filter(|o| o["name"] == "question_mark")
        .collect();
    let err = parsed.iter().find(|o| o["level"] == "ERROR").unwrap();
    assert_eq!(err["statusMessage"], "invalid digit found in string");
    // Without `err`, the output of an `Err` is omitted.
    assert!(err.get("output").is_none_or(Value::is_null));
    let ok = parsed.iter().find(|o| o["level"] != "ERROR").unwrap();
    assert_eq!(ok["output"], 42);
}

#[tokio::test]
async fn skip_and_capture_options() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    assert_eq!(skipping("s3cret", 4, Opaque(9), Hidden), "s3cret4Opaque(9)");
    assert_eq!(no_input(5), 5);
    let service = Service {
        prefix: "p-".into(),
    };
    assert_eq!(service.call(3).await, "p-3");
    assert_eq!(service.borrowed(), "p-");
    finish(batcher).await;

    let skipped = recorder.v3_observation("skipping");
    // `secret` skipped, `opaque` captured with Debug, `hidden` omitted.
    assert_eq!(
        skipped["input"],
        json!({"visible": 4, "opaque": "Opaque(9)"})
    );
    assert!(skipped.get("output").is_none_or(Value::is_null));
    let no_input = recorder.v3_observation("no_input");
    assert!(no_input.get("input").is_none_or(Value::is_null));
    assert_eq!(no_input["output"], 5);
    assert_eq!(no_input["level"], "DEBUG");
    let call = recorder.v3_observation("service-call");
    assert_eq!(call["input"], json!({"value": 3}));
    assert_eq!(call["output"], "p-3");
    assert_eq!(recorder.v3_observation("borrowed")["output"], "p-");
}

#[tokio::test]
async fn panics_and_cancellation_still_end_observations() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    assert!(std::panic::catch_unwind(panics).is_err());
    let task = tokio::spawn(async { panics() }.in_current_observation());
    assert!(task.await.unwrap_err().is_panic());
    assert!(tokio::time::timeout(Duration::from_millis(10), slow())
        .await
        .is_err());
    finish(batcher).await;

    let observations = recorder.v3_observations();
    let panicked: Vec<_> = observations
        .iter()
        .filter(|o| o["name"] == "panics")
        .collect();
    assert_eq!(panicked.len(), 2);
    for observation in panicked {
        assert_eq!(observation["level"], "ERROR");
        assert_eq!(observation["statusMessage"], "panicked");
        assert!(observation["endTime"].is_string());
    }
    let cancelled = recorder.v3_observation("slow");
    assert_eq!(cancelled["level"], "WARNING");
    assert_eq!(cancelled["statusMessage"], "cancelled");
}

#[tokio::test]
async fn spawned_tasks_nest_with_in_current_observation() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    assert_eq!(spawns_child().await, 7);
    // Without the extension a spawned task starts its own trace.
    let detached = tokio::spawn(child_task()).await.unwrap();
    assert_eq!(detached, 7);
    finish(batcher).await;

    let parent = recorder.v3_observation("spawns_child");
    let children: Vec<_> = recorder
        .v3_observations()
        .into_iter()
        .filter(|o| o["name"] == "child_task")
        .collect();
    assert_eq!(children.len(), 2);
    assert!(children
        .iter()
        .any(|c| c["parentObservationId"] == parent["id"] && c["traceId"] == parent["traceId"]));
    assert!(children
        .iter()
        .any(|c| c.get("parentObservationId").is_none_or(Value::is_null)
            && c["traceId"] != parent["traceId"]));
}

#[tokio::test]
async fn streams_and_iterators_record_items_until_they_end() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    let items: Vec<_> = streaming(4).await.collect().await;
    assert_eq!(items.len(), 4);
    assert_eq!(counting(3).collect::<Vec<_>>(), vec![0, 1, 2]);
    // Dropped before the end: cancelled.
    let mut partial = counting(10);
    assert_eq!(partial.next(), Some(0));
    drop(partial);
    finish(batcher).await;

    let stream = recorder.v3_observation("streaming");
    assert_eq!(stream["kind"], "generation-create");
    assert_eq!(stream["output"], json!([0, 1, 3]));
    assert_eq!(stream["level"], "ERROR");
    assert_eq!(stream["statusMessage"], "bad item");
    assert!(stream["completionStartTime"].is_string());
    let counts: Vec<_> = recorder
        .v3_observations()
        .into_iter()
        .filter(|o| o["name"] == "counting")
        .collect();
    assert!(counts.iter().any(|c| c["output"] == json!([0, 1, 2])));
    assert!(counts
        .iter()
        .any(|c| c["level"] == "WARNING" && c["output"] == json!([0])));
}

#[tokio::test]
async fn sampling_and_no_sink_record_nothing() {
    let _guard = GLOBAL_SINK.lock().await;
    let (recorder, batcher) = setup(ApiVersion::V3).await;

    observe::set_sample_rate(0.0);
    pipeline("sampled out".into()).await.unwrap();
    observe::set_sample_rate(1.0);
    finish(batcher).await;
    assert!(
        recorder.v3_events().is_empty(),
        "{:#?}",
        recorder.v3_events()
    );

    // No sink at all: the functions run, nothing is current.
    assert!(!observe::is_installed());
    assert_eq!(tokenize("a b"), 2);
    #[observe]
    fn reads_current() -> bool {
        observe::current().is_some()
    }
    assert!(!reads_current());
}

#[tokio::test]
async fn manual_observations_and_masking() {
    let _guard = GLOBAL_SINK.lock().await;
    let recorder = Recorder::start().await;
    let client = langfuse_ergonomic::ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(recorder.server.url())
        .api_version(ApiVersion::V3)
        .mask(|value| match value {
            Value::String(s) if s.contains("secret") => json!("[redacted]"),
            other => other,
        })
        .build()
        .unwrap();
    let batcher = recorder.batcher(client).await;

    let root = observe::Observation::builder("manual")
        .as_type(ObservationType::Agent)
        .batcher(batcher.clone())
        .input("my secret")
        .start();
    assert!(root.is_recording());
    let child = root.child("step", ObservationType::Tool);
    child.end_with_output("done");
    {
        let _scope = root.clone().enter_scope();
        #[observe]
        fn inside() -> u8 {
            1
        }
        inside();
    }
    batcher.flush().await.unwrap();

    let manual = recorder.v3_observation("manual");
    assert_eq!(manual["type"], "AGENT");
    assert_eq!(manual["input"], "[redacted]");
    let step = recorder.v3_observation("step");
    assert_eq!(step["type"], "TOOL");
    assert_eq!(step["parentObservationId"], manual["id"]);
    assert_eq!(
        recorder.v3_observation("inside")["parentObservationId"],
        manual["id"]
    );
}

#[test]
fn sync_observations_work_without_a_runtime() {
    // A batcher sink with try_add needs no runtime in the observed code itself.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let _guard = runtime.block_on(GLOBAL_SINK.lock());
    let (recorder, batcher) = runtime.block_on(setup(ApiVersion::V3));
    std::thread::spawn(|| {
        assert_eq!(tokenize("one two three"), 3);
    })
    .join()
    .unwrap();
    runtime.block_on(finish(batcher));
    assert_eq!(recorder.v3_observation("tokenize")["output"], 3);
}
