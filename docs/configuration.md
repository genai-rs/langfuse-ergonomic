# Configuration

## Environment

`ClientBuilder::from_env()` reads:

```bash
LANGFUSE_PUBLIC_KEY=pk-lf-...                  # required
LANGFUSE_SECRET_KEY=sk-lf-...                  # required
LANGFUSE_BASE_URL=https://cloud.langfuse.com   # optional (default Langfuse Cloud EU)
LANGFUSE_API_VERSION=v4                        # optional: v3 or v4 (default v4)
```

`from_env` is not available on WebAssembly, which has no process environment
([WebAssembly](webassembly.md)).

## Explicit configuration

```rust,no_run
use langfuse_ergonomic::{ApiVersion, ClientBuilder};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .public_key("pk-lf-...")
        .secret_key("sk-lf-...")
        .base_url("https://cloud.langfuse.com")  // or your self-hosted URL
        .timeout(Duration::from_secs(30))        // request timeout (default 60 s)
        .connect_timeout(Duration::from_secs(5)) // connection timeout (default 10 s)
        .user_agent("my-app/1.0.0")
        .api_version(ApiVersion::V4)             // Langfuse server API (default V4)
        .mask(|value| value)                     // redaction hook, see masking.md
        .build()?;
    let _ = client;
    Ok(())
}
```

Self-hosted servers only need their `base_url` (see `examples/self_hosted.rs`).

## Langfuse v3 and v4

The client writes to a Langfuse v4 server by default: traces and observations are sent as
OpenTelemetry spans, scores to the v4 scores API. Select `ApiVersion::V3` (or
`LANGFUSE_API_VERSION=v3`) for a server still on Langfuse v3. The client never probes the
server. Langfuse Cloud removes the v3 ingestion of traces and observations on 2026-11-16.

[API versions](api-versions.md) explains what changes: the transport, the id mapping, how
traces, updates and attributes are written, and how to read data back.

## HTTP client and middleware

Requests go through a [`timvw_rust_runtime::Client`](https://docs.rs/timvw-rust-runtime): a
[tower](https://docs.rs/tower) service, reqwest by default. The default client does not retry
by itself: the [batcher](batching.md) retries ingestion with exponential backoff and jitter. To
add retries for the other calls, timeouts, logging or middleware of your own, pass a client
with tower layers (feature `retry` for the runtime's retry policy):

```rust,ignore
use timvw_rust_runtime::{retry::RetryPolicy, Client, ReqwestService};
use tower::retry::RetryLayer;

let http_client = Client::from_service(ReqwestService::new(reqwest::Client::new()))
    .layer(RetryLayer::new(RetryPolicy::new(3)));
let client = ClientBuilder::from_env()?.http_client(http_client).build()?;
```

`timeout` and `connect_timeout` apply only to the default client. See
[`examples/http_middleware_retry.rs`](../examples/http_middleware_retry.rs) (run it with
`--features retry`) and the [runtime's guide](https://docs.rs/timvw-rust-runtime).

## Features

| Feature | Default | What it adds |
|---|---|---|
| `rustls` | yes | TLS with rustls |
| `macros` | yes | the `#[observe]` attribute ([Observe](observe.md)) |
| `native-tls` | | TLS with the platform's library instead |
| `compression` | | gzip, brotli and deflate responses |
| `retry` | | `timvw_rust_runtime::retry::RetryPolicy`, for clients set with `http_client` |
| `tracing-http` | | `timvw_rust_runtime::trace::TraceLayer`, which logs each request with `tracing` |
| `otel-context` | | OpenTelemetry context propagation for observations ([Observe](observe.md#opentelemetry-and-openai-ergonomic)) |

```toml
[dependencies]
langfuse-ergonomic = { version = "0.x", features = ["compression", "otel-context"] }
```

On WebAssembly use `default-features = false` (the host's `fetch` does TLS) and add `macros`
if you use `#[observe]` ([WebAssembly](webassembly.md)).
