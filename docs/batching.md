# Batching

The `Batcher` queues ingestion events and sends them in batches from a background task:
natively a tokio task, on WebAssembly a task of the JavaScript host. It is the recommended
sink for [`#[observe]`](observe.md) and what [experiments](datasets-and-experiments.md) use.

## Defaults

| Setting | Default |
|---|---|
| `max_events` | 100 events per batch |
| `max_bytes` | 3.5 MB per batch (Langfuse accepts 5 MB) |
| `flush_interval` | 5 seconds |
| `max_retries` | 3 |
| `initial_retry_delay` / `max_retry_delay` | 100 ms / 30 s |
| `retry_jitter` | on (up to 25% extra) |
| `max_queue_size` | 10,000 events |
| `backpressure_policy` | `Block` |

```rust,no_run
use langfuse_ergonomic::{BackpressurePolicy, Batcher, ClientBuilder, IngestionEvent};
use std::time::Duration;

async fn example(events: Vec<IngestionEvent>) -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.build()?;
    let batcher = Batcher::builder()
        .client(client)
        .max_events(50)
        .max_bytes(2_000_000)
        .flush_interval(Duration::from_secs(10))
        .max_retries(5)
        .max_queue_size(5_000)
        .backpressure_policy(BackpressurePolicy::DropNew)
        .build()
        .await;

    for event in events {
        batcher.add(event).await?; // or try_add(event) from synchronous code
    }

    let response = batcher.flush().await?;
    println!("sent {} events", response.success_count);

    let metrics = batcher.metrics();
    println!(
        "queued {}, flushed {}, failed {}, dropped {}, retries {}",
        metrics.queued, metrics.flushed, metrics.failed, metrics.dropped, metrics.retries
    );

    batcher.shutdown().await?; // flushes what is left, waiting for retries
    Ok(())
}
```

`client.batcher()` starts the builder with the client set.

## Adding events

- `add(event).await` queues an event; with `BackpressurePolicy::Block` it waits for room.
- `try_add(event)` never waits, so it works from synchronous code and `Drop`: a full queue
  drops the new event (`Error::Backpressure`), except with `DropOldest`, which first tries to
  drop the oldest buffered event. `#[observe]` ends observations with it.

Both apply the client's [mask](masking.md) first, and with Langfuse v4 reject events the v4
transport cannot send (an update of an unknown span) at once.

`Batcher` is cheap to clone: clones share the queue, the background task and the metrics. Keep
one clone to flush or shut down the batcher you installed elsewhere.

## Retries

Retryable failures (5xx, 429 and network errors) are retried up to `max_retries` times, with
exponential backoff: `initial_retry_delay * 2^n`, at most `max_retry_delay`, plus up to 25%
jitter. A `Retry-After` header, on a 429 or a 5xx response, raises the wait to at least that
long. A 413 splits the batch in two and sends the halves. Authentication failures stop at
once.

With Langfuse v3, a 207 response reports results per event, and only the failed retryable
events are retried. With Langfuse v4 an OTLP export succeeds or fails as a whole; scores are
sent one by one (see [API versions](api-versions.md)).

## Backpressure policies

- `Block` (default): `add` waits until there is room; `try_add` drops the new event.
- `DropNew`: drop the new event when the queue is full.
- `DropOldest`: drop the oldest buffered event to make room.

Dropped events are counted in `metrics().dropped`.

## Request-scoped hosts

Cloudflare Workers and similar hosts stop a request's tasks once its response is sent, which
stops the flush interval too: call `flush()` (or `shutdown()`) before returning the response.
See [WebAssembly](webassembly.md).

See `examples/batch_ingestion.rs`, `examples/batch_ingestion_207.rs` and
`examples/comprehensive_batch_test.rs`.
