//! Mock-server tests for the Langfuse v4 read path (`GET /api/public/v2/observations`) and
//! for the v3-only reads in V4 mode.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use chrono::{TimeZone, Utc};
use langfuse_ergonomic::{
    otel, ApiVersion, ClientBuilder, Error, LangfuseClient, ObservationLevel,
    ALL_OBSERVATION_FIELDS, MAX_OBSERVATIONS_PAGE_SIZE,
};
use mockito::{Matcher, Server, ServerGuard};
use serde_json::json;

const OBSERVATIONS_PATH: &str = "/api/public/v2/observations";

fn client(server: &ServerGuard, version: ApiVersion) -> LangfuseClient {
    ClientBuilder::new()
        .public_key("pk-test")
        .secret_key("sk-test")
        .base_url(server.url())
        .api_version(version)
        .build()
        .unwrap()
}

fn observation(id: &str, trace_id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "traceId": trace_id,
        "startTime": "2026-11-10T12:00:00.000Z",
        "endTime": null,
        "parentObservationId": null,
        // The server always sends these, as null unless the `model` group is requested.
        "modelId": null,
        "inputPrice": null,
        "outputPrice": null,
        "totalPrice": null,
        "projectId": "project-1",
        "type": "SPAN",
        "name": format!("obs {id}"),
        "isRootObservation": false
    })
}

#[tokio::test]
async fn observations_sends_filters_as_query_parameters() {
    let mut server = Server::new_async().await;
    let trace_hex = otel::trace_id("my-trace");
    let parent_hex = otel::observation_id("parent");
    let mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("traceId".into(), trace_hex.clone()),
            Matcher::UrlEncoded("parentObservationId".into(), parent_hex),
            Matcher::UrlEncoded("isRootObservation".into(), "false".into()),
            Matcher::UrlEncoded("fromStartTime".into(), "2026-11-01T00:00:00+00:00".into()),
            Matcher::UrlEncoded("toStartTime".into(), "2026-11-02T00:00:00+00:00".into()),
            Matcher::UrlEncoded("userId".into(), "user-1".into()),
            Matcher::UrlEncoded("sessionId".into(), "session-1".into()),
            Matcher::UrlEncoded("type".into(), "GENERATION".into()),
            Matcher::UrlEncoded("name".into(), "llm".into()),
            Matcher::UrlEncoded("level".into(), "ERROR".into()),
            // A repeated key; `UrlEncoded` only sees one value per key.
            Matcher::Regex("environment=prod&environment=staging".into()),
            Matcher::UrlEncoded("version".into(), "v1".into()),
            Matcher::UrlEncoded("fields".into(), ALL_OBSERVATION_FIELDS.into()),
            Matcher::UrlEncoded("limit".into(), "25".into()),
            Matcher::UrlEncoded("cursor".into(), "abc".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "data": [observation("o1", &trace_hex)],
                "meta": {"cursor": "next-1"}
            })
            .to_string(),
        )
        .expect(1)
        .create_async()
        .await;
    let page = client(&server, ApiVersion::V4)
        .observations()
        .trace_id("my-trace")
        .parent_observation_id("parent")
        .is_root_observation(false)
        .from_start_time(Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap())
        .to_start_time(Utc.with_ymd_and_hms(2026, 11, 2, 0, 0, 0).unwrap())
        .user_id("user-1")
        .session_id("session-1")
        .observation_type("GENERATION")
        .name("llm")
        .level(ObservationLevel::Error)
        .environment(vec!["prod".to_owned(), "staging".to_owned()])
        .version("v1")
        .fields(ALL_OBSERVATION_FIELDS)
        .limit(25)
        .cursor("abc")
        .call()
        .await
        .unwrap();

    mock.assert_async().await;
    assert_eq!(page.data.len(), 1);
    assert_eq!(page.data[0].id, "o1");
    assert_eq!(page.next_cursor.as_deref(), Some("next-1"));
    assert!(page.has_next_page());
}

#[tokio::test]
async fn observations_last_page_has_no_cursor() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[],"meta":{}}"#)
        .create_async()
        .await;
    let page = client(&server, ApiVersion::V4)
        .observations()
        .call()
        .await
        .unwrap();
    assert!(page.data.is_empty());
    assert_eq!(page.next_cursor, None);
    assert!(!page.has_next_page());
}

#[tokio::test]
async fn v3_client_passes_ids_unchanged() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::UrlEncoded("traceId".into(), "my-trace".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[],"meta":{}}"#)
        .expect(1)
        .create_async()
        .await;
    client(&server, ApiVersion::V3)
        .observations()
        .trace_id("my-trace")
        .call()
        .await
        .unwrap();
    mock.assert_async().await;
}

#[tokio::test]
async fn trace_observations_follows_the_cursor() {
    let mut server = Server::new_async().await;
    let trace_hex = otel::trace_id("my-trace");
    let queries = Arc::new(Mutex::new(Vec::new()));
    let sink = queries.clone();
    let body_trace = trace_hex.clone();
    let mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("traceId".into(), trace_hex.clone()),
            Matcher::UrlEncoded("fields".into(), ALL_OBSERVATION_FIELDS.into()),
            Matcher::UrlEncoded("limit".into(), MAX_OBSERVATIONS_PAGE_SIZE.to_string()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body_from_request(move |request| {
            let query = request.path_and_query().to_owned();
            sink.lock().unwrap().push(query.clone());
            let body = if query.contains("cursor=page-2") {
                json!({"data": [observation("o3", &body_trace)], "meta": {}})
            } else if query.contains("cursor=page-1b") {
                // An empty page with a cursor: keep going.
                json!({"data": [], "meta": {"cursor": "page-2"}})
            } else {
                json!({
                    "data": [observation("o1", &body_trace), observation("o2", &body_trace)],
                    "meta": {"cursor": "page-1b"}
                })
            };
            body.to_string().into_bytes()
        })
        .expect(3)
        .create_async()
        .await;

    let observations = client(&server, ApiVersion::V4)
        .trace_observations("my-trace")
        .await
        .unwrap();

    mock.assert_async().await;
    let ids: Vec<_> = observations.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, ["o1", "o2", "o3"]);
    let queries = queries.lock().unwrap();
    assert!(!queries[0].contains("cursor="));
    assert!(queries[1].contains("cursor=page-1b"));
    assert!(queries[2].contains("cursor=page-2"));
}

#[tokio::test]
async fn trace_observations_stops_on_a_repeated_cursor() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[],"meta":{"cursor":"same"}}"#)
        .expect(2)
        .create_async()
        .await;
    let err = client(&server, ApiVersion::V4)
        .trace_observations("t")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Api(_)), "{err:?}");
}

#[tokio::test]
async fn observations_maps_http_errors() {
    let mut server = Server::new_async().await;
    let _mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .with_status(401)
        .with_body(r#"{"message":"bad key"}"#)
        .create_async()
        .await;
    let err = client(&server, ApiVersion::V4)
        .observations()
        .call()
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Auth { .. }), "{err:?}");
}

fn assert_v3_only(result: Result<impl std::fmt::Debug, Error>, method: &str) {
    match result {
        Err(Error::Configuration(message)) => {
            assert!(message.contains(method), "{message}");
            assert!(message.contains("ApiVersion::V3"), "{message}");
        }
        other => panic!("{method}: expected a configuration error, got {other:?}"),
    }
}

#[tokio::test]
#[allow(deprecated)]
async fn v3_only_reads_fail_in_v4_without_a_request() {
    let mut server = Server::new_async().await;
    let any_get = server
        .mock("GET", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let any_delete = server
        .mock("DELETE", Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let client = client(&server, ApiVersion::V4);

    assert_v3_only(client.get_trace("t").await, "get_trace");
    assert_v3_only(client.list_traces().limit(1).call().await, "list_traces");
    assert_v3_only(client.get_observation("o").await, "get_observation");
    assert_v3_only(
        client.get_observations().trace_id("t").call().await,
        "get_observations",
    );
    assert_v3_only(client.get_dataset_run("d", "r").await, "get_dataset_run");
    assert_v3_only(client.get_dataset_runs("d").await, "get_dataset_runs");
    assert_v3_only(
        client.delete_dataset_run("d", "r").await,
        "delete_dataset_run",
    );

    any_get.assert_async().await;
    any_delete.assert_async().await;
}

#[tokio::test]
async fn delete_trace_works_in_v4_with_mapped_id() {
    let mut server = Server::new_async().await;
    let path = format!("/api/public/traces/{}", otel::trace_id("my-trace"));
    let single = server
        .mock("DELETE", path.as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"ok"}"#)
        .expect(1)
        .create_async()
        .await;
    let multiple = server
        .mock("DELETE", "/api/public/traces")
        .match_body(Matcher::PartialJson(json!({
            "traceIds": [otel::trace_id("a"), otel::trace_id("b")]
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"ok"}"#)
        .expect(1)
        .create_async()
        .await;

    let client = client(&server, ApiVersion::V4);
    client.delete_trace("my-trace").await.unwrap();
    client
        .delete_multiple_traces(vec!["a".to_owned(), "b".to_owned()])
        .await
        .unwrap();
    single.assert_async().await;
    multiple.assert_async().await;
}

#[tokio::test]
async fn observations_rejects_limits_above_the_server_maximum() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .expect(0)
        .create_async()
        .await;
    let client = client(&server, ApiVersion::V4);
    for limit in [0, MAX_OBSERVATIONS_PAGE_SIZE + 1] {
        let err = client.observations().limit(limit).call().await.unwrap_err();
        assert!(matches!(err, Error::Validation(_)), "{limit}: {err:?}");
    }
    mock.assert_async().await;
}

#[tokio::test]
async fn observations_rounds_non_integer_usage() {
    let mut server = Server::new_async().await;
    let mut item = observation("o1", "t");
    item["usageDetails"] = json!({"input": 10, "output": 2.6});
    let _mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({"data": [item], "meta": {}}).to_string())
        .create_async()
        .await;
    let page = client(&server, ApiVersion::V4)
        .observations()
        .call()
        .await
        .unwrap();
    let usage = page.data[0].usage_details.clone().flatten().unwrap();
    assert_eq!(usage["input"], 10);
    assert_eq!(usage["output"], 3);
}

#[tokio::test]
async fn trace_observations_keeps_the_latest_row_per_id() {
    let mut server = Server::new_async().await;
    let mut old = observation("o1", "t");
    old["name"] = json!("old");
    old["updatedAt"] = json!("2026-11-10T12:00:00.000Z");
    let mut new = observation("o1", "t");
    new["name"] = json!("new");
    new["updatedAt"] = json!("2026-11-10T12:00:05.000Z");
    let other = observation("o2", "t");
    let _mock = server
        .mock("GET", OBSERVATIONS_PATH)
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({"data": [new, other, old], "meta": {}}).to_string())
        .create_async()
        .await;
    let observations = client(&server, ApiVersion::V4)
        .trace_observations("t")
        .await
        .unwrap();
    let names: Vec<_> = observations
        .iter()
        .map(|o| (o.id.as_str(), o.name.clone().flatten()))
        .collect();
    assert_eq!(
        names,
        [
            ("o1", Some("new".to_owned())),
            ("o2", Some("obs o2".to_owned()))
        ]
    );
}
