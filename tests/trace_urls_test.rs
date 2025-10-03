//! Tests for Trace URLs and BYO IDs functionality

use langfuse_ergonomic::{IdGenerator, LangfuseClient};
use mockito::Server;

#[test]
fn test_trace_url_generation() {
    // Test various base URL formats
    let test_cases = vec![
        (
            "https://cloud.langfuse.com",
            "test-123",
            "https://cloud.langfuse.com/trace/test-123",
        ),
        (
            "https://cloud.langfuse.com/",
            "test-456",
            "https://cloud.langfuse.com/trace/test-456",
        ),
        (
            "https://cloud.langfuse.com/api",
            "test-789",
            "https://cloud.langfuse.com/trace/test-789",
        ),
        (
            "http://localhost:3000",
            "local-123",
            "http://localhost:3000/trace/local-123",
        ),
        (
            "http://localhost:3000/api",
            "local-456",
            "http://localhost:3000/trace/local-456",
        ),
    ];

    for (base_url, trace_id, expected_url) in test_cases {
        let response = langfuse_ergonomic::TraceResponse {
            id: trace_id.to_string(),
            base_url: base_url.to_string(),
        };

        assert_eq!(
            response.url(),
            expected_url,
            "Failed for base_url: {}, trace_id: {}",
            base_url,
            trace_id
        );
    }
}

#[test]
fn test_deterministic_id_generation() {
    // Test that same seed produces same ID
    let seed = "test-seed-123";
    let id1 = IdGenerator::from_seed(seed);
    let id2 = IdGenerator::from_seed(seed);

    assert_eq!(id1, id2, "Same seed should produce same ID");

    // Test that different seeds produce different IDs
    let id3 = IdGenerator::from_seed("different-seed");
    assert_ne!(id1, id3, "Different seeds should produce different IDs");
}

#[test]
fn test_component_based_id_generation() {
    // Test hierarchical ID generation
    let components = vec!["user-123", "session-456", "request-789"];
    let id1 = IdGenerator::from_components(&components);

    // Same components should produce same ID
    let id2 = IdGenerator::from_components(&components);
    assert_eq!(id1, id2, "Same components should produce same ID");

    // Different order should produce different ID
    let components_reordered = vec!["session-456", "user-123", "request-789"];
    let id3 = IdGenerator::from_components(&components_reordered);
    assert_ne!(
        id1, id3,
        "Different component order should produce different ID"
    );

    // Different components should produce different ID
    let components_different = vec!["user-999", "session-888", "request-777"];
    let id4 = IdGenerator::from_components(&components_different);
    assert_ne!(id1, id4, "Different components should produce different ID");
}

#[test]
fn test_hash_based_id_generation() {
    // Test hash-based ID generation
    let seed = "hash-seed-123";
    let id1 = IdGenerator::from_hash(seed);
    let id2 = IdGenerator::from_hash(seed);

    assert_eq!(id1, id2, "Same seed should produce same hash ID");

    // Hash IDs should be 16 hex characters
    assert_eq!(id1.len(), 16, "Hash ID should be 16 characters");
    assert!(
        id1.chars().all(|c| c.is_ascii_hexdigit()),
        "Hash ID should be hexadecimal"
    );

    // Different seeds produce different hash IDs
    let id3 = IdGenerator::from_hash("different-hash-seed");
    assert_ne!(
        id1, id3,
        "Different seeds should produce different hash IDs"
    );
}

#[test]
fn test_uuid_v5_format() {
    // Test that from_seed produces valid UUID v5 format
    let seed = "uuid-test-seed";
    let id = IdGenerator::from_seed(seed);

    // UUID format: 8-4-4-4-12 hex characters with hyphens
    let parts: Vec<&str> = id.split('-').collect();
    assert_eq!(
        parts.len(),
        5,
        "UUID should have 5 parts separated by hyphens"
    );

    assert_eq!(parts[0].len(), 8, "First part should be 8 characters");
    assert_eq!(parts[1].len(), 4, "Second part should be 4 characters");
    assert_eq!(parts[2].len(), 4, "Third part should be 4 characters");
    assert_eq!(parts[3].len(), 4, "Fourth part should be 4 characters");
    assert_eq!(parts[4].len(), 12, "Fifth part should be 12 characters");

    // All parts should be hexadecimal
    for part in parts {
        assert!(
            part.chars().all(|c| c.is_ascii_hexdigit()),
            "UUID parts should be hexadecimal"
        );
    }
}

#[tokio::test]
async fn test_trace_with_custom_id() {
    let mut server = Server::new_async().await;

    // Mock the ingestion endpoint
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .expect("mock credentials should be valid");

    // Create trace with custom ID
    let custom_id = "my-custom-trace-id";
    let response = client
        .trace()
        .id(custom_id.to_string())
        .name("Test Trace")
        .call()
        .await
        .unwrap();

    // Verify the response contains our custom ID
    assert_eq!(response.id, custom_id);

    // Verify the URL is correctly generated
    let expected_url = format!("{}/trace/{}", server.url(), custom_id);
    assert_eq!(response.url(), expected_url);

    mock.assert_async().await;
}

#[tokio::test]
async fn test_trace_with_seed_based_id() {
    let mut server = Server::new_async().await;

    // Mock the ingestion endpoint
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .expect("mock credentials should be valid");

    // Generate deterministic ID from seed
    let seed = "test-seed-for-trace";
    let deterministic_id = IdGenerator::from_seed(seed);

    // Create trace with deterministic ID
    let response = client
        .trace()
        .id(deterministic_id.clone())
        .name("Deterministic Trace")
        .call()
        .await
        .unwrap();

    // Verify the response contains our deterministic ID
    assert_eq!(response.id, deterministic_id);

    // Verify reproducibility - same seed should give same ID
    let same_id = IdGenerator::from_seed(seed);
    assert_eq!(deterministic_id, same_id);

    mock.assert_async().await;
}

#[tokio::test]
async fn test_hierarchical_observations() {
    let mut server = Server::new_async().await;

    // Mock the ingestion endpoint for multiple calls
    let mock = server
        .mock("POST", "/api/public/ingestion")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"successes": [], "errors": []}"#)
        .expect(3) // Expect 3 calls: trace, span1, span2
        .create_async()
        .await;

    let client = LangfuseClient::builder()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .build()
        .expect("mock credentials should be valid");

    // Create hierarchical IDs
    let base_seed = "workflow-123";
    let trace_id = IdGenerator::from_components(&[base_seed, "trace"]);
    let span1_id = IdGenerator::from_components(&[base_seed, "span", "step1"]);
    let span2_id = IdGenerator::from_components(&[base_seed, "span", "step2"]);

    // Create trace
    let trace_response = client
        .trace()
        .id(trace_id.clone())
        .name("Workflow Trace")
        .call()
        .await
        .unwrap();

    assert_eq!(trace_response.id, trace_id);

    // Create first span
    let span1_response = client
        .span()
        .trace_id(trace_id.clone())
        .id(span1_id.clone())
        .name("Step 1")
        .call()
        .await
        .unwrap();

    assert_eq!(span1_response, span1_id);

    // Create nested span
    let span2_response = client
        .span()
        .trace_id(trace_id.clone())
        .id(span2_id.clone())
        .parent_observation_id(span1_id.clone())
        .name("Step 2")
        .call()
        .await
        .unwrap();

    assert_eq!(span2_response, span2_id);

    // Verify all IDs are deterministic and reproducible
    assert_eq!(
        trace_id,
        IdGenerator::from_components(&[base_seed, "trace"])
    );
    assert_eq!(
        span1_id,
        IdGenerator::from_components(&[base_seed, "span", "step1"])
    );
    assert_eq!(
        span2_id,
        IdGenerator::from_components(&[base_seed, "span", "step2"])
    );

    mock.assert_async().await;
}
