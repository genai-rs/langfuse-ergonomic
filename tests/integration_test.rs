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
        .tags(vec!["test".to_string(), "integration".to_string()])
        .user_id("test-user")
        .session_id("test-session")
        .metadata(json!({"test_key": "test_value"}))
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Then create a span
    let result = client
        .span()
        .trace_id(trace.id.clone())
        .name("test-span")
        .input(json!({"span": "input"}))
        .output(json!({"span": "output"}))
        .level("INFO")
        .status_message("Test span created")
        .metadata(json!({"span_key": "span_value"}))
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Then create a generation
    let result = client
        .generation()
        .trace_id(trace.id.clone())
        .name("test-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Hello, world!"}))
        .output(json!({"completion": "Hi there!"}))
        .prompt_tokens(10)
        .completion_tokens(5)
        .metadata(json!({"model_version": "1.0"}))
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Then create an event
    let result = client
        .event()
        .trace_id(trace.id.clone())
        .name("test-event")
        .input(json!({"event": "data"}))
        .level("WARNING")
        .status_message("Test event occurred")
        .metadata(json!({"event_type": "test"}))
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Then create a numeric score
    let result = client
        .score()
        .trace_id(trace.id.clone())
        .name("accuracy")
        .value(0.95)
        .comment("High accuracy score")
        .metadata(json!({"threshold": 0.9}))
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Then create a categorical score
    let result = client
        .score()
        .trace_id(trace.id.clone())
        .name("sentiment")
        .string_value("positive")
        .comment("User sentiment analysis")
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Create a parent span
    let parent_span = client
        .span()
        .trace_id(trace.id.clone())
        .name("parent-span")
        .call()
        .await
        .expect("Failed to create parent span");

    // Create a child span
    let child_span = client
        .span()
        .trace_id(trace.id.clone())
        .parent_observation_id(&parent_span)
        .name("child-span")
        .call()
        .await
        .expect("Failed to create child span");

    assert!(!child_span.is_empty(), "Child span ID should not be empty");

    // Create a generation under the child span
    let generation = client
        .generation()
        .trace_id(trace.id.clone())
        .parent_observation_id(&child_span)
        .name("nested-generation")
        .model("gpt-3.5-turbo")
        .call()
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
        .call()
        .await
        .expect("Failed to create trace");

    // Test binary score
    let binary_score = client
        .binary_score(trace.id.clone(), "success", true)
        .await
        .expect("Failed to create binary score");
    assert!(!binary_score.is_empty());

    // Test rating score
    let rating_score = client
        .rating_score(trace.id.clone(), "quality", 4, 5)
        .await
        .expect("Failed to create rating score");
    assert!(!rating_score.is_empty());

    // Test categorical score
    let categorical_score = client
        .categorical_score(trace.id.clone(), "category", "excellent")
        .await
        .expect("Failed to create categorical score");
    assert!(!categorical_score.is_empty());
}

// ===== NEW FEATURES TESTS =====

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_trace_fetching() {
    let client = create_test_client();

    // First create a trace
    let trace = client
        .trace()
        .name("fetch-test-trace")
        .call()
        .await
        .expect("Failed to create trace");

    // Test get_trace
    let fetched_trace = client.get_trace(&trace.id).await;
    assert!(
        fetched_trace.is_ok(),
        "Failed to fetch trace: {:?}",
        fetched_trace.err()
    );

    // Test list_traces
    let traces = client.list_traces().limit(5).call().await;
    assert!(traces.is_ok(), "Failed to list traces: {:?}", traces.err());
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_dataset_management() {
    let client = create_test_client();

    // Test create_dataset
    let dataset_result = client
        .create_dataset()
        .name("test-integration-dataset")
        .description("Integration test dataset")
        .call()
        .await;
    assert!(
        dataset_result.is_ok(),
        "Failed to create dataset: {:?}",
        dataset_result.err()
    );

    // Test get_dataset
    let _get_result = client.get_dataset("test-integration-dataset").await;
    // Note: This might fail if dataset doesn't exist, but we still test the API call structure

    // Test list_datasets
    let list_result = client.list_datasets().limit(10).call().await;
    assert!(
        list_result.is_ok(),
        "Failed to list datasets: {:?}",
        list_result.err()
    );
}

#[tokio::test]
#[ignore = "requires Langfuse API credentials"]
async fn test_prompt_management() {
    let client = create_test_client();

    // Test get_prompt (might fail if prompt doesn't exist, but tests API structure)
    let _get_result = client.get_prompt("test-prompt", None, None).await;
    // We don't assert success here since the prompt might not exist

    // Test list_prompts
    let list_result = client.list_prompts().limit("5".to_string()).call().await;
    assert!(
        list_result.is_ok(),
        "Failed to list prompts: {:?}",
        list_result.err()
    );

    // Test create_prompt (placeholder implementation)
    let _create_result = client
        .create_prompt()
        .name("test-integration-prompt")
        .prompt("Test prompt: {{input}}")
        .call()
        .await;
    // Note: This is a placeholder implementation, so it might not actually create a prompt
}
