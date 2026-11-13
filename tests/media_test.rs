//! Media uploads and downloads against a mock Langfuse.
#![cfg(not(target_family = "wasm"))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use langfuse_ergonomic::media::{MediaField, MediaReference};
use langfuse_ergonomic::{ApiVersion, ClientBuilder};
use mockito::Matcher;
use timvw_rust_runtime::Upload;

fn client(url: String) -> langfuse_ergonomic::LangfuseClient {
    ClientBuilder::new()
        .public_key("pk")
        .secret_key("sk")
        .base_url(url)
        .api_version(ApiVersion::V3)
        .build()
        .unwrap()
}

#[tokio::test]
async fn upload_gets_a_url_puts_the_bytes_and_reports_the_upload() {
    let mut server = mockito::Server::new_async().await;
    // SHA-256 of "hello", base64.
    let sha = "LPJNul+wow4m6DsqxbninhsWHlwfp0JecwQzYpOLmCQ=";
    let get_url = server
        .mock("POST", "/api/public/media")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "traceId": "trace-1",
            "contentType": "image/png",
            "contentLength": 5,
            "sha256Hash": sha,
            "field": "output",
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"mediaId":"m-1","uploadUrl":"{}/bucket/m-1"}}"#,
            server.url()
        ))
        .create_async()
        .await;
    let put = server
        .mock("PUT", "/bucket/m-1")
        .match_header("content-type", "image/png")
        .match_header("x-amz-checksum-sha256", sha)
        .match_body("hello")
        .with_status(200)
        .create_async()
        .await;
    let patch = server
        .mock("PATCH", "/api/public/media/m-1")
        .match_body(Matcher::PartialJson(
            serde_json::json!({"uploadHttpStatus": 200}),
        ))
        .with_status(204)
        .create_async()
        .await;

    let reference = client(server.url())
        .upload_media()
        .upload(Upload::bytes(b"hello".to_vec()).mime("image/png"))
        .trace_id("trace-1")
        .field(MediaField::Output)
        .call()
        .await
        .unwrap();
    assert_eq!(
        reference.reference_string(),
        "@@@langfuseMedia:type=image/png|id=m-1|source=bytes@@@"
    );
    get_url.assert_async().await;
    put.assert_async().await;
    patch.assert_async().await;
}

#[tokio::test]
async fn known_media_is_not_uploaded_again() {
    let mut server = mockito::Server::new_async().await;
    let get_url = server
        .mock("POST", "/api/public/media")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"mediaId":"m-2","uploadUrl":null}"#)
        .create_async()
        .await;
    let reference = client(server.url())
        .upload_media()
        .upload(Upload::bytes(b"again".to_vec()))
        .content_type("application/pdf")
        .trace_id("trace-1")
        .call()
        .await
        .unwrap();
    assert_eq!(reference.media_id, "m-2");
    get_url.assert_async().await;
}

#[tokio::test]
async fn failed_uploads_are_reported_and_returned() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("POST", "/api/public/media")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"mediaId":"m-3","uploadUrl":"{}/bucket/m-3"}}"#,
            server.url()
        ))
        .create_async()
        .await;
    server
        .mock("PUT", "/bucket/m-3")
        .with_status(403)
        .create_async()
        .await;
    let patch = server
        .mock("PATCH", "/api/public/media/m-3")
        .match_body(Matcher::PartialJson(
            serde_json::json!({"uploadHttpStatus": 403}),
        ))
        .with_status(204)
        .create_async()
        .await;
    let error = client(server.url())
        .upload_media()
        .upload(Upload::bytes(b"x".to_vec()).mime("image/jpeg"))
        .trace_id("t")
        .call()
        .await
        .unwrap_err();
    assert!(error.to_string().contains("403"), "{error}");
    patch.assert_async().await;
}

#[tokio::test]
async fn unsupported_types_and_missing_targets_fail_before_any_request() {
    let server = mockito::Server::new_async().await;
    let client = client(server.url());
    assert!(client
        .upload_media()
        .upload(Upload::bytes(b"x".to_vec()).mime("application/x-unknown"))
        .trace_id("t")
        .call()
        .await
        .is_err());
    assert!(client
        .upload_media()
        .upload(Upload::bytes(b"x".to_vec()).mime("image/png"))
        .call()
        .await
        .is_err());
}

#[tokio::test]
async fn download_reads_the_record_and_its_bytes() {
    let mut server = mockito::Server::new_async().await;
    server
        .mock("GET", "/api/public/media/m-4")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!(
            r#"{{"mediaId":"m-4","contentType":"text/plain","contentLength":2,"uploadedAt":"2026-11-01T00:00:00Z","url":"{}/files/m-4","urlExpiry":"2026-11-01T01:00:00Z"}}"#,
            server.url()
        ))
        .create_async()
        .await;
    server
        .mock("GET", "/files/m-4")
        .with_status(200)
        .with_body("hi")
        .create_async()
        .await;
    let (media, bytes) = client(server.url()).download_media("m-4").await.unwrap();
    assert_eq!(media.content_type, "text/plain");
    assert_eq!(bytes, b"hi");
    assert_eq!(
        MediaReference::parse("@@@langfuseMedia:type=text/plain|id=m-4|source=bytes@@@")
            .unwrap()
            .media_id,
        "m-4"
    );
}
