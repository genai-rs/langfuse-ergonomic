//! The wire format of the Langfuse v3 batch ingestion API (`POST /api/public/ingestion`).
//!
//! The legacy `observation-create` and `observation-update` events only accept the `SPAN`,
//! `GENERATION` and `EVENT` types: Langfuse rejects any other type with a per-event 400 in its
//! 207 response. Since Langfuse v3.99.0 the ingestion API takes the agent, tool, chain,
//! retriever, evaluator, embedding and guardrail types as their own events instead
//! (`agent-create`, `tool-create`, ...), with a generation-shaped body. These are not in the
//! public OpenAPI spec (so not in [`IngestionEvent`]), so such events are rewritten here, when the
//! batch is serialized.

use langfuse_client_base::models::{IngestionEvent, ObservationType};
use serde_json::Value;

use crate::client::LangfuseClient;
use crate::error::Result;

/// The `<type>-create` event type Langfuse takes for an observation type that the legacy
/// `observation-*` events reject.
fn typed_create_event(kind: ObservationType) -> Option<&'static str> {
    match kind {
        ObservationType::Span | ObservationType::Generation | ObservationType::Event => None,
        ObservationType::Agent => Some("agent-create"),
        ObservationType::Tool => Some("tool-create"),
        ObservationType::Chain => Some("chain-create"),
        ObservationType::Retriever => Some("retriever-create"),
        ObservationType::Evaluator => Some("evaluator-create"),
        ObservationType::Embedding => Some("embedding-create"),
        ObservationType::Guardrail => Some("guardrail-create"),
    }
}

/// One event as the v3 ingestion API takes it.
///
/// An `observation-create` or `observation-update` of a type other than span, generation or
/// event becomes that type's `<type>-create` event without the `type` body field (Langfuse
/// merges the events of one observation id, so an update is another create).
pub(crate) fn wire_event(event: &IngestionEvent) -> Result<Value> {
    let kind = match event {
        IngestionEvent::CreateObservationEvent(e) => Some(e.body.r#type),
        IngestionEvent::UpdateObservationEvent(e) => Some(e.body.r#type),
        _ => None,
    };
    let mut value = serde_json::to_value(event)?;
    if let Some(event_type) = kind.and_then(typed_create_event) {
        value["type"] = Value::from(event_type);
        if let Some(body) = value.get_mut("body").and_then(Value::as_object_mut) {
            body.remove("type");
        }
    }
    Ok(value)
}

/// The JSON body of a v3 ingestion request.
pub(crate) fn wire_batch<'a>(
    events: impl IntoIterator<Item = &'a IngestionEvent>,
) -> Result<Value> {
    let batch = events
        .into_iter()
        .map(wire_event)
        .collect::<Result<Vec<_>>>()?;
    Ok(serde_json::json!({ "batch": batch }))
}

impl LangfuseClient {
    /// `POST /api/public/ingestion` with the events in their v3 wire format.
    pub(crate) async fn post_ingestion_batch(
        &self,
        events: &[IngestionEvent],
    ) -> Result<timvw_rust_runtime::Response> {
        let body = serde_json::to_vec(&wire_batch(events)?)?;
        let mut request = self
            .configuration
            .client
            .request(
                timvw_rust_runtime::Method::POST,
                &format!("{}/api/public/ingestion", self.configuration.base_path),
            )
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .header(timvw_rust_runtime::header::CONTENT_TYPE, "application/json");
        if let Some(user_agent) = &self.configuration.user_agent {
            request = request.header(timvw_rust_runtime::header::USER_AGENT, user_agent);
        }
        Ok(request.body(body).send().await?)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use langfuse_client_base::models::{
        ingestion_event_create_observation_event, ingestion_event_update_observation_event,
        IngestionEventCreateObservationEvent, IngestionEventUpdateObservationEvent,
        ObservationBody,
    };

    fn body(kind: ObservationType) -> ObservationBody {
        ObservationBody {
            id: Some(Some("obs-1".into())),
            trace_id: Some(Some("trace-1".into())),
            r#type: kind,
            name: Some(Some("name".into())),
            start_time: None,
            end_time: None,
            completion_start_time: None,
            model: None,
            model_parameters: None,
            input: None,
            version: None,
            metadata: None,
            output: None,
            usage: None,
            level: None,
            status_message: None,
            parent_observation_id: Some(Some("parent-1".into())),
            environment: None,
        }
    }

    fn create(kind: ObservationType) -> IngestionEvent {
        IngestionEvent::CreateObservationEvent(IngestionEventCreateObservationEvent::new(
            "event-1".into(),
            "2026-01-01T00:00:00.000Z".into(),
            body(kind),
            ingestion_event_create_observation_event::Type::ObservationCreate,
        ))
    }

    #[test]
    fn new_observation_types_become_their_own_create_events() {
        for (kind, event_type) in [
            (ObservationType::Agent, "agent-create"),
            (ObservationType::Tool, "tool-create"),
            (ObservationType::Chain, "chain-create"),
            (ObservationType::Retriever, "retriever-create"),
            (ObservationType::Evaluator, "evaluator-create"),
            (ObservationType::Embedding, "embedding-create"),
            (ObservationType::Guardrail, "guardrail-create"),
        ] {
            let wire = wire_event(&create(kind)).unwrap();
            assert_eq!(wire["type"], event_type);
            assert_eq!(wire["id"], "event-1");
            assert!(wire["body"].get("type").is_none(), "{wire}");
            assert_eq!(wire["body"]["id"], "obs-1");
            assert_eq!(wire["body"]["parentObservationId"], "parent-1");
        }
    }

    #[test]
    fn legacy_types_and_updates_keep_their_events() {
        for kind in [
            ObservationType::Span,
            ObservationType::Generation,
            ObservationType::Event,
        ] {
            let event = create(kind);
            assert_eq!(
                wire_event(&event).unwrap(),
                serde_json::to_value(&event).unwrap()
            );
        }
        let update =
            IngestionEvent::UpdateObservationEvent(IngestionEventUpdateObservationEvent::new(
                "event-2".into(),
                "2026-01-01T00:00:00.000Z".into(),
                body(ObservationType::Tool),
                ingestion_event_update_observation_event::Type::ObservationUpdate,
            ));
        let wire = wire_event(&update).unwrap();
        assert_eq!(wire["type"], "tool-create");
        assert!(wire["body"].get("type").is_none());
    }
}
