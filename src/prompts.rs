//! Prompt management functionality
//!
//! This module contains types and utilities for prompt management.
//! The client methods live here as a `#[bon] impl` block on [`LangfuseClient`].

// Re-export common types that might be useful
// Note: CreatePromptRequest might be an enum or different structure
// pub use langfuse_client_base::models::CreatePromptRequest;

use bon::bon;
use serde_json::Value;

use crate::client::LangfuseClient;
use crate::error::{Error, Result};

#[bon]
impl LangfuseClient {
    // ===== PROMPT MANAGEMENT =====

    /// Create a new prompt or a new version of an existing prompt
    #[builder]
    pub async fn create_prompt(
        &self,
        #[builder(into)] name: String,
        #[builder(into)] prompt: String,
        _is_active: Option<bool>,
        config: Option<Value>,
        labels: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompts_api;
        use langfuse_client_base::models::{CreatePromptRequest, CreateTextPromptRequest};

        let prompt_request =
            CreatePromptRequest::CreateTextPromptRequest(Box::new(CreateTextPromptRequest {
                name: name.clone(),
                prompt,
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..Default::default()
            }));

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create prompt: {}", e)))
    }

    /// Create a chat prompt with messages
    ///
    /// Invalid messages return a validation error with their zero-based index
    /// before any HTTP request is sent.
    #[builder]
    pub async fn create_chat_prompt(
        &self,
        #[builder(into)] name: String,
        messages: Vec<serde_json::Value>, // Array of chat messages as JSON
        config: Option<Value>,
        labels: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompts_api;
        use langfuse_client_base::models::{
            ChatMessageWithPlaceholders, CreateChatPromptRequest, CreatePromptRequest,
        };

        let chat_messages: Vec<ChatMessageWithPlaceholders> = messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                serde_json::from_value(message).map_err(|_| {
                    // Deserialization errors can contain caller content.
                    Error::Validation(format!(
                        "Invalid chat prompt message at index {index}: expected a chat message or placeholder"
                    ))
                })
            })
            .collect::<Result<_>>()?;

        let prompt_request =
            CreatePromptRequest::CreateChatPromptRequest(Box::new(CreateChatPromptRequest {
                name: name.clone(),
                prompt: chat_messages,
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..Default::default()
            }));

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create chat prompt: {}", e)))
    }

    /// Update labels for a specific prompt version
    #[builder]
    pub async fn update_prompt_version(
        &self,
        #[builder(into)] name: String,
        version: i32,
        labels: Vec<String>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompt_version_api;
        use langfuse_client_base::models::PromptVersionUpdateRequest;

        let update_request = PromptVersionUpdateRequest { new_labels: labels };

        prompt_version_api::prompt_version_update()
            .configuration(self.configuration())
            .name(name.as_str())
            .version(version)
            .prompt_version_update_request(update_request)
            .call()
            .await
            .map_err(|e| {
                crate::error::Error::Api(format!("Failed to update prompt version: {}", e))
            })
    }

    /// Get a prompt by name and version
    pub async fn get_prompt(
        &self,
        prompt_name: impl Into<String>,
        version: Option<i32>,
        label: Option<&str>,
    ) -> Result<langfuse_client_base::models::Prompt> {
        use langfuse_client_base::apis::prompts_api;

        let prompt_name = prompt_name.into();

        prompts_api::prompts_get()
            .configuration(self.configuration())
            .prompt_name(prompt_name.as_str())
            .maybe_version(version)
            .maybe_label(label)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get prompt: {}", e)))
    }

    /// List prompts with filters
    #[builder]
    pub async fn list_prompts(
        &self,
        #[builder(into)] name: Option<String>,
        #[builder(into)] tag: Option<String>,
        #[builder(into)] label: Option<String>,
        page: Option<i32>,
        limit: Option<String>,
    ) -> Result<langfuse_client_base::models::PromptMetaListResponse> {
        use langfuse_client_base::apis::prompts_api;

        let name_ref = name.as_deref();
        let tag_ref = tag.as_deref();
        let label_ref = label.as_deref();
        let limit_num = limit.and_then(|value| value.parse::<i32>().ok());

        prompts_api::prompts_list()
            .configuration(self.configuration())
            .maybe_name(name_ref)
            .maybe_tag(tag_ref)
            .maybe_label(label_ref)
            .maybe_page(page)
            .maybe_limit(limit_num)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list prompts: {}", e)))
    }
}
