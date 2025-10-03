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
        .expect("mock credentials should be valid")
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
        .name("test-trace")
        .user_id("user-123")
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert!(!result.unwrap().id.is_empty());
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

    let result = client.trace().name("test-trace").call().await;

    mock.assert_async().await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("401") || err.to_string().contains("Unauthorized"));
    }
}

#[tokio::test]
async fn test_validate_success() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/health")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status": "ok"}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client.validate().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn test_validate_auth_error() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/health")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_header("x-request-id", "req-12345")
        .with_body(r#"{"error": "Invalid credentials"}"#)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client.validate().await;

    mock.assert_async().await;
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("Invalid credentials"));
    }
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

    let result = client.trace().name("test-trace").call().await;

    mock.assert_async().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_rate_limiting_handling() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(429)
        .with_header("content-type", "application/json")
        .with_header("Retry-After", "60")
        .with_body(r#"{"error": "Too many requests"}"#)
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
        // Check that it's a rate limit error
        assert!(
            error.is_retryable(),
            "Rate limit errors should be retryable"
        );
        let error_str = error.to_string();
        assert!(
            error_str.contains("Rate limit"),
            "Expected rate limit error, got: {}",
            error_str
        );
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
        .trace_id("trace-123")
        .name("test-span")
        .level("INFO")
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let span_id = result.unwrap();
    assert!(!span_id.is_empty());
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
        .trace_id("trace-123")
        .name("test-generation")
        .model("gpt-4")
        .input(json!({"prompt": "Hello"}))
        .output(json!({"response": "Hi"}))
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let gen_id = result.unwrap();
    assert!(!gen_id.is_empty());
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
        .trace_id("trace-123")
        .name("user-action")
        .level("INFO")
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let event_id = result.unwrap();
    assert!(!event_id.is_empty());
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

    let result = client
        .score()
        .trace_id("trace-123")
        .name("quality")
        .value(0.95)
        .comment("Excellent")
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let score_id = result.unwrap();
    assert!(!score_id.is_empty());
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
        .categorical_score("trace-123", "sentiment", "positive")
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_network_error_handling() {
    // Create a client with an invalid URL
    let client = LangfuseClient::builder()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url("http://localhost:19999".to_string()) // Non-existent port
        .build()
        .expect("mock builder should produce client");

    let result = client.trace().name("test-trace").call().await;

    assert!(result.is_err());
    if let Err(err) = result {
        // Should be a network/API error
        let error_str = err.to_string();
        assert!(
            error_str.contains("connect")
                || error_str.contains("Connection")
                || error_str.contains("Network")
                || error_str.contains("error sending request"),
            "Expected network error, got: {}",
            error_str
        );
    }
}

// Note: Builder validation is done at compile time via the bon crate
// These tests would fail to compile if required fields are missing

#[tokio::test]
async fn test_client_from_env_missing_vars() {
    // Clear the environment variables
    std::env::remove_var("LANGFUSE_PUBLIC_KEY");
    std::env::remove_var("LANGFUSE_SECRET_KEY");
    std::env::remove_var("LANGFUSE_BASE_URL");

    let result = LangfuseClient::from_env();
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.to_string().contains("LANGFUSE_PUBLIC_KEY"));
    }
}

// TODO: Enable when API endpoints are clarified
// #[tokio::test]
#[allow(dead_code)]
async fn test_list_traces_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/traces")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": [
                    {
                        "id": "trace-1",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "name": "test-trace-1"
                    },
                    {
                        "id": "trace-2",
                        "timestamp": "2024-01-01T00:01:00Z",
                        "name": "test-trace-2"
                    }
                ],
                "meta": {
                    "page": 1,
                    "limit": 10,
                    "totalItems": 2,
                    "totalPages": 1
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client.list_traces().page(1).limit(10).call().await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

// TODO: Enable when API endpoints are clarified
// #[tokio::test]
#[allow(dead_code)]
async fn test_dataset_create_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/v2/datasets")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "dataset-123",
                "name": "test-dataset",
                "description": "Test dataset",
                "createdAt": "2024-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .create_dataset()
        .name("test-dataset")
        .description("Test dataset")
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

// TODO: Enable when API endpoints are clarified
// #[tokio::test]
#[allow(dead_code)]
async fn test_dataset_get_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/v2/datasets/test-dataset")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "dataset-123",
                "name": "test-dataset",
                "description": "Test dataset",
                "createdAt": "2024-01-01T00:00:00Z",
                "metadata": {}
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client.get_dataset("test-dataset").await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

// TODO: Enable when API endpoints are clarified
// #[tokio::test]
#[allow(dead_code)]
async fn test_prompt_get_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/v2/prompts/test-prompt")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "prompt-123",
                "name": "test-prompt",
                "version": 1,
                "prompt": "You are a helpful assistant.",
                "config": {},
                "createdAt": "2024-01-01T00:00:00Z"
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client.get_prompt("test-prompt", None, None).await;

    mock.assert_async().await;
    assert!(result.is_ok());
}

// TODO: Enable when API endpoints are clarified
// #[tokio::test]
#[allow(dead_code)]
async fn test_prompt_list_mock() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("GET", "/api/public/v2/prompts")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": [
                    {
                        "id": "prompt-1",
                        "name": "greeting-prompt",
                        "version": 1,
                        "prompt": "Say hello"
                    },
                    {
                        "id": "prompt-2",
                        "name": "farewell-prompt",
                        "version": 1,
                        "prompt": "Say goodbye"
                    }
                ],
                "meta": {
                    "page": 1,
                    "limit": 20,
                    "totalItems": 2,
                    "totalPages": 1
                }
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);

    let result = client
        .list_prompts()
        .page(1)
        .limit("20".to_string())
        .call()
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}
