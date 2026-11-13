//! A mock Langfuse that records what the client sends (v3 ingestion events, v4 OTLP spans and
//! scores), for the observe, experiment and media tests.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use langfuse_ergonomic::{ApiVersion, Batcher, ClientBuilder, LangfuseClient};
use serde_json::Value;

/// Serialises the tests that install the process-wide observe sink.
pub static GLOBAL_SINK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct Recorder {
    pub server: mockito::ServerGuard,
    bodies: Arc<Mutex<Vec<(String, Value)>>>,
    _mocks: Vec<mockito::Mock>,
}

impl Recorder {
    pub async fn start() -> Self {
        let mut server = mockito::Server::new_async().await;
        let bodies: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
        let mut mocks = Vec::new();
        for (path, status, response) in [
            (
                "/api/public/ingestion",
                207,
                r#"{"successes":[],"errors":[]}"#,
            ),
            ("/api/public/otel/v1/traces", 200, "{}"),
            ("/api/public/scores", 200, r#"{"id":"score"}"#),
        ] {
            let sink = bodies.clone();
            let mock = server
                .mock("POST", path)
                .match_request(move |request| {
                    let body = request.utf8_lossy_body().unwrap().to_string();
                    let value = serde_json::from_str(&body).unwrap_or(Value::String(body));
                    sink.lock()
                        .unwrap()
                        .push((request.path().to_owned(), value));
                    true
                })
                .with_status(status)
                .with_body(response)
                .expect_at_least(0)
                .create_async()
                .await;
            mocks.push(mock);
        }
        Self {
            server,
            bodies,
            _mocks: mocks,
        }
    }

    pub fn client(&self, api_version: ApiVersion) -> LangfuseClient {
        ClientBuilder::new()
            .public_key("pk-test")
            .secret_key("sk-test")
            .base_url(self.server.url())
            .api_version(api_version)
            .build()
            .unwrap()
    }

    pub async fn batcher(&self, client: LangfuseClient) -> Batcher {
        Batcher::builder()
            .client(client)
            .flush_interval(Duration::from_secs(3600))
            .max_retries(0)
            .build()
            .await
    }

    /// Every request body sent to `path`.
    pub fn bodies(&self, path: &str) -> Vec<Value> {
        self.bodies
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, _)| p == path)
            .map(|(_, body)| body.clone())
            .collect()
    }

    /// Every v3 ingestion event sent so far.
    pub fn v3_events(&self) -> Vec<Value> {
        self.bodies("/api/public/ingestion")
            .into_iter()
            .flat_map(|body| body["batch"].as_array().cloned().unwrap_or_default())
            .collect()
    }

    /// The v3 observation create events (not traces or scores), as their bodies with a `kind`.
    pub fn v3_observations(&self) -> Vec<Value> {
        self.v3_events()
            .into_iter()
            .filter(|event| {
                let kind = event["type"].as_str().unwrap_or_default();
                kind.ends_with("-create") && kind != "trace-create" && kind != "score-create"
            })
            .map(|event| {
                let mut body = event["body"].clone();
                body["kind"] = event["type"].clone();
                body
            })
            .collect()
    }

    pub fn v3_traces(&self) -> Vec<Value> {
        self.v3_events()
            .into_iter()
            .filter(|event| event["type"] == "trace-create")
            .map(|event| event["body"].clone())
            .collect()
    }

    pub fn v3_scores(&self) -> Vec<Value> {
        self.v3_events()
            .into_iter()
            .filter(|event| event["type"] == "score-create")
            .map(|event| event["body"].clone())
            .collect()
    }

    pub fn v3_observation(&self, name: &str) -> Value {
        self.v3_observations()
            .into_iter()
            .find(|o| o["name"] == name)
            .unwrap_or_else(|| panic!("no observation {name}: {:#?}", self.v3_events()))
    }

    /// Every v4 span exported so far.
    pub fn v4_spans(&self) -> Vec<Value> {
        self.bodies("/api/public/otel/v1/traces")
            .into_iter()
            .flat_map(|body| {
                body["resourceSpans"][0]["scopeSpans"][0]["spans"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
            })
            .collect()
    }

    pub fn v4_span(&self, name: &str) -> Value {
        self.v4_spans()
            .into_iter()
            .rev()
            .find(|s| s["name"] == name)
            .unwrap_or_else(|| panic!("no span {name}: {:#?}", self.v4_spans()))
    }
}

/// A string attribute of an OTLP span.
pub fn attr(span: &Value, key: &str) -> Option<Value> {
    span["attributes"]
        .as_array()?
        .iter()
        .find(|a| a["key"] == key)
        .map(|a| {
            let value = &a["value"];
            value
                .get("stringValue")
                .or_else(|| value.get("intValue"))
                .or_else(|| value.get("boolValue"))
                .cloned()
                .unwrap_or_else(|| value.clone())
        })
}
