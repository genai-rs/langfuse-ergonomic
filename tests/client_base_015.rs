//! Protocol regressions for the client-base 0.15 migration, without live credentials.

use chrono::{DateTime, Utc};
use langfuse_ergonomic::{ClientBuilder, Error, LangfuseClient, ObservationsViewSingle};
use mockito::{Matcher, Server};
use serde_json::json;

fn client(server: &Server) -> LangfuseClient {
    ClientBuilder::new()
        .public_key("pk-lf-test")
        .secret_key("sk-lf-test")
        .base_url(server.url())
        .build()
        .unwrap()
}

#[tokio::test]
async fn ingestion_preserves_event_types_dates_and_usage() {
    let mut server = Server::new_async().await;
    let client = client(&server);
    let start: DateTime<Utc> = "2026-10-03T12:34:56.123Z".parse().unwrap();
    let end = start + chrono::Duration::seconds(2);
    let completion = start + chrono::Duration::seconds(1);

    for kind in [
        "trace-create",
        "span-create",
        "generation-create",
        "event-create",
        "span-update",
        "generation-update",
    ] {
        let mut body = json!({"id": "observation-1"});
        if kind == "trace-create" {
            body["timestamp"] = json!(start.fixed_offset());
        } else {
            body["traceId"] = json!("trace-1");
            body["startTime"] = json!(start.fixed_offset());
        }
        if matches!(
            kind,
            "span-create" | "generation-create" | "span-update" | "generation-update"
        ) {
            body["endTime"] = json!(end.fixed_offset());
        }
        if kind == "generation-create" {
            body["usageDetails"] = json!({"input": 1200, "output": 340, "total": 1600});
        }
        if kind == "generation-update" {
            body["completionStartTime"] = json!(completion.fixed_offset());
        }
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .match_body(Matcher::PartialJson(
                json!({"batch": [{"type": kind, "body": body}]}),
            ))
            .with_status(207)
            .with_header("content-type", "application/json")
            .with_body(r#"{"successes": [], "errors": []}"#)
            .create_async()
            .await;

        match kind {
            "trace-create" => {
                client
                    .trace()
                    .id("observation-1")
                    .timestamp(start)
                    .call()
                    .await
                    .unwrap();
            }
            "span-create" => {
                client
                    .span()
                    .trace_id("trace-1")
                    .id("observation-1")
                    .start_time(start)
                    .end_time(end)
                    .call()
                    .await
                    .unwrap();
            }
            "generation-create" => {
                client
                    .generation()
                    .trace_id("trace-1")
                    .id("observation-1")
                    .start_time(start)
                    .end_time(end)
                    .prompt_tokens(1200)
                    .completion_tokens(340)
                    .total_tokens(1600)
                    .call()
                    .await
                    .unwrap();
            }
            "event-create" => {
                client
                    .event()
                    .trace_id("trace-1")
                    .id("observation-1")
                    .start_time(start)
                    .call()
                    .await
                    .unwrap();
            }
            "span-update" => {
                client
                    .update_span()
                    .trace_id("trace-1")
                    .id("observation-1")
                    .start_time(start)
                    .end_time(end)
                    .call()
                    .await
                    .unwrap();
            }
            "generation-update" => {
                client
                    .update_generation()
                    .trace_id("trace-1")
                    .id("observation-1")
                    .start_time(start)
                    .end_time(end)
                    .completion_start_time(completion)
                    .call()
                    .await
                    .unwrap();
            }
            _ => unreachable!(),
        }
        mock.assert_async().await;
    }
}

#[tokio::test]
async fn scores_preserve_numeric_and_categorical_values() {
    let mut server = Server::new_async().await;
    let client = client(&server);
    for (value, data_type) in [(json!(0.95), "NUMERIC"), (json!("positive"), "CATEGORICAL")] {
        let mock = server
            .mock("POST", "/api/public/ingestion")
            .match_body(Matcher::PartialJson(
                json!({"batch": [{"type": "score-create", "body": {
                    "traceId": "trace-1", "name": "quality", "value": value, "dataType": data_type
                }}]}),
            ))
            .with_status(207)
            .with_header("content-type", "application/json")
            .with_body(r#"{"successes": [], "errors": []}"#)
            .create_async()
            .await;
        if data_type == "NUMERIC" {
            client
                .score()
                .trace_id("trace-1")
                .name("quality")
                .value(0.95)
                .call()
                .await
                .unwrap();
        } else {
            client
                .categorical_score("trace-1", "quality", "positive")
                .await
                .unwrap();
        }
        mock.assert_async().await;
    }
}

#[tokio::test]
async fn single_observation_decodes_typed_dates_and_usage() {
    let mut server = Server::new_async().await;
    let mut response = json!({
        "id": "generation-1", "traceId": "trace-1", "type": "GENERATION",
        "startTime": "2026-10-03T14:34:56.123+02:00", "metadata": {"source": "test"},
        "usage": {"input": 1200, "output": 340, "total": 1540, "unit": "TOKENS"}, "level": "DEFAULT",
        "usageDetails": {"input": 1200, "output": 340, "total": 1540},
        "costDetails": {"total": 0.01}, "environment": "default"
    });
    for field in [
        "name",
        "endTime",
        "completionStartTime",
        "model",
        "modelParameters",
        "input",
        "version",
        "output",
        "statusMessage",
        "parentObservationId",
        "promptId",
        "promptName",
        "promptVersion",
        "modelId",
        "inputPrice",
        "outputPrice",
        "totalPrice",
        "calculatedInputCost",
        "calculatedOutputCost",
        "calculatedTotalCost",
        "latency",
        "timeToFirstToken",
    ] {
        response[field] = serde_json::Value::Null;
    }
    let mock = server
        .mock("GET", "/api/public/observations/generation-1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response.to_string())
        .create_async()
        .await;
    let observation: ObservationsViewSingle = client(&server)
        .get_observation("generation-1")
        .await
        .unwrap();
    mock.assert_async().await;
    assert_eq!(observation.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(
        observation.start_time.with_timezone(&Utc),
        "2026-10-03T12:34:56.123Z".parse::<DateTime<Utc>>().unwrap()
    );
    assert_eq!(observation.usage_details["total"], 1540);
    assert_eq!(observation.usage.total, 1540);
    assert_eq!(observation.cost_details["total"], 0.01);
    assert_eq!(observation.metadata["source"], "test");
}

#[tokio::test]
async fn trace_filters_preserve_rfc3339_offsets_and_reject_invalid_dates() {
    let mut server = Server::new_async().await;
    let client = client(&server);
    let from = "2026-10-03T14:34:56+02:00";
    let to = "2026-10-03T14:35:56+02:00";
    let mock = server
        .mock("GET", "/api/public/traces")
        .match_header("authorization", "Basic cGstbGYtdGVzdDpzay1sZi10ZXN0")
        .match_header("user-agent", Matcher::Regex("^langfuse-ergonomic/".into()))
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("fromTimestamp".into(), from.into()),
            Matcher::UrlEncoded("toTimestamp".into(), to.into()),
            Matcher::UrlEncoded("page".into(), "2".into()),
            Matcher::UrlEncoded("limit".into(), "10".into()),
            Matcher::UrlEncoded("userId".into(), "user-1".into()),
            Matcher::UrlEncoded("name".into(), "name-1".into()),
            Matcher::UrlEncoded("sessionId".into(), "session-1".into()),
            Matcher::UrlEncoded("version".into(), "v1".into()),
            Matcher::UrlEncoded("release".into(), "release-1".into()),
            Matcher::UrlEncoded("orderBy".into(), "timestamp.asc".into()),
            Matcher::UrlEncoded("tags".into(), "test".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"data":[],"meta":{"page":1,"limit":10,"totalItems":0,"totalPages":0}}"#)
        .create_async()
        .await;
    client
        .list_traces()
        .page(2)
        .limit(10)
        .user_id("user-1")
        .name("name-1")
        .session_id("session-1")
        .version("v1")
        .release("release-1")
        .order_by("timestamp.asc")
        .tags("test")
        .from_timestamp(from)
        .to_timestamp(to)
        .call()
        .await
        .unwrap();
    mock.assert_async().await;
    let invalid = server
        .mock("GET", "/api/public/traces")
        .expect(0)
        .create_async()
        .await;
    assert!(matches!(
        client.list_traces().from_timestamp("invalid").call().await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client.list_traces().to_timestamp("invalid").call().await,
        Err(Error::Validation(_))
    ));
    invalid.assert_async().await;

    let denied = server
        .mock("GET", "/api/public/traces")
        .with_status(401)
        .with_body("Unauthorized")
        .create_async()
        .await;
    let error = client.list_traces().call().await.unwrap_err();
    assert!(error.to_string().contains("401"));
    assert!(error.to_string().contains("Unauthorized"));
    denied.assert_async().await;
}

#[tokio::test]
async fn dataset_schemas_preserve_objects_and_reject_non_objects() {
    let mut server = Server::new_async().await;
    let client = client(&server);
    let schema = json!({"type": "object", "properties": {"query": {"type": "string"}}});
    let mock = server
        .mock("POST", "/api/public/v2/datasets")
        .match_body(Matcher::PartialJson(
            json!({"inputSchema": schema, "expectedOutputSchema": schema}),
        ))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"dataset-1","name":"test","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"project-1","createdAt":"2026-10-03T12:00:00Z","updatedAt":"2026-10-03T12:00:00Z"}"#)
        .create_async()
        .await;
    client
        .create_dataset()
        .name("test")
        .input_schema(schema.clone())
        .expected_output_schema(schema)
        .call()
        .await
        .unwrap();
    mock.assert_async().await;
    let invalid = server
        .mock("POST", "/api/public/v2/datasets")
        .expect(0)
        .create_async()
        .await;
    let nullable = server.mock("POST", "/api/public/v2/datasets")
        .match_body(Matcher::PartialJson(json!({"inputSchema": null, "expectedOutputSchema": null})))
        .with_status(201).with_header("content-type", "application/json")
        .with_body(r#"{"id":"dataset-1","name":"test","description":null,"metadata":null,"inputSchema":null,"expectedOutputSchema":null,"projectId":"project-1","createdAt":"2026-10-03T12:00:00Z","updatedAt":"2026-10-03T12:00:00Z"}"#)
        .create_async().await;
    client
        .create_dataset()
        .name("test")
        .input_schema(json!(null))
        .expected_output_schema(json!(null))
        .call()
        .await
        .unwrap();
    nullable.assert_async().await;
    assert!(matches!(
        client
            .create_dataset()
            .name("test")
            .input_schema(json!(true))
            .call()
            .await,
        Err(Error::Validation(_))
    ));
    assert!(matches!(
        client
            .create_dataset()
            .name("test")
            .expected_output_schema(json!([]))
            .call()
            .await,
        Err(Error::Validation(_))
    ));
    invalid.assert_async().await;
}
