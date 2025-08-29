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
# Trace examples
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces

# Observations (spans, generations, events)
cargo run --example observations

# Scoring and evaluation
cargo run --example scores
```

## API Coverage

### Currently Implemented ✅

#### Traces
- Full trace creation with metadata support
- Session and user tracking
- Tags and custom timestamps
- Input/output data capture

#### Observations
- **Spans** - Track execution steps and nested operations
- **Generations** - Monitor LLM calls with token usage
- **Events** - Log important milestones and errors
- Nested observations with parent-child relationships
- Log levels (DEBUG, INFO, WARNING, ERROR)

#### Scoring
- **Numeric scores** - Evaluate with decimal values (0.0-1.0)
- **Categorical scores** - Text-based classifications
- **Binary scores** - Success/failure tracking
- **Rating scores** - Star ratings and scales
- Trace-level and observation-level scoring
- Score metadata and comments

### Coming Soon 🚧
- Dataset management
- Prompt management
- Batch operations
- Fetching existing traces

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