# langfuse-ergonomic

[![Crates.io](https://img.shields.io/crates/v/langfuse-ergonomic)](https://crates.io/crates/langfuse-ergonomic)
[![docs.rs](https://img.shields.io/docsrs/langfuse-ergonomic)](https://docs.rs/langfuse-ergonomic)
[![MSRV](https://img.shields.io/crates/msrv/langfuse-ergonomic)](Cargo.toml)
[![Dependencies](https://deps.rs/crate/langfuse-ergonomic/latest/status.svg)](https://deps.rs/crate/langfuse-ergonomic)
[![CI](https://github.com/genai-rs/langfuse-ergonomic/actions/workflows/ci.yml/badge.svg)](https://github.com/genai-rs/langfuse-ergonomic/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/langfuse-ergonomic)](#license)

Ergonomic Rust client for [Langfuse](https://langfuse.com), the open-source LLM engineering
platform: trace your LLM application, manage prompts, run experiments on datasets and score
the results. Builders on top of the generated
[langfuse-client-base](https://github.com/genai-rs/langfuse-client-base), for Langfuse v3 and
v4, natively and on WebAssembly.

## Quick start

```bash
cargo add langfuse-ergonomic
cargo add tokio --features full
```

```rust,no_run
use langfuse_ergonomic::{observe, ClientBuilder};

#[observe(as_type = "generation", model = "gpt-4o")]
async fn answer(question: &str) -> Result<String, std::io::Error> {
    Ok(format!("You asked: {question}")) // call your model here
}

#[observe(user_id = "user-123")]
async fn handle(question: String) -> Result<String, std::io::Error> {
    answer(&question).await // a child observation of `handle`
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // LANGFUSE_PUBLIC_KEY, LANGFUSE_SECRET_KEY and optionally LANGFUSE_BASE_URL
    let batcher = ClientBuilder::from_env()?.build()?.batcher().build().await;
    observe::install_batcher(batcher.clone());

    handle("What is Langfuse?".to_owned()).await?;

    batcher.shutdown().await?;
    Ok(())
}
```

## Features

- **`#[observe]`**: trace functions automatically, nested across `.await`, with errors,
  panics and cancellation recorded ([Observe](docs/observe.md)).
- **OpenTelemetry interop**: spans from [openai-ergonomic](https://github.com/genai-rs/openai-ergonomic)
  and other instrumentations nest under your observations ([Observe](docs/observe.md#opentelemetry-and-openai-ergonomic)).
- **Builders** for traces and every observation type: spans, generations (usage, cost,
  prompts), events, agents, tools, chains, retrievers, evaluators, embeddings, guardrails
  ([Tracing](docs/tracing.md)).
- **Batching** with backpressure, exponential backoff and `Retry-After` ([Batching](docs/batching.md)).
- **Prompts** with a cache, fallbacks and compilation ([Prompts](docs/prompts.md)).
- **Experiments** over datasets with evaluators ([Datasets and experiments](docs/datasets-and-experiments.md)).
- **Scores**, **media** uploads and a **masking** hook ([Scores](docs/scores.md),
  [Media](docs/media.md), [Masking](docs/masking.md)).
- **Langfuse v3 and v4** (OpenTelemetry ingestion, the default) ([API versions](docs/api-versions.md)).
- **WebAssembly**: browsers, Node.js, Cloudflare Workers ([WebAssembly](docs/webassembly.md)).

## Documentation

| Page | Contents |
|---|---|
| [Getting started](docs/getting-started.md) | Install, credentials, first traces |
| [Configuration](docs/configuration.md) | Environment, client options, HTTP middleware, feature flags |
| [API versions](docs/api-versions.md) | Langfuse v3 and v4: transport, ids, updates, attribute mapping, reading data |
| [Tracing](docs/tracing.md) | Traces, observations, observation types, generations |
| [Observe](docs/observe.md) | `#[observe]`, context propagation, OpenTelemetry and openai-ergonomic |
| [Batching](docs/batching.md) | The batcher: defaults, retries, backpressure |
| [Scores](docs/scores.md) | Numeric, boolean and categorical scores |
| [Prompts](docs/prompts.md) | Cache, fallbacks, compilation, management |
| [Datasets and experiments](docs/datasets-and-experiments.md) | Datasets, experiments, evaluators |
| [Media](docs/media.md) | Uploads, downloads, reference strings |
| [Masking](docs/masking.md) | Redacting inputs, outputs and metadata |
| [WebAssembly](docs/webassembly.md) | `wasm32-unknown-unknown` in JavaScript hosts |
| [Migrating](docs/migration.md) | Upgrade notes per release |
| [Development](docs/development.md) | Workspace layout, tests, live tests, local CI |

The API reference is on [docs.rs](https://docs.rs/langfuse-ergonomic), and
[`examples/`](examples) holds runnable programs (`cargo run --example observe`).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Contributions are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).
