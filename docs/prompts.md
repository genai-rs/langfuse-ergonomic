# Prompts

## Fetch, cache and compile

`client.prompt(name)` fetches a prompt through a cache shared by the client's clones:

- a cached prompt younger than its time to live (`cache_ttl`, default 60 seconds; zero
  disables the cache) is returned without a request;
- otherwise it is fetched and cached, keyed by name, version and label (no version and no
  label means the `production` label);
- when fetching fails, a stale cached prompt is returned (with a warning), else the
  caller-supplied `fallback`, else the error.

`FetchedPrompt::source` tells which: `Api`, `Cache`, `StaleCache` or `Fallback`. The refresh
happens in the call, without a background task, so it works on any runtime and on WebAssembly.

```rust,no_run
use langfuse_ergonomic::{ClientBuilder, PromptFallback, PromptSource};
use std::time::Duration;

async fn example() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.build()?;

    // Optional: fill the cache at startup.
    client
        .prefetch_prompts([("support-answer", None, Some("production"))])
        .await?;

    let prompt = client
        .prompt("support-answer")
        .label("production")
        .cache_ttl(Duration::from_secs(300))
        .fallback(PromptFallback::text("Answer briefly: {{question}}"))
        .call()
        .await?;
    if prompt.source == PromptSource::Fallback {
        eprintln!("Langfuse unreachable, using the built-in prompt");
    }

    // Text prompts compile to a string; `{{ name }}` with or without spaces.
    let compiled = prompt.compile([("question", "How do I reset my password?")]);
    println!("{:?} (model config: {:?})", compiled.as_text(), prompt.config());
    Ok(())
}
```

Chat prompts compile to messages (`{"role", "content"}` JSON objects, ready for most chat
APIs); placeholder messages are filled from `compile_with_placeholders`:

```rust,no_run
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient) -> Result<(), Box<dyn std::error::Error>> {
    let prompt = client.prompt("chat-agent").version(4).call().await?;
    let compiled = prompt.compile_with_placeholders(
        [("persona", "a patient tutor")],
        [("history", vec![json!({"role": "user", "content": "Hi!"})])],
    );
    for message in compiled.as_messages().unwrap_or_default() {
        println!("{message}");
    }
    Ok(())
}
```

Variables that are not given stay as `{{name}}`; placeholders that are not given stay as
`{"type": "placeholder", "name": ..}`. `clear_prompt_cache()` empties the cache.

## Link prompts to generations

A generation records which prompt produced it: `generation().prompt_name(prompt.name())
.maybe_prompt_version(prompt.version())`, or `observation.set_prompt(&prompt)` inside
[`#[observe]`](observe.md). Fallback prompts have no version and are not linked.

## Manage prompts

`create_prompt()` and `create_chat_prompt()` create prompts (or new versions),
`update_prompt_version()` adds (`labels`) and removes (`remove_labels`) a version's labels and
drops the prompt from the client's cache, `get_prompt()` fetches without the cache
and `list_prompts()` lists them. Invalid chat messages fail with their index before any
request.

See `examples/prompts.rs` and `examples/prompts_cache.rs`.
