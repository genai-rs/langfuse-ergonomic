//! Helpers for the live tests, which run against a real Langfuse server.
//!
//! The API version comes from `LANGFUSE_API_VERSION` (`v3` or `v4`, required), so the same
//! tests cover both write and read paths. `scripts/live-tests.sh` starts a self-hosted
//! Langfuse v4 that serves both (see `docker/langfuse/compose.yml`).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// The v3 read methods are deprecated; the helpers use them only in V3 mode.
#![allow(deprecated)]

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use langfuse_ergonomic::{ApiVersion, ClientBuilder, LangfuseClient};
use tokio::time::{sleep, timeout, Instant};

/// How long a write may take to become readable (Langfuse ingests asynchronously).
const READBACK_TIMEOUT: Duration = Duration::from_secs(120);
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A client from `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, `LANGFUSE_BASE_URL` and
/// `LANGFUSE_API_VERSION`.
///
/// `LANGFUSE_API_VERSION` must be set: a missing or mistyped variable would otherwise run the
/// default version and silently skip the other one.
pub fn live_client() -> LangfuseClient {
    let expected: ApiVersion = std::env::var(ApiVersion::ENV_VAR)
        .expect("set LANGFUSE_API_VERSION to v3 or v4 for the live tests")
        .parse()
        .expect("LANGFUSE_API_VERSION must be v3 or v4");
    let client = ClientBuilder::from_env()
        .and_then(|builder| builder.build())
        .expect("configure LANGFUSE_PUBLIC_KEY, LANGFUSE_SECRET_KEY and LANGFUSE_BASE_URL");
    assert_eq!(
        client.api_version(),
        expected,
        "the client does not use LANGFUSE_API_VERSION"
    );
    eprintln!("live tests use Langfuse API {expected}");
    client
}

/// Poll `attempt` until it returns `Ok`, failing the test with the last diagnostic after
/// [`READBACK_TIMEOUT`].
pub async fn eventually<T, F, Fut>(what: &str, mut attempt: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let deadline = Instant::now() + READBACK_TIMEOUT;
    loop {
        let diagnostic = match timeout(REQUEST_TIMEOUT, attempt()).await {
            Ok(Ok(value)) => return value,
            Ok(Err(diagnostic)) => diagnostic,
            Err(_) => "request timed out".to_owned(),
        };
        assert!(
            Instant::now() < deadline,
            "{what} not readable after {READBACK_TIMEOUT:?}: {diagnostic}"
        );
        sleep(POLL_INTERVAL).await;
    }
}

/// The fields the live tests check, from either the v3 or the v4 read API.
#[derive(Debug, Clone)]
pub struct StoredObservation {
    pub id: String,
    pub trace_id: Option<String>,
    pub r#type: String,
    pub name: Option<String>,
    pub parent_observation_id: Option<String>,
    pub usage_details: HashMap<String, i64>,
}

/// Read one observation: `GET /api/public/observations/{id}` (V3) or the trace's
/// observations from `GET /api/public/v2/observations` (V4).
///
/// `trace_id` and `observation_id` are the ids the client returned.
pub async fn read_observation(
    client: &LangfuseClient,
    trace_id: &str,
    observation_id: &str,
) -> Result<StoredObservation, String> {
    match client.api_version() {
        ApiVersion::V3 => {
            let observation = client
                .get_observation(observation_id)
                .await
                .map_err(|e| format!("readback error: {e}"))?;
            Ok(StoredObservation {
                id: observation.id,
                trace_id: observation.trace_id,
                r#type: observation.r#type,
                name: observation.name,
                parent_observation_id: observation.parent_observation_id,
                usage_details: observation.usage_details,
            })
        }
        ApiVersion::V4 => {
            let observations = client
                .trace_observations(trace_id)
                .await
                .map_err(|e| format!("readback error: {e}"))?;
            let observation = observations
                .into_iter()
                .find(|o| o.id == observation_id)
                .ok_or_else(|| format!("observation {observation_id} not in trace {trace_id}"))?;
            Ok(StoredObservation {
                id: observation.id,
                trace_id: observation.trace_id,
                r#type: observation.r#type,
                name: observation.name.flatten(),
                parent_observation_id: observation.parent_observation_id,
                usage_details: observation.usage_details.flatten().unwrap_or_default(),
            })
        }
    }
}

/// Wait until the observation is readable and `check` accepts it.
pub async fn wait_for_observation(
    client: &LangfuseClient,
    trace_id: &str,
    observation_id: &str,
    check: impl Fn(&StoredObservation) -> Result<(), String>,
) -> StoredObservation {
    eventually(&format!("observation {observation_id}"), || async {
        let observation = read_observation(client, trace_id, observation_id).await?;
        check(&observation).map_err(|e| format!("{e}; stored: {observation:?}"))?;
        Ok(observation)
    })
    .await
}

/// The trace-level fields the live tests check.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredTrace {
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub tags: Vec<String>,
}

/// Wait until the trace is readable: `GET /api/public/traces/{id}` (V3) or the trace's root
/// observation from `GET /api/public/v2/observations` (V4).
pub async fn wait_for_trace(client: &LangfuseClient, trace_id: &str) -> StoredTrace {
    eventually(&format!("trace {trace_id}"), || async {
        match client.api_version() {
            ApiVersion::V3 => {
                let trace = client
                    .get_trace(trace_id)
                    .await
                    .map_err(|e| format!("readback error: {e}"))?;
                Ok(StoredTrace {
                    name: trace.name,
                    user_id: trace.user_id,
                    session_id: trace.session_id,
                    tags: trace.tags,
                })
            }
            ApiVersion::V4 => {
                let observations = client
                    .trace_observations(trace_id)
                    .await
                    .map_err(|e| format!("readback error: {e}"))?;
                let root = observations
                    .into_iter()
                    .find(|o| o.is_root_observation.flatten() == Some(true))
                    .ok_or_else(|| format!("no root observation for trace {trace_id} yet"))?;
                Ok(StoredTrace {
                    name: root.trace_name.flatten().or(root.name.flatten()),
                    user_id: root.user_id.flatten(),
                    session_id: root.session_id.flatten(),
                    tags: root.tags.flatten().unwrap_or_default(),
                })
            }
        }
    })
    .await
}

/// Wait until the score is readable and return it: `GET /api/public/v2/scores/{id}`.
///
/// The crate has no score reads, so this is a raw request. A self-hosted Langfuse v4 serves
/// the endpoint in every write mode, and it returns scores written with either API version.
pub async fn wait_for_score(client: &LangfuseClient, score_id: &str) -> serde_json::Value {
    let configuration = client.configuration();
    let url = format!(
        "{}/api/public/v2/scores/{score_id}",
        configuration.base_path.trim_end_matches('/')
    );
    eventually(&format!("score {score_id}"), || async {
        let mut request = configuration
            .client
            .request(timvw_rust_runtime::Method::GET, &url);
        if let Some((user, password)) = &configuration.basic_auth {
            request = request.basic_auth(
                user,
                password
                    .as_ref()
                    .map(timvw_rust_runtime::ExposeSecret::expose_secret),
            );
        }
        let response = request
            .send()
            .await
            .map_err(|e| format!("readback error: {e}"))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| format!("readback error: {e}"))?;
        if !status.is_success() {
            return Err(format!("status {status}: {body}"));
        }
        serde_json::from_str(&body).map_err(|e| format!("invalid score JSON: {e}: {body}"))
    })
    .await
}
