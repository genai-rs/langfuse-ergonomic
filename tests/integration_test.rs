//! Live integration tests against a real Langfuse server.
//!
//! Ignored by default. `scripts/live-tests.sh` starts a self-hosted Langfuse and runs them for
//! both API versions; against another server set `LANGFUSE_PUBLIC_KEY`,
//! `LANGFUSE_SECRET_KEY`, `LANGFUSE_BASE_URL` and `LANGFUSE_API_VERSION` (`v3` or `v4`) and
//! run `cargo test --test integration_test -- --ignored`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The v3 read methods are deprecated; these tests call them in V3 mode and check that they
// fail cleanly in V4 mode.
#![allow(deprecated)]

mod common;

use common::{eventually, live_client, wait_for_observation, wait_for_score, wait_for_trace};
use langfuse_ergonomic::{ApiVersion, Error, LangfuseClient};
use serde_json::json;

fn unique(prefix: &str) -> String {
    format!("{prefix}-{}", uuid::Uuid::new_v4())
}

async fn new_trace(client: &LangfuseClient, name: &str) -> String {
    client
        .trace()
        .name(unique(name))
        .call()
        .await
        .expect("Failed to create trace")
        .id
}

/// Read the score back and check its trace, name and value.
async fn assert_score(
    client: &LangfuseClient,
    score_id: &str,
    trace_id: &str,
    name: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    assert!(!score_id.is_empty(), "Score ID should not be empty");
    let score = wait_for_score(client, score_id).await;
    assert_eq!(score["id"], score_id, "{score}");
    assert_eq!(score["traceId"], trace_id, "{score}");
    assert_eq!(score["name"], name, "{score}");
    match value.as_f64() {
        // Compare numbers as f64: Langfuse returns 1.0 as `1`.
        Some(expected) => assert_eq!(score["value"].as_f64(), Some(expected), "{score}"),
        None => assert_eq!(score["stringValue"], value, "{score}"),
    }
    score
}

fn expect_type(
    expected: &'static str,
) -> impl Fn(&common::StoredObservation) -> Result<(), String> {
    move |o| {
        if o.r#type == expected {
            Ok(())
        } else {
            Err(format!("type is {}, expected {expected}", o.r#type))
        }
    }
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_trace_creation() {
    let client = live_client();
    let name = unique("integration-test-trace");

    let trace = client
        .trace()
        .name(name.clone())
        .input(json!({"test": "input"}))
        .output(json!({"test": "output"}))
        .tags(vec!["test".to_string(), "integration".to_string()])
        .user_id("test-user")
        .session_id("test-session")
        .metadata(json!({"test_key": "test_value"}))
        .call()
        .await
        .expect("Failed to create trace");

    assert!(!trace.id.is_empty(), "Trace ID should not be empty");
    if client.api_version() == ApiVersion::V4 {
        assert_eq!(trace.id.len(), 32, "V4 trace ids are OTel trace ids");
    }
    let stored = wait_for_trace(&client, &trace.id).await;
    assert_eq!(stored.name, Some(name));
    assert_eq!(stored.user_id.as_deref(), Some("test-user"));
    assert_eq!(stored.session_id.as_deref(), Some("test-session"));
    let mut tags = stored.tags;
    tags.sort();
    assert_eq!(tags, ["integration", "test"]);
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_span_creation() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-for-span").await;

    let span_id = client
        .span()
        .trace_id(trace_id.clone())
        .name("test-span")
        .input(json!({"span": "input"}))
        .output(json!({"span": "output"}))
        .level("INFO")
        .status_message("Test span created")
        .metadata(json!({"span_key": "span_value"}))
        .call()
        .await
        .expect("Failed to create span");

    assert!(!span_id.is_empty(), "Span ID should not be empty");
    let stored = wait_for_observation(&client, &trace_id, &span_id, expect_type("SPAN")).await;
    assert_eq!(stored.trace_id.as_deref(), Some(trace_id.as_str()));
    assert_eq!(stored.name.as_deref(), Some("test-span"));
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_generation_creation() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-for-generation").await;

    let generation_id = client
        .generation()
        .trace_id(trace_id.clone())
        .name("test-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Hello, world!"}))
        .output(json!({"completion": "Hi there!"}))
        .prompt_tokens(10)
        .completion_tokens(5)
        .metadata(json!({"model_version": "1.0"}))
        .call()
        .await
        .expect("Failed to create generation");

    assert!(
        !generation_id.is_empty(),
        "Generation ID should not be empty"
    );
    let stored = wait_for_observation(
        &client,
        &trace_id,
        &generation_id,
        expect_type("GENERATION"),
    )
    .await;
    assert_eq!(stored.name.as_deref(), Some("test-generation"));
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_event_creation() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-for-event").await;

    let event_id = client
        .event()
        .trace_id(trace_id.clone())
        .name("test-event")
        .input(json!({"event": "data"}))
        .level("WARNING")
        .status_message("Test event occurred")
        .metadata(json!({"event_type": "test"}))
        .call()
        .await
        .expect("Failed to create event");

    assert!(!event_id.is_empty(), "Event ID should not be empty");
    let stored = wait_for_observation(&client, &trace_id, &event_id, expect_type("EVENT")).await;
    assert_eq!(stored.name.as_deref(), Some("test-event"));
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_numeric_score_creation() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-for-score").await;

    let score_id = client
        .score()
        .trace_id(trace_id.clone())
        .name("accuracy")
        .value(0.95)
        .comment("High accuracy score")
        .metadata(json!({"threshold": 0.9}))
        .call()
        .await
        .expect("Failed to create numeric score");
    let score = assert_score(&client, &score_id, &trace_id, "accuracy", json!(0.95)).await;
    assert_eq!(score["comment"], "High accuracy score");
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_categorical_score_creation() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-for-categorical-score").await;

    let score_id = client
        .score()
        .trace_id(trace_id.clone())
        .name("sentiment")
        .string_value("positive")
        .comment("User sentiment analysis")
        .call()
        .await
        .expect("Failed to create categorical score");
    assert_score(
        &client,
        &score_id,
        &trace_id,
        "sentiment",
        json!("positive"),
    )
    .await;
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_nested_observations() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-nested").await;

    let parent_span = client
        .span()
        .trace_id(trace_id.clone())
        .name("parent-span")
        .call()
        .await
        .expect("Failed to create parent span");

    let child_span = client
        .span()
        .trace_id(trace_id.clone())
        .parent_observation_id(&parent_span)
        .name("child-span")
        .call()
        .await
        .expect("Failed to create child span");

    let generation = client
        .generation()
        .trace_id(trace_id.clone())
        .parent_observation_id(&child_span)
        .name("nested-generation")
        .model("gpt-3.5-turbo")
        .call()
        .await
        .expect("Failed to create nested generation");

    let parent_is = |expected: String| {
        move |o: &common::StoredObservation| {
            if o.parent_observation_id.as_deref() == Some(expected.as_str()) {
                Ok(())
            } else {
                Err(format!("parent is not {expected}"))
            }
        }
    };
    wait_for_observation(&client, &trace_id, &child_span, parent_is(parent_span)).await;
    wait_for_observation(&client, &trace_id, &generation, parent_is(child_span)).await;
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_score_helpers() {
    let client = live_client();
    let trace_id = new_trace(&client, "test-trace-score-helpers").await;

    let binary_score = client
        .binary_score(trace_id.clone(), "success", true)
        .await
        .expect("Failed to create binary score");
    assert_score(&client, &binary_score, &trace_id, "success", json!(1.0)).await;

    let rating_score = client
        .rating_score(trace_id.clone(), "quality", 4, 5)
        .await
        .expect("Failed to create rating score");
    assert_score(&client, &rating_score, &trace_id, "quality", json!(0.8)).await;

    let categorical_score = client
        .categorical_score(trace_id.clone(), "category", "excellent")
        .await
        .expect("Failed to create categorical score");
    assert_score(
        &client,
        &categorical_score,
        &trace_id,
        "category",
        json!("excellent"),
    )
    .await;
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_trace_fetching() {
    let client = live_client();
    let name = unique("fetch-test-trace");
    let trace_id = client
        .trace()
        .name(name.clone())
        .call()
        .await
        .expect("Failed to create trace")
        .id;
    wait_for_trace(&client, &trace_id).await;

    match client.api_version() {
        ApiVersion::V3 => {
            let trace = client
                .get_trace(&trace_id)
                .await
                .expect("Failed to fetch trace");
            assert_eq!(trace.id, trace_id);
            let traces = eventually("trace in list_traces", || async {
                let traces = client
                    .list_traces()
                    .name(name.as_str())
                    .limit(5)
                    .call()
                    .await
                    .map_err(|e| format!("list error: {e}"))?;
                if traces.data.iter().any(|t| t.id == trace_id) {
                    Ok(traces)
                } else {
                    Err(format!("{} traces, not {trace_id}", traces.data.len()))
                }
            })
            .await;
            assert_eq!(traces.data.len(), 1);
        }
        ApiVersion::V4 => {
            // The v3 reads fail before sending a request.
            assert!(matches!(
                client.get_trace(&trace_id).await,
                Err(Error::Configuration(_))
            ));
            assert!(matches!(
                client.list_traces().limit(5).call().await,
                Err(Error::Configuration(_))
            ));
            // A trace is its root observation; list them by trace name.
            let roots = eventually("trace in root observations", || async {
                let roots = client
                    .observations()
                    .is_root_observation(true)
                    .filter(
                        json!([{
                            "type": "string",
                            "column": "traceName",
                            "operator": "=",
                            "value": name,
                        }])
                        .to_string(),
                    )
                    .limit(5)
                    .call()
                    .await
                    .map_err(|e| format!("list error: {e}"))?;
                if roots
                    .data
                    .iter()
                    .any(|o| o.trace_id.as_deref() == Some(trace_id.as_str()))
                {
                    Ok(roots)
                } else {
                    Err(format!(
                        "{} root observations, not {trace_id}",
                        roots.data.len()
                    ))
                }
            })
            .await;
            assert_eq!(roots.data.len(), 1);
        }
    }
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_dataset_management() {
    let client = live_client();
    let name = unique("test-integration-dataset");

    client
        .create_dataset()
        .name(name.clone())
        .description("Integration test dataset")
        .call()
        .await
        .expect("Failed to create dataset");

    let dataset = client
        .get_dataset(name.clone())
        .await
        .expect("Failed to get dataset");
    assert_eq!(dataset.name, name);

    client
        .list_datasets()
        .limit(10)
        .call()
        .await
        .expect("Failed to list datasets");
}

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_prompt_management() {
    let client = live_client();
    let name = unique("test-integration-prompt");

    client
        .create_prompt()
        .name(name.clone())
        .prompt("Test prompt: {{input}}")
        .labels(vec!["production".to_string()])
        .call()
        .await
        .expect("Failed to create prompt");

    client
        .get_prompt(name.as_str(), None, None)
        .await
        .expect("Failed to get prompt");

    client
        .list_prompts()
        .limit("5".to_string())
        .call()
        .await
        .expect("Failed to list prompts");
}

/// `#[observe]` nests across `.await`: the child is stored with its parent and its type.
#[cfg(feature = "macros")]
#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_observe_nesting_live() {
    use langfuse_ergonomic::observe;
    use langfuse_ergonomic::observe::{Observation, ObservationType};

    #[observe(as_type = "tool", name = "live-observe-tool")]
    async fn tool(input: u32) -> u32 {
        tokio::task::yield_now().await;
        input * 2
    }

    let client = live_client();
    let batcher = client.batcher().build().await;
    let root = Observation::builder(unique("live-observe-root"))
        .as_type(ObservationType::Agent)
        .batcher(batcher.clone())
        .start();
    let trace_id = root.trace_id().unwrap().to_owned();
    let root_id = root.id().unwrap().to_owned();
    let doubled = root
        .clone()
        .instrument(async {
            assert_eq!(observe::current().unwrap().id(), Some(root_id.as_str()));
            tool(21).await
        })
        .await;
    assert_eq!(doubled, 42);
    batcher.shutdown().await.expect("shutdown");

    let observations = eventually("observe nesting", || async {
        let observations = match client.api_version() {
            ApiVersion::V4 => client
                .trace_observations(&trace_id)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|o| (o.name.flatten(), o.parent_observation_id, o.r#type))
                .collect::<Vec<_>>(),
            ApiVersion::V3 => client
                .get_trace(&trace_id)
                .await
                .map_err(|e| e.to_string())?
                .observations
                .into_iter()
                .map(|o| (o.name, o.parent_observation_id, o.r#type))
                .collect(),
        };
        if observations.len() < 2 {
            return Err(format!("{} observations so far", observations.len()));
        }
        Ok(observations)
    })
    .await;
    let tool = observations
        .iter()
        .find(|(name, _, _)| name.as_deref() == Some("live-observe-tool"))
        .expect("the tool observation");
    assert_eq!(tool.1.as_deref(), Some(root_id.as_str()));
    assert_eq!(tool.2, "TOOL");
}
