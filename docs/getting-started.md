# Getting started

## Install

```bash
cargo add langfuse-ergonomic
cargo add tokio --features full
cargo add serde_json
```

or by hand (the [crates.io badge](https://crates.io/crates/langfuse-ergonomic) shows the
latest version):

```toml
[dependencies]
langfuse-ergonomic = "0.x"   # see the crates.io badge for the current release
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

The default features are `rustls` (TLS) and `macros` (the `#[observe]` attribute). See
[Configuration](configuration.md#features) for the others.

## Credentials

```bash
export LANGFUSE_PUBLIC_KEY=pk-lf-...
export LANGFUSE_SECRET_KEY=sk-lf-...
export LANGFUSE_BASE_URL=https://cloud.langfuse.com   # optional
```

## Trace a function

The quickest way in is the [`#[observe]` attribute](observe.md): every call becomes an
observation, nested calls become children, and errors, panics and cancellation are recorded.

```rust,no_run
use langfuse_ergonomic::{observe, ClientBuilder};

#[observe(as_type = "generation", model = "gpt-4o")]
async fn complete(prompt: &str) -> Result<String, std::io::Error> {
    Ok(format!("echo: {prompt}"))
}

#[observe(user_id = "user-123")]
async fn handle(question: String) -> Result<String, std::io::Error> {
    complete(&question).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
    observe::install_batcher(batcher.clone());

    handle("What is Langfuse?".to_owned()).await?;

    batcher.shutdown().await?; // send what is queued before exiting
    Ok(())
}
```

## Write traces and observations yourself

The builders give full control over ids, timestamps and every field
([Tracing](tracing.md)):

```rust,no_run
use langfuse_ergonomic::ClientBuilder;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.build()?;

    let trace = client
        .trace()
        .name("my-application")
        .input(json!({"query": "Hello, world!"}))
        .output(json!({"response": "Hi there!"}))
        .user_id("user-123")
        .tags(vec!["production".to_owned(), "chat".to_owned()])
        .call()
        .await?;
    println!("Created trace {} ({})", trace.id, trace.url());

    client
        .generation()
        .trace_id(&trace.id)
        .name("chat-completion")
        .model("gpt-4o")
        .prompt_tokens(12)
        .completion_tokens(30)
        .call()
        .await?;

    // Read the trace back: all of its observations
    let observations = client.trace_observations(&trace.id).await?;
    println!("The trace has {} observations", observations.len());
    Ok(())
}
```

## Next

- [Configuration](configuration.md): environment, API versions, HTTP client, features
- [Observe](observe.md): the attribute, context propagation, OpenTelemetry interop
- [Batching](batching.md): the batcher behind high-volume ingestion
- [Examples](../examples): runnable programs (`cargo run --example basic_trace`)
