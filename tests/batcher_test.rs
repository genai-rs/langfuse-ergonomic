//! Tests for batching and 207 Multi-Status handling

use langfuse_ergonomic::{Batcher, BatcherConfig, LangfuseClient};
use mockito::Server;
use serde_json::json;
use std::time::Duration;

/// Helper to create a mock client for testing
fn create_mock_client(server: &Server) -> LangfuseClient {
    LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
}

#[tokio::test]
async fn test_batch_207_partial_success() {
    let mut server = Server::new_async().await;

    // Mock a 207 Multi-Status response with partial failures
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_header("x-request-id", "test-request-123")
        .with_body(
            r#"{
            "successes": [
                {"id": "event-1", "status": 201},
                {"id": "event-3", "status": 201}
            ],
            "errors": [
                {
                    "id": "event-2",
                    "status": 500,
                    "message": "Internal server error processing event",
                    "error": "SERVER_ERROR"
                },
                {
                    "id": "event-4",
                    "status": 400,
                    "message": "Invalid event format",
                    "error": "VALIDATION_ERROR"
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let client = create_mock_client(&server);
    let batcher = Batcher::builder()
        .client(client)
        .max_events(10)
        .max_retries(2)
        .build()
        .await;

    // Add test events
    use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};

    for i in 1..=4 {
        let trace_body = TraceBody {
            id: Some(Some(format!("trace-{}", i))),
            name: Some(Some(format!("Test Trace {}", i))),
            ..Default::default()
        };

        let event = IngestionEventOneOf {
            body: Box::new(trace_body),
            id: format!("event-{}", i),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        };

        let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
        batcher.add(ingestion_event).await.unwrap();
    }

    // Flush and check results
    let result = batcher.flush().await.unwrap();

    mock.assert_async().await;

    // Verify partial success handling
    assert_eq!(result.success_count, 2, "Should have 2 successful events");
    assert_eq!(result.failure_count, 2, "Should have 2 failed events");
    assert!(result.success_ids.contains(&"event-1".to_string()));
    assert!(result.success_ids.contains(&"event-3".to_string()));

    // Check failure details
    let event_2_failure = result
        .failures
        .iter()
        .find(|f| f.event_id == "event-2")
        .expect("Should have failure for event-2");
    assert!(event_2_failure.retryable, "500 errors should be retryable");
    assert!(event_2_failure.message.contains("Internal server error"));

    let event_4_failure = result
        .failures
        .iter()
        .find(|f| f.event_id == "event-4")
        .expect("Should have failure for event-4");
    assert!(
        !event_4_failure.retryable,
        "400 errors should not be retryable"
    );
    assert!(event_4_failure.message.contains("Invalid event format"));
}

#[tokio::test]
async fn test_batch_size_chunking() {
    let mut server = Server::new_async().await;

    // Expect multiple batch requests due to size limit
    // With 4 events of ~1KB each and max_bytes=2KB, we'll get:
    // - Auto-flush after event 2 (2KB)
    // - Auto-flush after event 4 (2KB)
    // - Manual flush (empty, everything already flushed)
    // So we expect 2 actual requests
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect_at_least(2)
        .expect_at_most(3) // May get an extra flush if timing is off
        .create_async()
        .await;

    let client = create_mock_client(&server);
    let batcher = Batcher::builder()
        .client(client)
        .max_events(100)
        .max_bytes(2000) // Very small limit to force chunking
        .flush_interval(Duration::from_secs(60)) // Long interval to prevent auto-flush
        .build()
        .await;

    // Add large events that will exceed the size limit
    use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};

    for i in 1..=4 {
        let large_metadata = json!({
            "data": "x".repeat(800), // Each event ~1KB
            "index": i
        });

        let trace_body = TraceBody {
            id: Some(Some(format!("trace-{}", i))),
            name: Some(Some(format!("Large Trace {}", i))),
            metadata: Some(Some(large_metadata)),
            ..Default::default()
        };

        let event = IngestionEventOneOf {
            body: Box::new(trace_body),
            id: format!("event-{}", i),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
        };

        let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
        batcher.add(ingestion_event).await.unwrap();
    }

    // Give background task time to process and auto-flush
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Flush and verify chunking happened
    let _result = batcher.flush().await.unwrap();

    // The mock assertion verifies that 2 requests were made (chunking happened)
    mock1.assert_async().await;
}

#[tokio::test]
async fn test_batch_retry_on_rate_limit() {
    let mut server = Server::new_async().await;

    // First request returns 429, second succeeds
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(429)
        .with_header("retry-after", "1")
        .with_body(r#"{"error": "Rate limit exceeded"}"#)
        .expect(1)
        .create_async()
        .await;

    let mock2 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    let client = create_mock_client(&server);
    let batcher = Batcher::builder()
        .client(client)
        .max_retries(2)
        .build()
        .await;

    // Add a test event
    use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};

    let trace_body = TraceBody {
        id: Some(Some("trace-1".to_string())),
        name: Some(Some("Test Trace".to_string())),
        ..Default::default()
    };

    let event = IngestionEventOneOf {
        body: Box::new(trace_body),
        id: "event-1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
    };

    let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
    batcher.add(ingestion_event).await.unwrap();

    // Flush - should retry after rate limit
    let result = batcher.flush().await.unwrap();

    mock1.assert_async().await;
    mock2.assert_async().await;

    assert_eq!(result.success_count, 1, "Event should succeed after retry");
    assert_eq!(result.failure_count, 0, "No failures after retry");
}

#[tokio::test]
async fn test_batch_auth_failure_no_retry() {
    let mut server = Server::new_async().await;

    // Auth failure - should not retry
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(401)
        .with_header("x-request-id", "auth-fail-123")
        .with_body(r#"{"error": "Invalid API credentials"}"#)
        .expect(1) // Should only try once
        .create_async()
        .await;

    let client = create_mock_client(&server);
    let batcher = Batcher::builder()
        .client(client)
        .max_retries(3) // Even with retries, auth should fail fast
        .build()
        .await;

    // Add a test event
    use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};

    let trace_body = TraceBody {
        id: Some(Some("trace-1".to_string())),
        name: Some(Some("Test Trace".to_string())),
        ..Default::default()
    };

    let event = IngestionEventOneOf {
        body: Box::new(trace_body),
        id: "event-1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
    };

    let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));
    batcher.add(ingestion_event).await.unwrap();

    // Flush - should fail immediately without retry
    let result = batcher.flush().await;

    mock.assert_async().await;

    assert!(result.is_err(), "Auth failure should return error");
    if let Err(e) = result {
        match e {
            langfuse_ergonomic::Error::Auth { .. } => {
                // Expected auth error
            }
            _ => panic!("Expected Auth error, got: {:?}", e),
        }
    }
}

#[test]
fn test_batcher_config_defaults() {
    let config = BatcherConfig::default();
    assert_eq!(config.max_events, 100);
    assert_eq!(config.max_bytes, 3_500_000);
    assert_eq!(config.flush_interval, Duration::from_secs(5));
    assert_eq!(config.max_retries, 3);
    assert!(!config.fail_fast);
}
