//! Mock tests for offline development and testing without API credentials

use langfuse_ergonomic::LangfuseClient;
use mockito::Server;
use serde_json::json;

/// Helper to create a mock client pointing to a mockito server
fn create_mock_client(mock_server: &Server) -> LangfuseClient {
    LangfuseClient::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url(mock_server.url())
        .build()
}

#[tokio::test]
async fn test_trace_creation_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .trace()
        .name("mock-trace".to_string())
        .input(json!({"test": "data"}))
        .output(json!({"result": "success"}))
        .user_id("test-user".to_string())
        .session_id("test-session".to_string())
        .tags(vec!["test".to_string(), "mock".to_string()])
        .call()
        .await;

    mock.assert_async().await;
    assert!(
        result.is_ok(),
        "Trace creation should succeed with mock server"
    );
}

#[tokio::test]
async fn test_trace_creation_auth_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Unauthorized"}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .trace()
        .name("mock-trace-fail".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_err(), "Trace creation should fail with 401");

    if let Err(error) = result {
        // Check that we get a meaningful error
        assert!(error.to_string().contains("401") || error.to_string().contains("Unauthorized"));
    }
}

#[tokio::test]
async fn test_rate_limiting_handling() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("retry-after", "5")
        .with_body(r#"{"error": "Rate limit exceeded"}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .trace()
        .name("rate-limited-trace".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_err(), "Should fail with rate limit error");

    if let Err(error) = result {
        assert!(error.to_string().contains("429") || error.to_string().contains("rate"));
    }
}

#[tokio::test]
async fn test_span_creation_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .span()
        .trace_id("mock-trace-id".to_string())
        .name("mock-span".to_string())
        .input(json!({"operation": "test"}))
        .output(json!({"result": "completed"}))
        .level("INFO".to_string())
        .status_message("Operation completed successfully".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "Span creation should succeed");
}

#[tokio::test]
async fn test_generation_creation_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .generation()
        .trace_id("mock-trace-id".to_string())
        .name("mock-generation".to_string())
        .model("gpt-4".to_string())
        .input(json!({"prompt": "What is 2+2?"}))
        .output(json!({"completion": "2+2 equals 4"}))
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "Generation creation should succeed");
}

#[tokio::test]
async fn test_event_creation_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .event()
        .trace_id("mock-trace-id".to_string())
        .name("mock-event".to_string())
        .input(json!({"event_type": "user_action"}))
        .level("WARNING".to_string())
        .status_message("User performed unexpected action".to_string())
        .metadata(json!({"action_id": 12345}))
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "Event creation should succeed");
}

#[tokio::test]
async fn test_score_creation_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    // Test numeric score
    let result = client
        .score()
        .trace_id("mock-trace-id".to_string())
        .name("accuracy".to_string())
        .value(0.95)
        .comment("High accuracy achieved".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "Numeric score creation should succeed");
}

#[tokio::test]
async fn test_categorical_score_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .score()
        .trace_id("mock-trace-id".to_string())
        .name("sentiment".to_string())
        .string_value("positive".to_string())
        .comment("Positive sentiment detected".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok(), "Categorical score creation should succeed");
}

#[tokio::test]
async fn test_server_error_handling() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(500)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error": "Internal server error"}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .trace()
        .name("server-error-trace".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_err(), "Should fail with server error");
}

#[tokio::test]
async fn test_network_error_handling() {
    // Create client with invalid URL to simulate network error
    let client = LangfuseClient::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url("http://invalid-url-that-does-not-exist.local:12345".to_string())
        .build();

    let result = client
        .trace()
        .name("network-error-trace".to_string())
        .call()
        .await;

    assert!(result.is_err(), "Should fail with network error");

    if let Err(error) = result {
        // Should be a network-related error
        let error_string = error.to_string().to_lowercase();
        assert!(
            error_string.contains("network")
                || error_string.contains("connection")
                || error_string.contains("dns")
                || error_string.contains("resolve")
                || error_string.contains("timeout")
                || error_string.contains("error sending request"),
            "Error should be network-related: {}",
            error
        );
    }
}

#[tokio::test]
async fn test_client_builder_validation() {
    // Test that builder works with different configurations
    let client = LangfuseClient::builder()
        .public_key("pk-lf-test-key")
        .secret_key("sk-lf-test-secret")
        .base_url("https://custom-langfuse.example.com".to_string())
        .timeout(std::time::Duration::from_secs(30))
        .connect_timeout(std::time::Duration::from_secs(5))
        .user_agent("custom-agent/1.0".to_string())
        .build();

    // Just test that the client was created successfully
    // We verify creation by checking that we can call methods on it
    assert!(client
        .trace()
        .name("test".to_string())
        .call()
        .await
        .is_err()); // Should fail due to fake URL, but client works
}

#[test]
fn test_client_from_env_missing_vars() {
    // Temporarily clear environment variables
    std::env::remove_var("LANGFUSE_PUBLIC_KEY");
    std::env::remove_var("LANGFUSE_SECRET_KEY");
    std::env::remove_var("LANGFUSE_BASE_URL");

    let result = LangfuseClient::from_env();
    assert!(
        result.is_err(),
        "Should fail when environment variables are missing"
    );

    if let Err(error) = result {
        let error_string = error.to_string().to_lowercase();
        assert!(
            error_string.contains("public_key") || error_string.contains("environment"),
            "Error should mention missing configuration: {}",
            error
        );
    }
}

/// Test helper functions for mock testing
pub mod test_helpers {
    use super::*;

    /// Create a mock server with common successful responses
    pub async fn create_mock_server_with_success_responses() -> mockito::ServerGuard {
        let mut server = Server::new_async().await;

        // Default success response for ingestion
        server
            .mock("POST", "/api/public/ingestion")
            .with_status(207)
            .with_header("content-type", "application/json")
            .with_body(r#"{"successes": [], "errors": []}"#)
            .create_async()
            .await;

        server
    }

    /// Create a mock server that simulates various error conditions
    pub async fn create_mock_server_with_errors() -> mockito::ServerGuard {
        let mut server = Server::new_async().await;

        // Simulate different error responses
        server
            .mock("POST", "/api/public/ingestion")
            .with_status(401)
            .with_header("content-type", "application/json")
            .with_body(r#"{"error": "Invalid API key"}"#)
            .create_async()
            .await;

        server
    }
}
