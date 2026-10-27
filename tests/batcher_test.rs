//! Tests for batching and 207 Multi-Status handling

use langfuse_ergonomic::{Batcher, BatcherConfig, ClientBuilder, LangfuseClient};
use mockito::Server;
use serde_json::json;
use std::time::Duration;

/// Helper to create a mock client for testing
fn create_mock_client(server: &Server) -> LangfuseClient {
    ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .expect("mock credentials should be valid")
}

#[tokio::test]
async fn test_metrics_accuracy_on_retries() {
    let mut server = Server::new_async().await;

    // First request fails with 503, second succeeds
    let mock1 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(503)
        .with_body(r#"{"error": "Service unavailable"}"#)
        .expect(1)
        .create_async()
        .await;

    let mock2 = server
        .mock("POST", "/api/public/ingestion")
        .with_status(207)
        .with_body(r#"{"successes": [{"id": "test-event", "status": 201}], "errors": []}"#)
        .expect(1)
        .create_async()
        .await;

    let client = create_mock_client(&server);

    // Use the bon-generated builder
    let batcher = Batcher::builder()
        .client(client)
        .max_retries(1)
        .initial_retry_delay(Duration::from_millis(10))
        .build()
        .await;

    // Add a single event using the correct types from langfuse-client-base
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

    let trace_body = TraceBody {
        id: Some(Some("test-event".to_string())),
        name: Some(Some("test".to_string())),
        timestamp: Some(Some(chrono::Utc::now().fixed_offset())),
        ..Default::default()
    };

    let event_one_of = TraceEvent1 {
        body: Box::new(trace_body),
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
    };

    let event = IngestionEvent::TraceEvent1(Box::new(event_one_of));
    batcher.add(event).await.unwrap();

    // Flush and verify metrics
    let _result = batcher.flush().await.unwrap();
    let metrics = batcher.metrics();

    // Should have 1 successful flush, 1 retry
    assert_eq!(metrics.flushed, 1, "Should have 1 flushed event");
    assert_eq!(metrics.retries, 1, "Should have 1 retry");
    assert_eq!(
        metrics.failed, 0,
        "Should have no permanently failed events"
    );
    assert_eq!(metrics.queued, 0, "Queue should be empty after flush");

    mock1.assert_async().await;
    mock2.assert_async().await;
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
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

    for i in 1..=4 {
        let trace_body = TraceBody {
            id: Some(Some(format!("trace-{i}"))),
            name: Some(Some(format!("Test Trace {i}"))),
            ..Default::default()
        };

        let event = TraceEvent1 {
            body: Box::new(trace_body),
            id: format!("event-{i}"),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };

        let ingestion_event = IngestionEvent::TraceEvent1(Box::new(event));
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
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

    for i in 1..=4 {
        let large_metadata = json!({
            "data": "x".repeat(800), // Each event ~1KB
            "index": i
        });

        let trace_body = TraceBody {
            id: Some(Some(format!("trace-{i}"))),
            name: Some(Some(format!("Large Trace {i}"))),
            metadata: Some(Some(large_metadata)),
            ..Default::default()
        };

        let event = TraceEvent1 {
            body: Box::new(trace_body),
            id: format!("event-{i}"),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };

        let ingestion_event = IngestionEvent::TraceEvent1(Box::new(event));
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
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

    let trace_body = TraceBody {
        id: Some(Some("trace-1".to_string())),
        name: Some(Some("Test Trace".to_string())),
        ..Default::default()
    };

    let event = TraceEvent1 {
        body: Box::new(trace_body),
        id: "event-1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
    };

    let ingestion_event = IngestionEvent::TraceEvent1(Box::new(event));
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
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

    let trace_body = TraceBody {
        id: Some(Some("trace-1".to_string())),
        name: Some(Some("Test Trace".to_string())),
        ..Default::default()
    };

    let event = TraceEvent1 {
        body: Box::new(trace_body),
        id: "event-1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
    };

    let ingestion_event = IngestionEvent::TraceEvent1(Box::new(event));
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
            _ => panic!("Expected Auth error, got: {e:?}"),
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

/// Exercise shutdown with a known retry already buffered. `success_on_request`
/// counts the initial explicit flush too; `None` keeps returning retryable 207s.
async fn characterize_partial_retry_shutdown(
    success_on_request: Option<usize>,
) -> (langfuse_ergonomic::IngestionResponse, Vec<Vec<String>>) {
    use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};
    use std::sync::{Arc, Mutex};

    tokio::time::timeout(Duration::from_secs(5), async {
        let mut server = Server::new_async().await;
        let requests = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
        let recorded = Arc::clone(&requests);
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(207)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |request| {
                let body: serde_json::Value =
                    serde_json::from_slice(request.body().unwrap()).unwrap();
                let ids: Vec<String> = body["batch"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|event| event["id"].as_str().unwrap().to_owned())
                    .collect();
                let mut recorded = recorded.lock().unwrap();
                recorded.push(ids.clone());
                let recovered = success_on_request.is_some_and(|n| recorded.len() >= n);
                let mut successes = Vec::new();
                let mut errors = Vec::new();
                for id in ids {
                    if id == "already-successful" || recovered {
                        successes.push(json!({"id": id, "status": 201}));
                    } else {
                        errors.push(json!({"id": id, "status": 503, "message": "retry me"}));
                    }
                }
                serde_json::to_vec(&json!({"successes": successes, "errors": errors})).unwrap()
            })
            .create_async()
            .await;

        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(100)
            .flush_interval(Duration::from_secs(3600))
            .max_retries(3)
            .initial_retry_delay(Duration::from_millis(1))
            .max_retry_delay(Duration::from_millis(1))
            .retry_jitter(false)
            .build()
            .await;
        for id in ["already-successful", "retryable"] {
            let event = TraceEvent1 {
                body: Box::new(TraceBody::default()),
                id: id.to_owned(),
                timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
                metadata: None,
                r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
            };
            batcher
                .add(IngestionEvent::TraceEvent1(Box::new(event)))
                .await
                .unwrap();
        }

        // Synchronize with channel consumption instead of guessing a sleep.
        // The large batch/interval limits prevent any automatic flush here.
        while batcher.metrics().queued != 2 {
            tokio::task::yield_now().await;
        }
        let first = batcher.flush().await.unwrap();
        assert_eq!(first.success_ids, ["already-successful"]);
        assert_eq!(first.failure_count, 1);
        assert_eq!(first.failures[0].event_id, "retryable");
        assert!(first.failures[0].retryable);
        let metrics = batcher.metrics();
        assert_eq!(metrics.queued, 1, "the 207 failure is queued for retry");
        assert_eq!(metrics.flushed, 1);
        assert_eq!(
            metrics.failed, 0,
            "retryable events are not failed until retries are exhausted"
        );
        assert_eq!(
            metrics.retries, 0,
            "only retries that were sent are counted"
        );
        assert_eq!(metrics.dropped, 0);
        assert_eq!(requests.lock().unwrap().len(), 1);

        let result = batcher.shutdown().await.unwrap();
        let requests = requests.lock().unwrap().clone();
        assert_eq!(requests[0], ["already-successful", "retryable"]);
        for ids in &requests[1..] {
            assert_eq!(ids, &["retryable"], "successful events must not be resent");
        }
        // Counts and bodies above cover every request; a single dynamic mock
        // avoids response ordering depending on mock registration precedence.
        drop(mock);
        (result, requests)
    })
    .await
    .expect("partial retry shutdown must complete within five seconds")
}

#[tokio::test]
async fn test_shutdown_207_recovery_in_final_flush() {
    let (result, requests) = characterize_partial_retry_shutdown(Some(3)).await;
    assert_eq!(requests.len(), 3);
    assert_eq!(result.success_ids, ["retryable"]);
    assert_eq!(result.success_count, 1);
    assert_eq!(result.failure_count, 0);
}

#[tokio::test]
async fn test_shutdown_207_includes_background_success_in_result() {
    let (result, requests) = characterize_partial_retry_shutdown(Some(2)).await;
    assert_eq!(requests.len(), 2);
    assert_eq!(result.success_ids, ["retryable"]);
    assert_eq!(result.success_count, 1);
    assert_eq!(result.failure_count, 0);
}

#[tokio::test]
async fn test_shutdown_207_recovers_on_last_allowed_attempt() {
    let (result, requests) = characterize_partial_retry_shutdown(Some(4)).await;
    assert_eq!(requests.len(), 4, "initial attempt plus three retries");
    assert_eq!(result.success_ids, ["retryable"]);
    assert_eq!(result.success_count, 1);
    assert_eq!(result.failure_count, 0);
}

#[tokio::test]
async fn test_shutdown_207_always_failing_exhausts_retry_budget() {
    let (result, requests) = characterize_partial_retry_shutdown(None).await;
    assert_eq!(requests.len(), 4, "initial attempt plus three retries");
    assert!(result.success_ids.is_empty());
    assert_eq!(result.success_count, 0);
    assert_eq!(result.failure_count, 1);
    assert_eq!(result.failures[0].event_id, "retryable");
    assert!(result.failures[0].retryable);
}

#[tokio::test]
async fn test_shutdown_reconciles_manual_flush_after_background_retry() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let mut server = Server::new_async().await;
        let requests = Arc::new(AtomicUsize::new(0));
        let request_counter = Arc::clone(&requests);
        let body_request_counter = Arc::clone(&requests);
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status_code_from_request(move |_| {
                if request_counter.fetch_add(1, Ordering::SeqCst) == 0 {
                    207
                } else {
                    200
                }
            })
            .with_body_from_request(move |request| {
                let body: serde_json::Value =
                    serde_json::from_slice(request.body().unwrap()).unwrap();
                let id = body["batch"][0]["id"].as_str().unwrap();
                if body_request_counter.load(Ordering::SeqCst) == 1 {
                    serde_json::to_vec(&json!({
                        "successes": [],
                        "errors": [{"id": id, "status": 503, "message": "retry me"}]
                    }))
                    .unwrap()
                } else {
                    serde_json::to_vec(&json!({
                        "successes": [{"id": id, "status": 201}],
                        "errors": []
                    }))
                    .unwrap()
                }
            })
            .expect(2)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(1)
            .max_retries(3)
            .initial_retry_delay(Duration::from_millis(1))
            .max_retry_delay(Duration::from_millis(1))
            .retry_jitter(false)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "manual-recovery".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();

        while requests.load(Ordering::SeqCst) == 0 || batcher.metrics().queued != 1 {
            tokio::task::yield_now().await;
        }
        let manual_response = batcher.flush().await.unwrap();
        assert_eq!(manual_response.success_ids, ["manual-recovery"]);

        let result = batcher.shutdown().await.unwrap();
        assert_eq!(requests.load(Ordering::SeqCst), 2);
        assert!(result.success_ids.is_empty());
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 0);
        mock.assert_async().await;
    })
    .await
    .expect("manual recovery must replace the earlier retryable failure");
}

#[tokio::test]
async fn test_many_manual_flushes_do_not_retain_historical_results() {
    tokio::time::timeout(Duration::from_secs(10), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(207)
            .with_body_from_request(|request| {
                let body: serde_json::Value =
                    serde_json::from_slice(request.body().unwrap()).unwrap();
                let successes: Vec<_> = body["batch"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|event| json!({"id": event["id"], "status": 201}))
                    .collect();
                serde_json::to_vec(&json!({"successes": successes, "errors": []})).unwrap()
            })
            .expect(20)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(100)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;

        for index in 0..20 {
            let id = format!("manual-{index}");
            let event = TraceEvent1 {
                body: Box::new(TraceBody::default()),
                id: id.clone(),
                timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
                metadata: None,
                r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
            };
            batcher
                .add(IngestionEvent::TraceEvent1(Box::new(event)))
                .await
                .unwrap();

            let response = batcher.flush().await.unwrap();
            assert_eq!(response.success_ids, [id]);
            assert_eq!(response.success_count, 1);
            assert_eq!(response.failure_count, 0);
        }

        assert_eq!(batcher.metrics().flushed, 20);
        let shutdown_response = batcher.shutdown().await.unwrap();
        assert!(shutdown_response.success_ids.is_empty());
        assert_eq!(shutdown_response.success_count, 0);
        assert_eq!(shutdown_response.failure_count, 0);
        mock.assert_async().await;
    })
    .await
    .expect("completed manual flushes must not accumulate until shutdown");
}

#[tokio::test]
async fn test_shutdown_propagates_auth_failure_from_drain() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(401)
            .with_body(r#"{"error":"invalid credentials"}"#)
            .expect(1)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "auth-failure".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();

        let result = batcher.shutdown().await;
        assert!(matches!(
            result,
            Err(langfuse_ergonomic::Error::Auth { .. })
        ));
        mock.assert_async().await;
    })
    .await
    .expect("shutdown must surface a drain error within five seconds");
}

#[tokio::test]
async fn test_shutdown_transport_failure_respects_total_retry_budget() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(429)
            .with_body(r#"{"error":"unavailable"}"#)
            .expect(4)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_retries(3)
            .initial_retry_delay(Duration::from_millis(1))
            .max_retry_delay(Duration::from_millis(1))
            .retry_jitter(false)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "transport-failure".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();

        let result = batcher.shutdown().await.unwrap();
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.failures[0].event_id, "transport-failure");
        mock.assert_async().await;
    })
    .await
    .expect("transport retries must finish within five seconds");
}

#[tokio::test]
async fn test_shutdown_mixed_failures_share_retry_budget() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};
        use std::sync::{Arc, Mutex};

        let mut server = Server::new_async().await;
        let request_ids = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&request_ids);
        let status_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let status_requests = Arc::clone(&status_counter);
        let body_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let body_requests = Arc::clone(&body_counter);
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status_code_from_request(move |_| {
                if status_requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    207
                } else {
                    429
                }
            })
            .with_body_from_request(move |request| {
                let body: serde_json::Value = serde_json::from_slice(request.body().unwrap()).unwrap();
                let id = body["batch"][0]["id"].as_str().unwrap().to_owned();
                recorded.lock().unwrap().push(id);
                if body_requests.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    r#"{"successes":[],"errors":[{"id":"mixed-failure","status":503,"message":"partial"}]}"#.as_bytes().to_vec()
                } else {
                    r#"{"error":"unavailable"}"#.as_bytes().to_vec()
                }
            })
            .expect(4)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(100)
            .max_retries(3)
            .initial_retry_delay(Duration::from_millis(1))
            .max_retry_delay(Duration::from_millis(1))
            .retry_jitter(false)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "mixed-failure".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();

        let result = batcher.shutdown().await.unwrap();
        assert_eq!(request_ids.lock().unwrap().len(), 4);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.failures[0].event_id, "mixed-failure");
        mock.assert_async().await;
    })
    .await
    .expect("mixed failures must respect the retry budget");
}

#[tokio::test]
async fn test_shutdown_does_not_retain_prior_background_flush_results() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(200)
            .with_body(r#"{"success":true}"#)
            .expect(1)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(1)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "background-success".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();
        while !mock.matched() || batcher.metrics().flushed != 1 {
            tokio::task::yield_now().await;
        }

        let result = batcher.shutdown().await.unwrap();
        assert!(result.success_ids.is_empty());
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 0);
        mock.assert_async().await;
    })
    .await
    .expect("shutdown must not retain completed background results");
}

#[tokio::test]
async fn test_shutdown_includes_background_flush_in_flight_at_start() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(207)
            .with_body_from_request(|_| {
                std::thread::sleep(Duration::from_millis(300));
                r#"{"successes":[{"id":"in-flight","status":201}],"errors":[]}"#
                    .to_owned()
                    .into()
            })
            .expect(1)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(1)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "in-flight".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();
        while !mock.matched() {
            tokio::task::yield_now().await;
        }

        let shutdown = tokio::spawn(batcher.shutdown());
        tokio::time::sleep(Duration::from_millis(25)).await;
        let result = shutdown.await.unwrap().unwrap();
        assert_eq!(result.success_ids, ["in-flight"]);
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 0);
        mock.assert_async().await;
    })
    .await
    .expect("shutdown must include a background flush already in flight");
}

#[tokio::test]
async fn test_shutdown_surfaces_prior_background_flush_error() {
    tokio::time::timeout(Duration::from_secs(5), async {
        use langfuse_client_base::models::{IngestionEvent, TraceBody, TraceEvent1};

        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .with_status(401)
            .with_body(r#"{"error":"invalid credentials"}"#)
            .expect(1)
            .create_async()
            .await;
        let batcher = Batcher::builder()
            .client(create_mock_client(&server))
            .max_events(1)
            .flush_interval(Duration::from_secs(3600))
            .build()
            .await;
        let event = TraceEvent1 {
            body: Box::new(TraceBody::default()),
            id: "background-auth-failure".to_owned(),
            timestamp: "2026-10-17T00:00:00.000Z".to_owned(),
            metadata: None,
            r#type: langfuse_client_base::models::trace_event_1::Type::TraceCreate,
        };
        batcher
            .add(IngestionEvent::TraceEvent1(Box::new(event)))
            .await
            .unwrap();
        while !mock.matched() {
            tokio::task::yield_now().await;
        }

        let result = batcher.shutdown().await;
        assert!(matches!(
            result,
            Err(langfuse_ergonomic::Error::Auth { .. })
        ));
        mock.assert_async().await;
    })
    .await
    .expect("background worker errors must be surfaced within five seconds");
}
