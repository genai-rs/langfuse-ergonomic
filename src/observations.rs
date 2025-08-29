//! Observation-related functionality (spans, generations, and events)

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::client::LangfuseClient;
use crate::error::Result;

/// Type of observation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationType {
    Span,
    Generation,
    Event,
}

/// Builder for creating observations (spans and generations)
pub struct ObservationBuilder<'a> {
    #[allow(dead_code)]
    client: &'a LangfuseClient,
    id: Option<String>,
    #[allow(dead_code)]
    trace_id: String,
    parent_observation_id: Option<String>,
    observation_type: ObservationType,
    name: Option<String>,
    input: Option<Value>,
    output: Option<Value>,
    metadata: Option<Value>,
    level: Option<String>,
    status_message: Option<String>,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    // Generation-specific fields
    model: Option<String>,
    model_parameters: Option<Value>,
    prompt_tokens: Option<i32>,
    completion_tokens: Option<i32>,
    total_tokens: Option<i32>,
}

impl LangfuseClient {
    /// Start building a span observation
    pub fn span(&self, trace_id: impl Into<String>) -> ObservationBuilder<'_> {
        ObservationBuilder {
            client: self,
            id: None,
            trace_id: trace_id.into(),
            parent_observation_id: None,
            observation_type: ObservationType::Span,
            name: None,
            input: None,
            output: None,
            metadata: None,
            level: None,
            status_message: None,
            start_time: None,
            end_time: None,
            model: None,
            model_parameters: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        }
    }

    /// Start building a generation observation
    pub fn generation(&self, trace_id: impl Into<String>) -> ObservationBuilder<'_> {
        ObservationBuilder {
            client: self,
            id: None,
            trace_id: trace_id.into(),
            parent_observation_id: None,
            observation_type: ObservationType::Generation,
            name: None,
            input: None,
            output: None,
            metadata: None,
            level: None,
            status_message: None,
            start_time: None,
            end_time: None,
            model: None,
            model_parameters: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        }
    }

    /// Start building an event observation
    pub fn event(&self, trace_id: impl Into<String>) -> ObservationBuilder<'_> {
        ObservationBuilder {
            client: self,
            id: None,
            trace_id: trace_id.into(),
            parent_observation_id: None,
            observation_type: ObservationType::Event,
            name: None,
            input: None,
            output: None,
            metadata: None,
            level: None,
            status_message: None,
            start_time: None,
            end_time: None,
            model: None,
            model_parameters: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
        }
    }
}

impl<'a> ObservationBuilder<'a> {
    /// Set the observation ID (if not provided, one will be generated)
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the parent observation ID for nested observations
    pub fn parent_observation_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_observation_id = Some(parent_id.into());
        self
    }

    /// Set the observation name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the input data
    pub fn input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }

    /// Set the output data
    pub fn output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set the log level (e.g., "INFO", "WARNING", "ERROR")
    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = Some(level.into());
        self
    }

    /// Set a status message
    pub fn status_message(mut self, message: impl Into<String>) -> Self {
        self.status_message = Some(message.into());
        self
    }

    /// Set the start time
    pub fn start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = Some(start_time);
        self
    }

    /// Set the end time
    pub fn end_time(mut self, end_time: DateTime<Utc>) -> Self {
        self.end_time = Some(end_time);
        self
    }

    // Generation-specific methods

    /// Set the model name (for generations)
    pub fn model(mut self, model: impl Into<String>) -> Self {
        if self.observation_type == ObservationType::Generation {
            self.model = Some(model.into());
        }
        self
    }

    /// Set model parameters (for generations)
    pub fn model_parameters(mut self, params: Value) -> Self {
        if self.observation_type == ObservationType::Generation {
            self.model_parameters = Some(params);
        }
        self
    }

    /// Set token counts (for generations)
    pub fn tokens(mut self, prompt: i32, completion: i32) -> Self {
        if self.observation_type == ObservationType::Generation {
            self.prompt_tokens = Some(prompt);
            self.completion_tokens = Some(completion);
            self.total_tokens = Some(prompt + completion);
        }
        self
    }

    /// Execute the observation creation
    pub async fn send(self) -> Result<String> {
        use langfuse_client_base::apis::ingestion_api;
        use langfuse_client_base::models::{
            CreateEventBody, CreateGenerationBody, CreateSpanBody, IngestionBatchRequest,
            IngestionEvent, IngestionEventOneOf2, IngestionEventOneOf4, IngestionEventOneOf6,
            ObservationLevel,
        };

        let observation_id = self.id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let timestamp = self
            .start_time
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let event = match self.observation_type {
            ObservationType::Span => {
                let span_body = CreateSpanBody {
                    id: Some(Some(observation_id.clone())),
                    trace_id: Some(Some(self.trace_id.clone())),
                    name: self.name.map(Some),
                    start_time: Some(Some(timestamp.clone())),
                    end_time: self
                        .end_time
                        .map(|t| Some(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))),
                    input: self.input.map(Some),
                    output: self.output.map(Some),
                    level: self.level.map(|l| match l.as_str() {
                        "DEBUG" => ObservationLevel::Debug,
                        "DEFAULT" => ObservationLevel::Default,
                        "WARNING" => ObservationLevel::Warning,
                        "ERROR" => ObservationLevel::Error,
                        _ => ObservationLevel::Default,
                    }),
                    status_message: self.status_message.map(Some),
                    parent_observation_id: self.parent_observation_id.map(Some),
                    version: None,
                    metadata: self.metadata.map(Some),
                    environment: None,
                };

                let event = IngestionEventOneOf2 {
                    body: Box::new(span_body),
                    id: Uuid::new_v4().to_string(),
                    timestamp: timestamp.clone(),
                    metadata: None,
                    r#type:
                        langfuse_client_base::models::ingestion_event_one_of_2::Type::SpanCreate,
                };

                IngestionEvent::IngestionEventOneOf2(Box::new(event))
            }
            ObservationType::Generation => {
                let generation_body = CreateGenerationBody {
                    id: Some(Some(observation_id.clone())),
                    trace_id: Some(Some(self.trace_id.clone())),
                    name: self.name.map(Some),
                    start_time: Some(Some(timestamp.clone())),
                    completion_start_time: None,
                    end_time: self
                        .end_time
                        .map(|t| Some(t.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))),
                    model: self.model.map(Some),
                    model_parameters: None, // TODO: Convert JSON to HashMap if needed
                    input: self.input.map(Some),
                    output: self.output.map(Some),
                    usage: None, // TODO: Add usage tracking if needed
                    usage_details: None,
                    cost_details: None,
                    metadata: self.metadata.map(Some),
                    level: self.level.map(|l| match l.as_str() {
                        "DEBUG" => ObservationLevel::Debug,
                        "DEFAULT" => ObservationLevel::Default,
                        "WARNING" => ObservationLevel::Warning,
                        "ERROR" => ObservationLevel::Error,
                        _ => ObservationLevel::Default,
                    }),
                    status_message: self.status_message.map(Some),
                    parent_observation_id: self.parent_observation_id.map(Some),
                    version: None,
                    prompt_name: None,
                    prompt_version: None,
                    environment: None,
                };

                let event = IngestionEventOneOf4 {
                    body: Box::new(generation_body),
                    id: Uuid::new_v4().to_string(),
                    timestamp: timestamp.clone(),
                    metadata: None,
                    r#type: langfuse_client_base::models::ingestion_event_one_of_4::Type::GenerationCreate,
                };

                IngestionEvent::IngestionEventOneOf4(Box::new(event))
            }
            ObservationType::Event => {
                let event_body = CreateEventBody {
                    id: Some(Some(observation_id.clone())),
                    trace_id: Some(Some(self.trace_id.clone())),
                    name: self.name.map(Some),
                    start_time: Some(Some(timestamp.clone())),
                    input: self.input.map(Some),
                    output: self.output.map(Some),
                    level: self.level.map(|l| match l.as_str() {
                        "DEBUG" => ObservationLevel::Debug,
                        "DEFAULT" => ObservationLevel::Default,
                        "WARNING" => ObservationLevel::Warning,
                        "ERROR" => ObservationLevel::Error,
                        _ => ObservationLevel::Default,
                    }),
                    status_message: self.status_message.map(Some),
                    parent_observation_id: self.parent_observation_id.map(Some),
                    version: None,
                    metadata: self.metadata.map(Some),
                    environment: None,
                };

                let event = IngestionEventOneOf6 {
                    body: Box::new(event_body),
                    id: Uuid::new_v4().to_string(),
                    timestamp: timestamp.clone(),
                    metadata: None,
                    r#type:
                        langfuse_client_base::models::ingestion_event_one_of_6::Type::EventCreate,
                };

                IngestionEvent::IngestionEventOneOf6(Box::new(event))
            }
        };

        let batch_request = IngestionBatchRequest {
            batch: vec![event],
            metadata: None,
        };

        ingestion_api::ingestion_batch(self.client.configuration(), batch_request)
            .await
            .map(|_| observation_id)
            .map_err(|e| crate::error::Error::Api(format!("Failed to create observation: {}", e)))
    }
}
