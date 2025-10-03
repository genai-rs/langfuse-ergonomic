//! Simple test to debug batching issues

use langfuse_ergonomic::{Batcher, ClientBuilder};
use mockito::Server;

#[tokio::test]
async fn test_simple_batch_200() {
    let mut server = Server::new_async().await;

    // Mock a simple 200 response
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"success": true}"#)
        .create_async()
        .await;

    let client = ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .expect("mock credentials should be valid");

    let batcher = Batcher::builder().client(client).build().await;

    // Add a simple event
    use langfuse_client_base::models::{IngestionEvent, IngestionEventOneOf, TraceBody};

    let trace_body = TraceBody {
        id: Some(Some("test-trace-1".to_string())),
        name: Some(Some("Test Trace".to_string())),
        ..Default::default()
    };

    let event = IngestionEventOneOf {
        body: Box::new(trace_body),
        id: "test-event-1".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        metadata: None,
        r#type: langfuse_client_base::models::ingestion_event_one_of::Type::TraceCreate,
    };

    let ingestion_event = IngestionEvent::IngestionEventOneOf(Box::new(event));

    // Add and flush
    batcher.add(ingestion_event).await.unwrap();

    let result = batcher.flush().await.unwrap();

    mock.assert_async().await;

    assert_eq!(result.success_count, 1, "Should have 1 successful event");
    assert_eq!(result.failure_count, 0, "Should have 0 failed events");
    assert!(result.success_ids.contains(&"test-event-1".to_string()));
}
