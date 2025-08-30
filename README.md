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
- 🔁 **Batch Processing** - Automatic batching with retry logic and chunking
- ⚡ **Production Ready** - Built-in timeouts, connection pooling, and error handling
- 🏠 **Self-Hosted Support** - Full support for self-hosted Langfuse instances

## Installation

```toml
[dependencies]
langfuse-ergonomic = "*"
tokio = { version = "1", features = ["full"] }
serde_json = "1"
```

### Optional Features

```toml
[dependencies]
langfuse-ergonomic = { version = "*", features = ["compression"] }
```

- `compression` - Enable gzip, brotli, and deflate compression for requests (reduces bandwidth usage)

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
        .call()
        .await?;
    
    println!("Created trace: {}", trace.id);

    // Fetch and list traces
    let fetched_trace = client.get_trace(&trace.id).await?;
    let traces = client.list_traces()
        .limit(10)
        .user_id("user-123")
        .call()
        .await?;

    // Create a dataset
    let dataset = client.create_dataset()
        .name("my-dataset")
        .description("Example dataset")
        .call()
        .await?;
    
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

Or configure explicitly with advanced options:

```rust
use std::time::Duration;

let client = LangfuseClient::builder()
    .public_key("pk-lf-...")
    .secret_key("sk-lf-...")
    .base_url("https://cloud.langfuse.com")
    .timeout(Duration::from_secs(30))        // Custom timeout
    .connect_timeout(Duration::from_secs(5)) // Connection timeout
    .user_agent("my-app/1.0.0")              // Custom user agent
    .build();
```

## Examples

Check the `examples/` directory for more usage examples:

```bash
# Trace examples
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces

# Trace fetching and management
cargo run --example traces_fetch

# Observations (spans, generations, events)
cargo run --example observations

# Scoring and evaluation
cargo run --example scores

# Dataset management
cargo run --example datasets

# Prompt management
cargo run --example prompts

# Batch processing
cargo run --example batch_ingestion

# Self-hosted configuration
cargo run --example self_hosted
```

### Batch Processing

The client supports efficient batch processing with automatic chunking, retry logic, and comprehensive error handling:

#### Default Configuration
- **Max events per batch**: 100 events
- **Max batch size**: 3.5 MB (conservative limit for Langfuse Cloud's 5MB limit)
- **Auto-flush interval**: 5 seconds
- **Max retries**: 3 attempts with exponential backoff
- **Retry jitter**: Enabled by default (25% random jitter to avoid thundering herd)
- **Backpressure policy**: Block (waits when queue is full)
- **Max queue size**: 10,000 events

```rust
use langfuse_ergonomic::{Batcher, BackpressurePolicy, LangfuseClient};
use std::time::Duration;

let client = LangfuseClient::from_env()?;

// Create a batcher with custom configuration
let batcher = Batcher::builder()
    .client(client)
    .max_events(50)                            // Events per batch (default: 100)
    .max_bytes(2_000_000)                      // Max batch size in bytes (default: 3.5MB)
    .flush_interval(Duration::from_secs(10))   // Auto-flush interval (default: 5s)
    .max_retries(5)                            // Retry attempts (default: 3)
    .max_queue_size(5000)                      // Max events to queue (default: 10,000)
    .backpressure_policy(BackpressurePolicy::DropNew) // What to do when queue is full
    .build()
    .await;

// Add events - they'll be automatically batched
for event in events {
    batcher.add(event).await?;
}

// Manual flush if needed
let response = batcher.flush().await?;
println!("Sent {} events", response.success_count);

// Monitor metrics
let metrics = batcher.metrics();
println!("Queued: {}, Flushed: {}, Failed: {}, Dropped: {}", 
    metrics.queued, metrics.flushed, metrics.failed, metrics.dropped);

// Graceful shutdown (flushes remaining events)
let final_response = batcher.shutdown().await?;
```

#### Advanced Features

**207 Multi-Status Handling**: Automatically handles partial failures where some events succeed and others fail.

**Backpressure Policies**:
- `Block`: Wait when queue is full (default)
- `DropNew`: Drop new events when queue is full
- `DropOldest`: Remove oldest events to make room

**Metrics & Monitoring**:
```rust
let metrics = batcher.metrics();
// Available metrics:
// - queued: Current events waiting to be sent
// - flushed: Total successfully sent
// - failed: Total failed after all retries
// - dropped: Total dropped due to backpressure
// - retries: Total retry attempts
// - last_error_ts: Unix timestamp of last error
```

**Error Handling**:
```rust
match batcher.flush().await {
    Ok(response) => {
        println!("Success: {}, Failed: {}", 
            response.success_count, response.failure_count);
    }
    Err(Error::PartialFailure { success_count, failure_count, errors, .. }) => {
        println!("Partial success: {} ok, {} failed", success_count, failure_count);
        for error in errors {
            if error.retryable {
                println!("Retryable error: {}", error.message);
            }
        }
    }
    Err(e) => eprintln!("Complete failure: {}", e),
}
```

## API Coverage

### Implemented Features ✅

#### Traces
- **Creation** - Full trace creation with metadata support
- **Fetching** - Get individual traces by ID
- **Listing** - List traces with filtering and pagination
- **Management** - Delete single or multiple traces
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

#### Dataset Management
- **Creation** - Create datasets with metadata
- **Listing** - List all datasets with pagination
- **Fetching** - Get dataset details by name
- **Run Management** - Get, list, and delete dataset runs

#### Prompt Management
- **Fetching** - Get prompts by name and version
- **Listing** - List prompts with filtering
- **Creation** - Basic prompt creation (placeholder implementation)

#### Batch Processing
- **Automatic Batching** - Events are automatically grouped into optimal batch sizes
- **Size Limits** - Respects Langfuse's 3.5MB batch size limit
- **Retry Logic** - Exponential backoff for failed requests
- **Partial Failures** - Handles 207 Multi-Status responses
- **Background Processing** - Non-blocking event submission

#### Production Features
- **Timeouts** - Configurable request and connection timeouts
- **Compression** - Optional gzip, brotli, and deflate support (via `compression` feature flag)
- **HTTP/2** - Efficient connection multiplexing
- **Connection Pooling** - Reuses connections for better performance
- **Error Handling** - Structured error types with retry metadata
- **Self-Hosted Support** - Full compatibility with self-hosted instances

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