# langfuse-rs

[![CI](https://github.com/timvw/langfuse-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/timvw/langfuse-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/timvw/langfuse-rs/graph/badge.svg?token=CODECOV_TOKEN)](https://codecov.io/gh/timvw/langfuse-rs)
[![Crates.io](https://img.shields.io/crates/v/langfuse-ergonomic.svg)](https://crates.io/crates/langfuse-ergonomic)
[![Documentation](https://docs.rs/langfuse-ergonomic/badge.svg)](https://docs.rs/langfuse-ergonomic)
[![License](https://img.shields.io/crates/l/langfuse-ergonomic.svg)](LICENSE-MIT)

A Rust client library for [Langfuse](https://langfuse.com) with both auto-generated API bindings and an ergonomic interface powered by the [Bon](https://bon-rs.com) builder pattern library.

This client provides full access to the [Langfuse Public API](https://langfuse.com/docs/api-and-data-platform/features/public-api), enabling you to integrate observability and monitoring into your Rust-based LLM applications.

## Project Structure

This project consists of two crates:

- **`langfuse-client-base`** - Auto-generated client from the Langfuse OpenAPI specification
- **`langfuse-ergonomic`** - User-friendly wrapper with builder patterns for better developer experience

## Features

- ✅ Full API coverage via OpenAPI generation
- ✅ Type-safe builder patterns using Bon
- ✅ Async/await support with Tokio
- ✅ Comprehensive error handling
- ✅ Support for traces, observations (spans/generations), and scores
- ✅ Flexible authentication (API keys, basic auth)

### Supported API Operations

This client provides access to all Langfuse Public API endpoints:

- **Traces**: Create and manage traces for LLM interactions
- **Observations**: Track spans, generations, and events within traces
- **Scores**: Evaluate trace quality with numeric, categorical, or binary scores
- **Sessions**: Group related traces into sessions
- **Prompts**: Manage and version prompts (when available)
- **Projects**: Access project configurations and settings

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
langfuse-ergonomic = { path = "langfuse-ergonomic" }
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### Basic Usage

```rust
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create client from environment variables
    let client = LangfuseClient::from_env()?;
    
    // Or create with explicit credentials
    let client = LangfuseClient::builder()
        .public_key("pk-lf-...")
        .secret_key("sk-lf-...")
        .build();
    
    // Create a trace
    let trace = client.trace()
        .name("my-llm-app")
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

## Langfuse API

### Authentication

The Langfuse Public API uses Basic Authentication with project-specific API keys:
- **Public Key**: Used as the username
- **Secret Key**: Used as the password

You can obtain these keys from your [Langfuse project settings](https://cloud.langfuse.com).

### API Endpoints

The client connects to Langfuse Cloud by default (`https://cloud.langfuse.com`), but you can configure it for different regions or self-hosted instances:

- **US Cloud**: `https://us.cloud.langfuse.com`
- **EU Cloud**: `https://cloud.langfuse.com` (default)
- **Self-hosted**: Your custom Langfuse instance URL

### Resources

- [Langfuse Public API Documentation](https://langfuse.com/docs/api-and-data-platform/features/public-api)
- [API Reference](https://api.reference.langfuse.com)
- [OpenAPI Specification](https://cloud.langfuse.com/generated/api/openapi.yml)
- [Postman Collection](https://cloud.langfuse.com/generated/postman/collection.json)

## Setup & Development

### Prerequisites

- Rust 1.75 or later
- Node.js (for OpenAPI generator)

### Initial Setup

1. Clone the repository:
```bash
git clone https://github.com/timvw/langfuse-rs.git
cd langfuse-rs
```

2. Initialize the development environment:
```bash
just init  # Sets up git hooks and checks dependencies
```

3. Generate the base client from OpenAPI:
```bash
just generate  # or ./scripts/generate-openapi-client.sh
```

4. Build the project:
```bash
cargo build
```

5. Run tests:
```bash
cargo test
```

### Development Workflow

Before committing code, ensure it passes all checks:

```bash
just pre-commit  # Runs fmt, clippy, build, and tests
```

Or run individual checks:
```bash
just fmt        # Format code
just lint       # Run clippy
just test       # Run tests
just check      # Run all checks
```

Git hooks are automatically configured to run these checks before each commit.

### Environment Variables

Create a `.env` file for running examples:

```env
LANGFUSE_PUBLIC_KEY=pk-lf-...
LANGFUSE_SECRET_KEY=sk-lf-...
LANGFUSE_BASE_URL=https://cloud.langfuse.com  # Optional
```

### Running Examples

```bash
cd langfuse-ergonomic

# Basic trace creation
cargo run --example basic_trace

# Trace with full metadata
cargo run --example trace_with_metadata  

# Multiple traces in a session
cargo run --example multiple_traces

# Simple test trace
cargo run --example test_trace
```

## API Overview

### Currently Implemented

#### Creating Traces

The library currently supports creating traces with the Langfuse ingestion API:

```rust
let trace = client.trace()
    .name("my-trace")
    .input(json!({"key": "value"}))
    .output(json!({"result": "data"}))
    .metadata(json!({"version": "1.0"}))
    .user_id("user-123")
    .session_id("session-456")
    .tags(["tag1", "tag2"])
    .send()
    .await?;
```

### Planned Features

The following features are planned for future releases:

#### Observations (Spans, Generations, Events)
- Creating spans for nested operations
- Recording LLM generation calls with token usage
- Logging events within traces

#### Scoring System
- Numeric scores for quality metrics
- Binary scores for success/failure
- Rating scores (e.g., 1-5 stars)
- Categorical scores for classification

#### Additional Functionality
- Fetching existing traces
- Updating traces and observations
- Batch operations for improved performance
- Dataset management
- Prompt management

## Architecture

### OpenAPI Generation

The `langfuse-client-base` crate is generated from the official Langfuse OpenAPI specification using `openapi-generator`. This ensures complete API coverage and type safety.

To regenerate the client:
```bash
./scripts/generate-openapi-client.sh
```

### Builder Pattern Enhancement

The `langfuse-ergonomic` crate wraps the generated client with intuitive builder patterns using the Bon library. This provides:

- Named parameters for better readability
- Optional parameters with sensible defaults
- Compile-time validation
- Method chaining for fluent interfaces

## Publishing & Releases

This project uses [release-plz](https://release-plz.ieni.dev/) to automate releases and publishing to crates.io.

### Automated Releases

When changes are merged to `main`, release-plz will:
1. Create a PR with version bumps and changelog updates
2. After merging the release PR, automatically:
   - Create GitHub releases with changelogs
   - Publish crates to crates.io
   - Tag the release

### Manual Publishing

To publish manually:
```bash
cargo publish -p langfuse-client-base
cargo publish -p langfuse-ergonomic
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Workflow

1. Make changes to the ergonomic layer in `langfuse-ergonomic/`
2. If API changes are needed, update the OpenAPI generation script
3. Run tests: `cargo test`
4. Format code: `cargo fmt`
5. Check lints: `cargo clippy`

## License

This project is licensed under MIT OR Apache-2.0.

## Acknowledgments

- [Langfuse](https://langfuse.com) for the observability platform
- [Bon](https://bon-rs.com) for the excellent builder macro library
- [OpenAPI Generator](https://openapi-generator.tech) for code generation
- Initial implementation created with [Claude Code](https://claude.ai/code)