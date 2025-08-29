# langfuse-ergonomic

[![Crates.io](https://img.shields.io/crates/v/langfuse-ergonomic.svg)](https://crates.io/crates/langfuse-ergonomic)
[![Documentation](https://docs.rs/langfuse-ergonomic/badge.svg)](https://docs.rs/langfuse-ergonomic)
[![CI](https://github.com/genai-rs/langfuse-ergonomic/workflows/CI/badge.svg)](https://github.com/genai-rs/langfuse-ergonomic/actions)
[![License](https://img.shields.io/crates/l/langfuse-ergonomic)](./LICENSE-MIT)

Ergonomic Rust client for [Langfuse](https://langfuse.com), the open-source LLM observability platform.

## Features

- 🏗️ **Builder Pattern** - Intuitive API using the [Bon](https://bon-rs.com) builder pattern library
- 🔄 **Async/Await** - Full async support with Tokio
- 🔒 **Type Safe** - Strongly typed with compile-time guarantees
- 🚀 **Easy Setup** - Simple configuration from environment variables
- 📊 **Comprehensive** - Support for traces, observations, scores, and more

## Installation

```toml
[dependencies]
langfuse-ergonomic = "0.1"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

## Quick Start

```rust
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client from environment variables
    let client = LangfuseClient::from_env()?;
    
    // Create a trace
    let trace = client.trace()
        .name("my-application")
        .input(json!({"query": "Hello, world!"}))
        .output(json!({"response": "Hi there!"}))
        .user_id("user-123")
        .tags(["production", "chat"])
        .send()
        .await?;
    
    println!("Created trace: {}", trace.id);
    Ok(())
}
```

## Configuration

Set these environment variables:

```bash
LANGFUSE_PUBLIC_KEY=pk-lf-...
LANGFUSE_SECRET_KEY=sk-lf-...
LANGFUSE_BASE_URL=https://cloud.langfuse.com  # Optional
```

Or configure explicitly:

```rust
let client = LangfuseClient::builder()
    .public_key("pk-lf-...")
    .secret_key("sk-lf-...")
    .base_url("https://cloud.langfuse.com")
    .build();
```

## Examples

Check the `examples/` directory for more usage examples:

```bash
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces
```

## API Coverage

### Currently Implemented ✅
- Trace creation with full metadata support
- Session tracking
- User identification
- Tags and metadata
- Custom timestamps

### Coming Soon 🚧
- Observations (spans, generations, events)
- Scoring system
- Dataset management
- Prompt management

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Links

- [Langfuse Documentation](https://langfuse.com/docs)
- [API Reference](https://api.reference.langfuse.com)
- [Base Client](https://github.com/genai-rs/langfuse-client-base)