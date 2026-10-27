//! Mock tests for offline development and testing without API credentials

use langfuse_ergonomic::{ClientBuilder, Error, LangfuseClient, ObservationsViews};
use mockito::{Matcher, Server};
use serde_json::json;

/// Helper to create a mock client pointing to a mockito server
fn create_mock_client(mock_server: &Server) -> LangfuseClient {
    ClientBuilder::new()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url(mock_server.url())
        .build()
        .expect("mock credentials should be valid")
}

#[tokio::test]
async fn test_create_chat_prompt_rejects_invalid_messages_without_http() {
    let invalid_messages = [
        json!(null),
        json!("private-message-content"),
        json!({"role": "user"}),
        json!({"role": "user", "content": {"secret": "private-message-content"}}),
        json!({"name": 42, "type": "placeholder"}),
        json!({"role": "user", "content": "private-message-content", "type": "invalid"}),
    ];

    for invalid_message in invalid_messages {
        // Exercise the first, middle, and last positions in a mixed list.
        for index in 0..3 {
            let mut server = Server::new_async().await;
            let mock = server
                .mock("POST", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let client = create_mock_client(&server);
            let mut messages = vec![
                json!({"role": "system", "content": "private-system-content"}),
                json!({"name": "history", "type": "placeholder"}),
            ];
            messages.insert(index, invalid_message.clone());

            let error = client
                .create_chat_prompt()
                .name("private-prompt-name")
                .messages(messages)
                .call()
                .await
                .expect_err("invalid messages must fail before HTTP");

            mock.assert_async().await;
            let expected = format!(
                "Invalid chat prompt message at index {index}: expected a chat message or placeholder"
            );
            assert!(matches!(&error, Error::Validation(message) if message == &expected));
            assert!(!error.is_retryable());
            assert_eq!(error.to_string(), format!("Validation error: {expected}"));
            for sensitive in [
                "private-message-content",
                "private-system-content",
                "private-prompt-name",
                "pk-lf-test",
                "sk-lf-test",
            ] {
                assert!(!format!("{error:?}").contains(sensitive));
            }
        }
    }
}

#[tokio::test]
async fn test_create_chat_prompt_preserves_mixed_messages() {
    let mut server = Server::new_async().await;
    let messages = vec![
        json!({"role": "system", "content": "Keep {{topic}} unchanged.\n日本語"}),
        json!({"name": "history", "type": "placeholder"}),
        json!({"role": "user", "content": "  Hello {{name}}!  ", "type": "chatmessage"}),
        json!({"role": "assistant", "content": ""}),
    ];
    let config = json!({"temperature": 0.25});
    let mock = server
        .mock("POST", "/api/public/v2/prompts")
        .match_body(mockito::Matcher::Json(json!({
            "name": "mixed-chat",
            "type": "chat",
            "prompt": messages,
            "config": config,
            "labels": ["production"],
            "tags": ["test"]
        })))
        .expect(1)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "name": "mixed-chat",
                "version": 1,
                "type": "chat",
                "prompt": messages,
                "config": config,
                "labels": ["production"],
                "tags": ["test"]
            })
            .to_string(),
        )
        .create_async()
        .await;
    let client = create_mock_client(&server);

    let response = client
        .create_chat_prompt()
        .name("mixed-chat")
        .messages(messages.clone())
        .config(config)
        .labels(vec!["production".to_owned()])
        .tags(vec!["test".to_owned()])
        .call()
        .await
        .expect("create a valid mixed chat prompt");

    mock.assert_async().await;
    assert_eq!(
        serde_json::to_value(response).unwrap()["prompt"],
        json!(messages)
    );
}

#[tokio::test]
async fn dataset_schemas_preserve_objects_and_reject_non_objects() {
    let mut server = Server::new_async().await;
    let client = create_mock_client(&server);
    let input_schema = json!({"type": "object", "properties": {"query": {"type": "string"}}});
    let expected_output_schema =
        json!({"type": "object", "properties": {"answer": {"type": "integer"}}});
    let objects = server
        .mock("POST", "/api/public/v2/datasets")
        .match_body(Matcher::Json(json!({
            "name": "objects",
            "inputSchema": input_schema,
            "expectedOutputSchema": expected_output_schema
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"dataset-1","name":"test","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"project-1","createdAt":"2026-10-03T12:00:00Z","updatedAt":"2026-10-03T12:00:00Z"}"#)
        .create_async()
        .await;
    client
        .create_dataset()
        .name("objects")
        .input_schema(input_schema)
        .expected_output_schema(expected_output_schema)
        .call()
        .await
        .unwrap();
    objects.assert_async().await;

    let nulls = server
        .mock("POST", "/api/public/v2/datasets")
        .match_body(Matcher::Json(json!({
            "name": "nulls",
            "inputSchema": null,
            "expectedOutputSchema": null
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"dataset-1","name":"nulls","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"project-1","createdAt":"2026-10-03T12:00:00Z","updatedAt":"2026-10-03T12:00:00Z"}"#)
        .create_async()
        .await;
    client
        .create_dataset()
        .name("nulls")
        .input_schema(json!(null))
        .expected_output_schema(json!(null))
        .call()
        .await
        .unwrap();
    nulls.assert_async().await;

    let omitted = server
        .mock("POST", "/api/public/v2/datasets")
        .match_body(Matcher::Json(json!({"name": "omitted"})))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"dataset-1","name":"omitted","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"project-1","createdAt":"2026-10-03T12:00:00Z","updatedAt":"2026-10-03T12:00:00Z"}"#)
        .create_async()
        .await;
    client
        .create_dataset()
        .name("omitted")
        .call()
        .await
        .unwrap();
    omitted.assert_async().await;

    let invalid = server
        .mock("POST", "/api/public/v2/datasets")
        .expect(0)
        .create_async()
        .await;
    let errors = [
        client
            .create_dataset()
            .name("private-dataset")
            .input_schema(json!("private-input-schema"))
            .call()
            .await
            .unwrap_err(),
        client
            .create_dataset()
            .name("private-dataset")
            .input_schema(json!(true))
            .call()
            .await
            .unwrap_err(),
        client
            .create_dataset()
            .name("private-dataset")
            .expected_output_schema(json!(["private-output-schema"]))
            .call()
            .await
            .unwrap_err(),
        client
            .create_dataset()
            .name("private-dataset")
            .expected_output_schema(json!([]))
            .call()
            .await
            .unwrap_err(),
    ];
    for error in errors {
        assert!(matches!(
            &error,
            Error::Validation(message)
                if message == "Dataset schema must be a JSON object or null"
        ));
        let diagnostics = format!("{error:?}");
        assert!(!diagnostics.contains("private-input-schema"));
        assert!(!diagnostics.contains("private-output-schema"));
    }
    invalid.assert_async().await;
}

#[tokio::test]
async fn test_get_observations_preserves_public_response_type() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", "/api/public/observations")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": [],
                "meta": {"page": 1, "limit": 10, "totalItems": 0, "totalPages": 0}
            })
            .to_string(),
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);
    // The response name exported by 0.6.3 must still match this API's result.
    let response: ObservationsViews = client
        .get_observations()
        .page(1)
        .limit(10)
        .call()
        .await
        .expect("decode observations using the public response type");

    mock.assert_async().await;
    assert!(response.data.is_empty());
    assert_eq!(response.meta.page, 1);
    assert_eq!(response.meta.limit, 10);
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
            "Expected rate limit error, got: {error_str}"
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
async fn test_generation_token_usage_payload() {
    let cases = [
        (
            Some(1200),
            Some(340),
            None,
            Some(json!({"input": 1200, "output": 340})),
        ),
        (
            Some(1200),
            Some(340),
            Some(1600),
            Some(json!({"input": 1200, "output": 340, "total": 1600})),
        ),
        (Some(1200), None, None, Some(json!({"input": 1200}))),
        (None, Some(340), None, Some(json!({"output": 340}))),
        (None, None, Some(1540), Some(json!({"total": 1540}))),
        (
            Some(1200),
            None,
            Some(1540),
            Some(json!({"input": 1200, "total": 1540})),
        ),
        (
            None,
            Some(340),
            Some(1540),
            Some(json!({"output": 340, "total": 1540})),
        ),
        (None, None, None, None),
        (
            Some(0),
            Some(0),
            Some(0),
            Some(json!({"input": 0, "output": 0, "total": 0})),
        ),
        (
            Some(i32::MAX),
            Some(i32::MAX),
            None,
            Some(json!({"input": i32::MAX, "output": i32::MAX})),
        ),
    ];

    for (prompt_tokens, completion_tokens, total_tokens, expected_usage) in cases {
        let mut server = Server::new_async().await;
        let (sender, receiver) = std::sync::mpsc::channel();
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(207)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |request| {
                sender.send(request.body().unwrap().clone()).unwrap();
                br#"{"successes": [], "errors": []}"#.to_vec()
            })
            .create_async()
            .await;

        let client = create_mock_client(&server);
        let id = client
            .generation()
            .trace_id("trace-123")
            .name("llm-call")
            .model("gemini-3.1-pro-preview")
            .maybe_prompt_tokens(prompt_tokens)
            .maybe_completion_tokens(completion_tokens)
            .maybe_total_tokens(total_tokens)
            .call()
            .await
            .expect("generation should be ingested");

        mock.assert_async().await;
        let payload: serde_json::Value =
            serde_json::from_slice(&receiver.try_recv().unwrap()).unwrap();
        let batch = payload["batch"].as_array().unwrap();
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0]["type"], "generation-create");
        let body = &batch[0]["body"];
        assert_eq!(body["id"], id);
        assert_eq!(body["traceId"], "trace-123");
        assert_eq!(body["name"], "llm-call");
        assert_eq!(body["model"], "gemini-3.1-pro-preview");
        // Exact equality also ensures missing counts are neither null nor zero-filled,
        // and no total is calculated locally or overwrites the caller's total.
        assert_eq!(
            body.get("usageDetails"),
            expected_usage.as_ref(),
            "token counts: {prompt_tokens:?}, {completion_tokens:?}, {total_tokens:?}"
        );
        assert!(body.get("usage").is_none());
    }
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
    let client = ClientBuilder::new()
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
            "Expected network error, got: {error_str}"
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

    let result = ClientBuilder::from_env().and_then(|builder| builder.build());
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
