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
            CreatePromptRequest::CreateTextPromptRequest(CreateTextPromptRequest {
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..CreateTextPromptRequest::new(name.clone(), prompt)
            });

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(&prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create prompt: {e}")))
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
            ChatMessageType, ChatMessageWithPlaceholders, CreateChatPromptRequest,
            CreateChatPromptType, CreatePromptRequest, PlaceholderMessageType,
        };

        let chat_messages: Vec<ChatMessageWithPlaceholders> = messages
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                let invalid = || {
                    // Deserialization errors can contain caller content.
                    Error::Validation(format!(
                        "Invalid chat prompt message at index {index}: expected a chat message or placeholder"
                    ))
                };
                let message: ChatMessageWithPlaceholders =
                    serde_json::from_value(message).map_err(|_| invalid())?;
                // The message types accept any `type` value, since a response
                // may carry one this crate does not know yet (`Unknown`). In a
                // request that is a caller error, not a value to send.
                let unknown_type = match &message {
                    ChatMessageWithPlaceholders::ChatMessage(m) => {
                        m.r#type == Some(ChatMessageType::Unknown)
                    }
                    ChatMessageWithPlaceholders::PlaceholderMessage(m) => {
                        m.r#type == Some(PlaceholderMessageType::Unknown)
                    }
                };
                if unknown_type {
                    return Err(invalid());
                }
                Ok(message)
            })
            .collect::<Result<_>>()?;

        let prompt_request =
            CreatePromptRequest::CreateChatPromptRequest(CreateChatPromptRequest {
                config: Some(config),
                labels: Some(labels),
                tags: Some(tags),
                ..CreateChatPromptRequest::new(
                    name.clone(),
                    chat_messages,
                    CreateChatPromptType::Chat,
                )
            });

        prompts_api::prompts_create()
            .configuration(self.configuration())
            .create_prompt_request(&prompt_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to create chat prompt: {e}")))
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
            .version(i64::from(version))
            .prompt_version_update_request(&update_request)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to update prompt version: {e}")))
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
            .maybe_version(version.map(i64::from))
            .maybe_label(label)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to get prompt: {e}")))
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
        let limit_num = limit.and_then(|value| value.parse::<i64>().ok());

        prompts_api::prompts_list()
            .configuration(self.configuration())
            .maybe_name(name_ref)
            .maybe_tag(tag_ref)
            .maybe_label(label_ref)
            .maybe_page(page.map(i64::from))
            .maybe_limit(limit_num)
            .call()
            .await
            .map_err(|e| crate::error::Error::Api(format!("Failed to list prompts: {e}")))
    }
}

// ===== CACHED FETCHING, FALLBACKS AND COMPILATION =====

use langfuse_client_base::models::Prompt;
use std::collections::HashMap;
use std::time::Duration;

/// How long [`LangfuseClient::prompt`] keeps a fetched prompt by default (60 seconds, like the
/// Langfuse Python and JS SDKs).
pub const DEFAULT_PROMPT_CACHE_TTL: Duration = Duration::from_secs(60);

/// The label Langfuse serves when neither a version nor a label is requested.
const DEFAULT_LABEL: &str = "production";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    name: String,
    version: Option<i32>,
    label: Option<String>,
}

impl CacheKey {
    fn new(name: &str, version: Option<i32>, label: Option<&str>) -> Self {
        // No version and no label means the `production` label: share its entry.
        let label = match (version, label) {
            (None, None) => Some(DEFAULT_LABEL.to_owned()),
            (_, label) => label.map(str::to_owned),
        };
        Self {
            name: name.to_owned(),
            version,
            label,
        }
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    prompt: Prompt,
    fetched_at: crate::time::Instant,
}

/// The prompt cache of a client (and its clones), keyed by name, version and label.
#[derive(Debug, Default)]
pub(crate) struct PromptCache {
    entries: std::sync::Mutex<HashMap<CacheKey, CacheEntry>>,
}

impl PromptCache {
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<CacheKey, CacheEntry>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn get(&self, key: &CacheKey) -> Option<CacheEntry> {
        self.lock().get(key).cloned()
    }

    fn put(&self, key: CacheKey, prompt: Prompt) {
        self.lock().insert(
            key,
            CacheEntry {
                prompt,
                fetched_at: crate::time::Instant::now(),
            },
        );
    }
}

/// Where a [`FetchedPrompt`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptSource {
    /// Fetched from Langfuse just now.
    Api,
    /// From the cache, within its time to live.
    Cache,
    /// From the cache after its time to live, because fetching it again failed.
    StaleCache,
    /// The caller-supplied fallback, because the prompt could not be fetched and was not cached.
    Fallback,
}

/// A prompt to use when Langfuse cannot be reached and nothing is cached
/// (see [`LangfuseClientPromptBuilder::fallback`]).
#[derive(Debug, Clone, PartialEq)]
pub enum PromptFallback {
    /// A text prompt.
    Text(String),
    /// A chat prompt: `(role, content)` pairs.
    Chat(Vec<(String, String)>),
}

impl PromptFallback {
    /// A text fallback.
    pub fn text(prompt: impl Into<String>) -> Self {
        Self::Text(prompt.into())
    }

    /// A chat fallback from `(role, content)` pairs.
    pub fn chat<R: Into<String>, C: Into<String>>(
        messages: impl IntoIterator<Item = (R, C)>,
    ) -> Self {
        Self::Chat(
            messages
                .into_iter()
                .map(|(role, content)| (role.into(), content.into()))
                .collect(),
        )
    }

    fn into_prompt(self, name: &str) -> Prompt {
        use langfuse_client_base::models::{
            prompt_chat_prompt, prompt_text_prompt, ChatMessage, ChatMessageWithPlaceholders,
            PromptChatPrompt, PromptTextPrompt,
        };
        match self {
            Self::Text(text) => Prompt::TextPrompt(PromptTextPrompt::new(
                name.to_owned(),
                0,
                None,
                Vec::new(),
                Vec::new(),
                text,
                prompt_text_prompt::Type::Text,
            )),
            Self::Chat(messages) => Prompt::ChatPrompt(PromptChatPrompt::new(
                name.to_owned(),
                0,
                None,
                Vec::new(),
                Vec::new(),
                messages
                    .into_iter()
                    .map(|(role, content)| {
                        ChatMessageWithPlaceholders::ChatMessage(ChatMessage {
                            role,
                            content,
                            r#type: None,
                        })
                    })
                    .collect(),
                prompt_chat_prompt::Type::Chat,
            )),
        }
    }
}

/// A prompt returned by [`LangfuseClient::prompt`], with where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct FetchedPrompt {
    /// The prompt (for a fallback: version 0, no labels).
    pub prompt: Prompt,
    /// Where it came from.
    pub source: PromptSource,
}

/// A compiled prompt: the template with its variables filled in.
#[derive(Debug, Clone, PartialEq)]
pub enum CompiledPrompt {
    /// A text prompt.
    Text(String),
    /// A chat prompt: messages as JSON objects (`{"role": .., "content": ..}`), ready for most chat
    /// APIs. Placeholders that were not filled stay as `{"type": "placeholder", "name": ..}`.
    Chat(Vec<Value>),
}

impl CompiledPrompt {
    /// The text of a text prompt.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Chat(_) => None,
        }
    }

    /// The messages of a chat prompt.
    pub fn as_messages(&self) -> Option<&[Value]> {
        match self {
            Self::Text(_) => None,
            Self::Chat(messages) => Some(messages),
        }
    }
}

impl FetchedPrompt {
    /// The prompt's name.
    pub fn name(&self) -> &str {
        match &self.prompt {
            Prompt::ChatPrompt(p) => &p.name,
            Prompt::TextPrompt(p) => &p.name,
            Prompt::Unknown(v) => v.get("name").and_then(Value::as_str).unwrap_or_default(),
        }
    }

    /// The prompt's version; `None` for a fallback (which must not be linked to generations).
    pub fn version(&self) -> Option<i64> {
        if self.is_fallback() {
            return None;
        }
        match &self.prompt {
            Prompt::ChatPrompt(p) => Some(p.version),
            Prompt::TextPrompt(p) => Some(p.version),
            Prompt::Unknown(v) => v.get("version").and_then(Value::as_i64),
        }
    }

    /// Whether this is the caller-supplied fallback.
    pub fn is_fallback(&self) -> bool {
        self.source == PromptSource::Fallback
    }

    /// The prompt's `config` (model parameters and the like), if any.
    pub fn config(&self) -> Option<&Value> {
        match &self.prompt {
            Prompt::ChatPrompt(p) => p.config.as_ref(),
            Prompt::TextPrompt(p) => p.config.as_ref(),
            Prompt::Unknown(v) => v.get("config"),
        }
        .filter(|config| !config.is_null())
    }

    /// Fill in `{{variable}}` placeholders (whitespace inside the braces is ignored). Variables
    /// that are not given stay as they are.
    ///
    /// ```
    /// use langfuse_ergonomic::prompts::{FetchedPrompt, PromptFallback, PromptSource};
    ///
    /// let prompt = FetchedPrompt::from_fallback("greeting", PromptFallback::text("Hello {{ name }}!"));
    /// let compiled = prompt.compile([("name", "Ada")]);
    /// assert_eq!(compiled.as_text(), Some("Hello Ada!"));
    /// ```
    pub fn compile<K, V>(&self, variables: impl IntoIterator<Item = (K, V)>) -> CompiledPrompt
    where
        K: Into<String>,
        V: ToString,
    {
        self.compile_with_placeholders(variables, Vec::<(String, Vec<Value>)>::new())
    }

    /// Like [`FetchedPrompt::compile`], and for a chat prompt also replace placeholder messages
    /// with the given messages (JSON objects such as `{"role": "user", "content": ".."}`).
    pub fn compile_with_placeholders<K, V, P>(
        &self,
        variables: impl IntoIterator<Item = (K, V)>,
        placeholders: impl IntoIterator<Item = (P, Vec<Value>)>,
    ) -> CompiledPrompt
    where
        K: Into<String>,
        V: ToString,
        P: Into<String>,
    {
        use langfuse_client_base::models::ChatMessageWithPlaceholders as M;

        let variables: HashMap<String, String> = variables
            .into_iter()
            .map(|(k, v)| (k.into(), v.to_string()))
            .collect();
        let placeholders: HashMap<String, Vec<Value>> = placeholders
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();
        match &self.prompt {
            Prompt::TextPrompt(p) => CompiledPrompt::Text(compile_template(&p.prompt, &variables)),
            Prompt::ChatPrompt(p) => {
                let mut messages = Vec::new();
                for message in &p.prompt {
                    match message {
                        M::ChatMessage(m) => messages.push(serde_json::json!({
                            "role": m.role,
                            "content": compile_template(&m.content, &variables),
                        })),
                        M::PlaceholderMessage(m) => match placeholders.get(&m.name) {
                            Some(filled) => messages.extend(filled.iter().cloned()),
                            None => messages
                                .push(serde_json::json!({ "type": "placeholder", "name": m.name })),
                        },
                    }
                }
                CompiledPrompt::Chat(messages)
            }
            Prompt::Unknown(v) => CompiledPrompt::Text(compile_template(
                v.get("prompt").and_then(Value::as_str).unwrap_or_default(),
                &variables,
            )),
        }
    }

    /// A [`FetchedPrompt`] made from a fallback (its source is [`PromptSource::Fallback`]).
    pub fn from_fallback(name: &str, fallback: PromptFallback) -> Self {
        Self {
            prompt: fallback.into_prompt(name),
            source: PromptSource::Fallback,
        }
    }
}

/// Replace `{{ name }}` with its value; unknown names and unterminated braces stay as they are.
pub fn compile_template(template: &str, variables: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = after[..end].trim();
        match variables.get(name) {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[start..start + 2 + end + 2]),
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

#[bon]
impl LangfuseClient {
    /// Fetch a prompt through the client's cache, with an optional fallback.
    ///
    /// - A cached prompt younger than `cache_ttl` (default [`DEFAULT_PROMPT_CACHE_TTL`]; zero
    ///   disables the cache) is returned without a request ([`PromptSource::Cache`]).
    /// - Otherwise it is fetched ([`PromptSource::Api`]) and cached. The cache is keyed by name,
    ///   version and label, and shared by the client's clones.
    /// - If fetching fails, a stale cached prompt is returned ([`PromptSource::StaleCache`],
    ///   with a warning), else the `fallback` ([`PromptSource::Fallback`]), else the error.
    ///
    /// The refresh happens in the call (no background task), so it works on every runtime and
    /// on WebAssembly.
    ///
    /// ```no_run
    /// # use langfuse_ergonomic::{ClientBuilder, prompts::PromptFallback};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = ClientBuilder::from_env()?.build()?;
    /// let prompt = client
    ///     .prompt("support-answer")
    ///     .label("production")
    ///     .fallback(PromptFallback::text("Answer the question: {{question}}"))
    ///     .call()
    ///     .await?;
    /// let text = prompt.compile([("question", "How do I reset my password?")]);
    /// # Ok(())
    /// # }
    /// ```
    #[builder]
    pub async fn prompt(
        &self,
        #[builder(start_fn, into)] name: String,
        version: Option<i32>,
        #[builder(into)] label: Option<String>,
        cache_ttl: Option<Duration>,
        fallback: Option<PromptFallback>,
    ) -> Result<FetchedPrompt> {
        let ttl = cache_ttl.unwrap_or(DEFAULT_PROMPT_CACHE_TTL);
        let key = CacheKey::new(&name, version, label.as_deref());
        let cached = if ttl.is_zero() {
            None
        } else {
            self.prompt_cache.get(&key)
        };
        if let Some(entry) = &cached {
            if entry.fetched_at.elapsed() < ttl {
                return Ok(FetchedPrompt {
                    prompt: entry.prompt.clone(),
                    source: PromptSource::Cache,
                });
            }
        }
        match self
            .get_prompt(name.as_str(), version, label.as_deref())
            .await
        {
            Ok(prompt) => {
                if !ttl.is_zero() {
                    self.prompt_cache.put(key, prompt.clone());
                }
                Ok(FetchedPrompt {
                    prompt,
                    source: PromptSource::Api,
                })
            }
            Err(error) => {
                if let Some(entry) = cached {
                    tracing::warn!(
                        prompt = name.as_str(),
                        %error,
                        "fetching the prompt failed; using the stale cached version"
                    );
                    return Ok(FetchedPrompt {
                        prompt: entry.prompt,
                        source: PromptSource::StaleCache,
                    });
                }
                match fallback {
                    Some(fallback) => {
                        tracing::warn!(
                            prompt = name.as_str(),
                            %error,
                            "fetching the prompt failed; using the fallback"
                        );
                        Ok(FetchedPrompt::from_fallback(&name, fallback))
                    }
                    None => Err(error),
                }
            }
        }
    }

    /// Fetch prompts into the cache ahead of time (for example at startup), so that the first
    /// [`LangfuseClient::prompt`] call does not wait. Each entry is `(name, version, label)`.
    /// Returns the first error; prompts before it are cached.
    pub async fn prefetch_prompts<'a>(
        &self,
        prompts: impl IntoIterator<Item = (&'a str, Option<i32>, Option<&'a str>)>,
    ) -> Result<()> {
        for (name, version, label) in prompts {
            let prompt = self.get_prompt(name, version, label).await?;
            self.prompt_cache
                .put(CacheKey::new(name, version, label), prompt);
        }
        Ok(())
    }

    /// Empty the prompt cache (shared by this client's clones).
    pub fn clear_prompt_cache(&self) {
        self.prompt_cache.lock().clear();
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn templates_fill_known_variables_only() {
        let vars = HashMap::from([("a".to_owned(), "1".to_owned())]);
        assert_eq!(
            compile_template("x {{a}} {{ a }} {{b}}", &vars),
            "x 1 1 {{b}}"
        );
        assert_eq!(compile_template("open {{a", &vars), "open {{a");
        assert_eq!(compile_template("", &vars), "");
    }

    #[test]
    fn no_version_or_label_shares_the_production_entry() {
        assert_eq!(
            CacheKey::new("p", None, None),
            CacheKey::new("p", None, Some("production"))
        );
        assert_ne!(
            CacheKey::new("p", Some(1), None),
            CacheKey::new("p", None, None)
        );
    }

    #[test]
    fn chat_placeholders_are_filled_or_kept() {
        use langfuse_client_base::models::{
            prompt_chat_prompt, ChatMessage, ChatMessageWithPlaceholders, PlaceholderMessage,
            PromptChatPrompt,
        };
        let prompt = FetchedPrompt {
            prompt: Prompt::ChatPrompt(PromptChatPrompt::new(
                "chat".into(),
                3,
                None,
                vec![],
                vec![],
                vec![
                    ChatMessageWithPlaceholders::ChatMessage(ChatMessage {
                        role: "system".into(),
                        content: "You are {{persona}}".into(),
                        r#type: None,
                    }),
                    ChatMessageWithPlaceholders::PlaceholderMessage(PlaceholderMessage {
                        name: "history".into(),
                        r#type: None,
                    }),
                    ChatMessageWithPlaceholders::PlaceholderMessage(PlaceholderMessage {
                        name: "missing".into(),
                        r#type: None,
                    }),
                ],
                prompt_chat_prompt::Type::Chat,
            )),
            source: PromptSource::Api,
        };
        let compiled = prompt.compile_with_placeholders(
            [("persona", "a pirate")],
            [(
                "history",
                vec![serde_json::json!({"role": "user", "content": "hi"})],
            )],
        );
        assert_eq!(
            compiled,
            CompiledPrompt::Chat(vec![
                serde_json::json!({"role": "system", "content": "You are a pirate"}),
                serde_json::json!({"role": "user", "content": "hi"}),
                serde_json::json!({"type": "placeholder", "name": "missing"}),
            ])
        );
        assert_eq!(prompt.version(), Some(3));
        assert_eq!(prompt.name(), "chat");
    }

    #[test]
    fn fallbacks_have_no_version() {
        let prompt = FetchedPrompt::from_fallback("f", PromptFallback::chat([("user", "{{q}}")]));
        assert!(prompt.is_fallback());
        assert_eq!(prompt.version(), None);
        assert_eq!(
            prompt.compile([("q", 42)]),
            CompiledPrompt::Chat(vec![serde_json::json!({"role": "user", "content": "42"})])
        );
    }
}
