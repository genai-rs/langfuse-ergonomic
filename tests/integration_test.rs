//! Integration tests for langfuse-ergonomic

use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

/// Helper to create a client from environment variables
fn create_test_client() -> LangfuseClient {
    LangfuseClient::from_env().expect("Failed to create client from environment")
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_trace_creation() {
    let client = create_test_client();

    let result = client
        .trace()
        .name("integration-test-trace")
        .input(json!({"test": "input"}))
        .output(json!({"test": "output"}))
        .tags(["test", "integration"])
        .user_id("test-user")
        .session_id("test-session")
        .metadata(json!({"test_key": "test_value"}))
        .send()
        .await;

    assert!(result.is_ok(), "Failed to create trace: {:?}", result.err());
    let trace = result.unwrap();
    assert!(!trace.id.is_empty(), "Trace ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_span_creation() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("test-trace-for-span")
        .send()
        .await
        .expect("Failed to create trace");

    // Then create a span
    let result = client
        .span(trace.id.clone())
        .name("test-span")
        .input(json!({"span": "input"}))
        .output(json!({"span": "output"}))
        .level("INFO")
        .status_message("Test span created")
        .metadata(json!({"span_key": "span_value"}))
        .send()
        .await;

    assert!(result.is_ok(), "Failed to create span: {:?}", result.err());
    let span_id = result.unwrap();
    assert!(!span_id.is_empty(), "Span ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_generation_creation() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("test-trace-for-generation")
        .send()
        .await
        .expect("Failed to create trace");

    // Then create a generation
    let result = client
        .generation(trace.id.clone())
        .name("test-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Hello, world!"}))
        .output(json!({"completion": "Hi there!"}))
        .tokens(10, 5)
        .metadata(json!({"model_version": "1.0"}))
        .send()
        .await;

    assert!(
        result.is_ok(),
        "Failed to create generation: {:?}",
        result.err()
    );
    let generation_id = result.unwrap();
    assert!(
        !generation_id.is_empty(),
        "Generation ID should not be empty"
    );
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_event_creation() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("test-trace-for-event")
        .send()
        .await
        .expect("Failed to create trace");

    // Then create an event
    let result = client
        .event(trace.id.clone())
        .name("test-event")
        .input(json!({"event": "data"}))
        .level("WARNING")
        .status_message("Test event occurred")
        .metadata(json!({"event_type": "test"}))
        .send()
        .await;

    assert!(result.is_ok(), "Failed to create event: {:?}", result.err());
    let event_id = result.unwrap();
    assert!(!event_id.is_empty(), "Event ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_numeric_score_creation() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("test-trace-for-score")
        .send()
        .await
        .expect("Failed to create trace");

    // Then create a numeric score
    let result = client
        .score(trace.id.clone(), "accuracy")
        .value(0.95)
        .comment("High accuracy score")
        .metadata(json!({"threshold": 0.9}))
        .send()
        .await;

    assert!(
        result.is_ok(),
        "Failed to create numeric score: {:?}",
        result.err()
    );
    let score_id = result.unwrap();
    assert!(!score_id.is_empty(), "Score ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_categorical_score_creation() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("test-trace-for-categorical-score")
        .send()
        .await
        .expect("Failed to create trace");

    // Then create a categorical score
    let result = client
        .score(trace.id.clone(), "sentiment")
        .string_value("positive")
        .comment("User sentiment analysis")
        .send()
        .await;

    assert!(
        result.is_ok(),
        "Failed to create categorical score: {:?}",
        result.err()
    );
    let score_id = result.unwrap();
    assert!(!score_id.is_empty(), "Score ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_nested_observations() {
    let client = create_test_client();

    // Create a trace
    let trace = client
        .trace()
        .name("test-trace-nested")
        .send()
        .await
        .expect("Failed to create trace");

    // Create a parent span
    let parent_span = client
        .span(trace.id.clone())
        .name("parent-span")
        .send()
        .await
        .expect("Failed to create parent span");

    // Create a child span
    let child_span = client
        .span(trace.id.clone())
        .parent_observation_id(&parent_span)
        .name("child-span")
        .send()
        .await
        .expect("Failed to create child span");

    assert!(!child_span.is_empty(), "Child span ID should not be empty");

    // Create a generation under the child span
    let generation = client
        .generation(trace.id.clone())
        .parent_observation_id(&child_span)
        .name("nested-generation")
        .model("gpt-3.5-turbo")
        .send()
        .await
        .expect("Failed to create nested generation");

    assert!(!generation.is_empty(), "Generation ID should not be empty");
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_score_helpers() {
    let client = create_test_client();

    // Create a trace
    let trace = client
        .trace()
        .name("test-trace-score-helpers")
        .send()
        .await
        .expect("Failed to create trace");

    // Test binary score
    let binary_score = client
        .binary_score(trace.id.clone(), "success", true)
        .send()
        .await
        .expect("Failed to create binary score");
    assert!(!binary_score.is_empty());

    // Test rating score
    let rating_score = client
        .rating_score(trace.id.clone(), "quality", 4, 5)
        .send()
        .await
        .expect("Failed to create rating score");
    assert!(!rating_score.is_empty());

    // Test categorical score
    let categorical_score = client
        .categorical_score(trace.id.clone(), "category", "excellent")
        .send()
        .await
        .expect("Failed to create categorical score");
    assert!(!categorical_score.is_empty());
}
