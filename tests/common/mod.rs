//! Helpers for the live tests, which run against a real Langfuse server.
//!
//! The API version comes from `LANGFUSE_API_VERSION` (`v3` or `v4`, default `v4`), so the same
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
pub fn live_client() -> LangfuseClient {
    let client = ClientBuilder::from_env()
        .and_then(|builder| builder.build())
        .expect("configure LANGFUSE_PUBLIC_KEY, LANGFUSE_SECRET_KEY and LANGFUSE_BASE_URL");
    eprintln!("live tests use Langfuse API {}", client.api_version());
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
