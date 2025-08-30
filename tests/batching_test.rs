//! Comprehensive tests for batching functionality

use langfuse_client_base::models::{IngestionEvent, TraceBody};
use langfuse_ergonomic::{BackpressurePolicy, Batcher, LangfuseClient};
use mockito::Server;
use std::time::Duration;
use tokio;

/// Helper to create a test trace event
fn create_test_event(id: &str) -> IngestionEvent {
    IngestionEvent::IngestionEventOneOf(Box::new(TraceBody {
        id: Some(Some(id.to_string())),
        name: Some(Some(format!("Test event {}", id))),
        timestamp: None,
        metadata: None,
        input: None,
        output: None,
        version: None,
        release: None,
        user_id: None,
        session_id: None,
        tags: None,
        public: None,
        environment: None,
    }))
}

#[tokio::test]
async fn test_207_mixed_response() {
    let mut server = Server::new_async().await;

    // Mock returning 207 with mixed results
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "successes": [
                {"id": "success-1", "status": 200},
                {"id": "success-2", "status": 201}
            ],
            "errors": [
                {"id": "fail-1", "status": 500, "message": "Internal error", "error": "Server error"},
                {"id": "fail-2", "status": 400, "message": "Validation failed"}
            ]
        }"#)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder()
        .client(client)
        .max_retries(0) // No retries for this test
        .build();

    // Add events
    batcher.add(create_test_event("success-1")).await.unwrap();
    batcher.add(create_test_event("success-2")).await.unwrap();
    batcher.add(create_test_event("fail-1")).await.unwrap();
    batcher.add(create_test_event("fail-2")).await.unwrap();

    // Flush and check response
    let response = batcher.flush().await.unwrap();

    assert_eq!(response.success_count, 2);
    assert_eq!(response.failure_count, 2);
    assert!(response.success_ids.contains(&"success-1".to_string()));
    assert!(response.success_ids.contains(&"success-2".to_string()));

    // Check failure details
    let fail1 = response
        .failures
        .iter()
        .find(|f| f.event_id == "fail-1")
        .unwrap();
    assert!(fail1.retryable);
    assert_eq!(fail1.code, Some("500".to_string()));

    let fail2 = response
        .failures
        .iter()
        .find(|f| f.event_id == "fail-2")
        .unwrap();
    assert!(!fail2.retryable);
    assert_eq!(fail2.code, Some("400".to_string()));

    mock.assert_async().await;
}

#[tokio::test]
async fn test_retry_after_header() {
    let mut server = Server::new_async().await;

    // First request returns 429 with Retry-After
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(429)
        .with_header("retry-after", "1")
        .with_header("x-request-id", "req-123")
        .with_body("Rate limited")
        .expect(1)
        .create_async()
        .await;

    // Second request succeeds
    let mock2 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder().client(client).max_retries(1).build();

    batcher.add(create_test_event("test-1")).await.unwrap();

    let start = std::time::Instant::now();
    let response = batcher.flush().await.unwrap();
    let elapsed = start.elapsed();

    // Should have waited at least 1 second
    assert!(elapsed >= Duration::from_secs(1));
    assert_eq!(response.success_count, 1);

    mock1.assert_async().await;
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_413_payload_too_large() {
    let mut server = Server::new_async().await;

    // First request returns 413 (batch too large)
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(413)
        .with_body("Payload too large")
        .expect(1)
        .create_async()
        .await;

    // Subsequent smaller requests succeed
    let mock2 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(2) // Split into 2 smaller batches
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder().client(client).build();

    // Add multiple events
    batcher.add(create_test_event("test-1")).await.unwrap();
    batcher.add(create_test_event("test-2")).await.unwrap();

    let response = batcher.flush().await.unwrap();
    assert_eq!(response.success_count, 2);

    mock1.assert_async().await;
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_413_single_event_too_large() {
    let server = Server::new_async().await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder()
        .client(client)
        .max_bytes(100) // Very small limit
        .build();

    // Try to add an event larger than max_bytes
    let large_event = create_test_event(&"x".repeat(200));
    let result = batcher.add(large_event).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        langfuse_ergonomic::Error::BatchSizeExceeded { size, max_size } => {
            assert!(size > max_size);
            assert_eq!(max_size, 100);
        }
        _ => panic!("Expected BatchSizeExceeded error"),
    }
}

#[tokio::test]
async fn test_backpressure_drop_new() {
    let server = Server::new_async().await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder()
        .client(client)
        .max_queue_size(2)
        .backpressure_policy(BackpressurePolicy::DropNew)
        .build();

    // Fill the queue
    batcher.add(create_test_event("test-1")).await.unwrap();
    batcher.add(create_test_event("test-2")).await.unwrap();

    // This should be dropped
    let result = batcher.add(create_test_event("test-3")).await;
    assert!(result.is_err());

    let metrics = batcher.metrics();
    assert!(metrics.dropped > 0);
}

#[tokio::test]
async fn test_concurrent_flush_protection() {
    let mut server = Server::new_async().await;

    // Only one flush should happen
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder().client(client).build();

    batcher.add(create_test_event("test-1")).await.unwrap();

    // Start multiple concurrent flushes
    let batcher_clone1 = &batcher;
    let batcher_clone2 = &batcher;

    let (result1, result2) = tokio::join!(batcher_clone1.flush(), batcher_clone2.flush());

    // Both should succeed but only one actual flush
    assert!(result1.is_ok());
    assert!(result2.is_ok());

    mock.assert_async().await;
}

#[tokio::test]
async fn test_shutdown_idempotency() {
    let server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder().client(client).build();

    batcher.add(create_test_event("test-1")).await.unwrap();

    // Shutdown consumes batcher, so we can only call it once
    let result = batcher.shutdown().await;
    assert!(result.is_ok());

    mock.assert_async().await;
}

#[tokio::test]
async fn test_metrics_tracking() {
    let mut server = Server::new_async().await;

    // Success response
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    // 207 mixed response
    let mock2 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_body(
            r#"{
            "successes": [{"id": "s1"}],
            "errors": [{"id": "f1", "status": 400, "message": "Bad request"}]
        }"#,
        )
        .expect(1)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder().client(client).build();

    // Add and flush first batch
    batcher.add(create_test_event("test-1")).await.unwrap();
    batcher.flush().await.unwrap();

    let metrics1 = batcher.metrics();
    assert_eq!(metrics1.flushed, 1);
    assert_eq!(metrics1.failed, 0);

    // Add and flush mixed batch
    batcher.add(create_test_event("s1")).await.unwrap();
    batcher.add(create_test_event("f1")).await.unwrap();
    batcher.flush().await.unwrap();

    let metrics2 = batcher.metrics();
    assert_eq!(metrics2.flushed, 2); // 1 from first batch + 1 from mixed
    assert_eq!(metrics2.failed, 1); // 1 from mixed batch

    mock1.assert_async().await;
    mock2.assert_async().await;
}

#[tokio::test]
async fn test_auto_flush_on_size() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build();

    let batcher = Batcher::builder()
        .client(client)
        .max_events(2) // Auto-flush after 2 events
        .build();

    // Add 2 events - should trigger auto-flush
    batcher.add(create_test_event("test-1")).await.unwrap();
    batcher.add(create_test_event("test-2")).await.unwrap();

    // Wait for auto-flush
    tokio::time::sleep(Duration::from_millis(100)).await;

    mock.assert_async().await;
}
