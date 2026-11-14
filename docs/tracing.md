# Tracing with the builders

The builders write traces and observations with explicit ids and fields. For tracing code
automatically, see [Observe](observe.md); both write the same data and can be mixed.

How each call is sent depends on the [API version](api-versions.md): v3 ingestion events or
v4 OpenTelemetry spans. The builders keep their signatures in both.

## Traces

```rust,no_run
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient) -> Result<(), Box<dyn std::error::Error>> {
    let trace = client
        .trace()
        .id("conversation-42")             // optional: your own id (mapped in v4)
        .name("support-chat")
        .user_id("user-123")
        .session_id("session-9")
        .tags(vec!["production".to_owned()])
        .input(json!({"question": "Where is my order?"}))
        .output(json!({"answer": "It ships tomorrow."}))
        .metadata(json!({"channel": "web"}))
        .release("2026.11")
        .call()
        .await?;
    println!("{} -> {}", trace.id, trace.url());
    Ok(())
}
```

`delete_trace` and `delete_multiple_traces` work with both API versions.

## Observations

Spans, generations and events take the trace id, an optional parent and their fields. Write
each observation once, complete with its end time and output (the pattern Langfuse v4
prefers); `update_span()` and `update_generation()` exist for the cases that need them
([Updates in V4](api-versions.md#updates-in-v4)).

```rust,no_run
use chrono::Utc;
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient, trace_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let started = Utc::now();
    let span_id = client
        .span()
        .trace_id(trace_id)
        .name("retrieve-documents")
        .input(json!({"query": "order status"}))
        .output(json!({"documents": 3}))
        .start_time(started)
        .end_time(Utc::now())
        .call()
        .await?;

    client
        .event()
        .trace_id(trace_id)
        .parent_observation_id(&span_id)
        .name("cache-miss")
        .level("WARNING")
        .call()
        .await?;
    Ok(())
}
```

Levels are `DEBUG`, `DEFAULT` (also `INFO`), `WARNING` and `ERROR`; `status_message` explains
them.

## Observation types

Besides spans, generations and events, Langfuse knows agent, tool, chain, retriever,
evaluator, embedding and guardrail observations, which its UI shows with their own icons and
filters. Each has a builder; `observation().as_type(..)` takes the type as a value:

```rust,no_run
use langfuse_ergonomic::{LangfuseClient, ObservationType};
use serde_json::json;

async fn example(client: &LangfuseClient, trace_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let agent = client
        .agent()
        .trace_id(trace_id)
        .name("travel-agent")
        .input(json!({"goal": "book a flight"}))
        .call()
        .await?;

    client
        .tool()
        .trace_id(trace_id)
        .parent_observation_id(&agent)
        .name("search-flights")
        .input(json!({"from": "BRU", "to": "LIS"}))
        .output(json!({"flights": 4}))
        .call()
        .await?;

    client
        .observation()
        .as_type(ObservationType::Guardrail)
        .trace_id(trace_id)
        .parent_observation_id(&agent)
        .name("pii-check")
        .output(json!({"passed": true}))
        .call()
        .await?;
    Ok(())
}
```

| Builder | Type | v3 event | v4 `langfuse.observation.type` |
|---|---|---|---|
| `span()` | span | `span-create` | `span` |
| `generation()` | generation | `generation-create` | `generation` |
| `event()` | event | `event-create` | `event` |
| `agent()`, `tool()`, `chain()`, `retriever()`, `evaluator()`, `embedding()`, `guardrail()` | as named | `agent-create`, `tool-create`, ... (the type, in lower case, then `-create`) | the type, in lower case |

The same applies to `#[observe(as_type = "...")]` and `Observation::builder(..).as_type(..)`.
With `ApiVersion::V3`, the agent, tool, chain, retriever, evaluator, embedding and guardrail
types need Langfuse v3.99.0 or later: the legacy `observation-create` event only takes span,
generation and event, and older servers reject the `<type>-create` events. A rejected event
fails the builder call with `Error::PartialFailure`, is listed in the failures of
`Batcher::flush`/`Batcher::shutdown` and counted in `BatcherMetrics::failed`, and the batcher
logs it as a warning.

`observation()` and the typed builders take the generation fields too (`model`,
`model_parameters`, `completion_start_time`, `usage_details`, `cost_details`,
`prompt_name`/`prompt_version`). Generations send them all; the other types keep the model,
its parameters, the completion start time and the `input`/`output`/`total` usage and cost.

## Generations

```rust,no_run
use std::collections::HashMap;

use chrono::{Duration, Utc};
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient, trace_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let start = Utc::now();
    client
        .generation()
        .trace_id(trace_id)
        .name("answer")
        .model("gpt-4o")
        .model_parameters(json!({"temperature": 0.2}))
        .input(json!([{"role": "user", "content": "Hi"}]))
        .output(json!({"role": "assistant", "content": "Hello!"}))
        .start_time(start)
        .completion_start_time(start + Duration::milliseconds(180)) // time to first token
        .end_time(start + Duration::milliseconds(900))
        .usage_details(HashMap::from([
            ("input".to_owned(), 12),
            ("output".to_owned(), 30),
            ("cache_read_input_tokens".to_owned(), 8),
        ]))
        .cost_details(HashMap::from([("total".to_owned(), 0.000_21)]))
        .prompt_name("support-answer") // link to a Langfuse prompt
        .prompt_version(3)
        .call()
        .await?;
    Ok(())
}
```

- `usage_details` holds token counts by usage type; `prompt_tokens`, `completion_tokens`
  and `total_tokens` are shortcuts for `input`, `output` and `total`. Without a total,
  Langfuse derives it.
- `cost_details` holds costs in USD; without them Langfuse infers costs from the model and
  the usage.
- `prompt_name` and `prompt_version` link the generation to a [prompt](prompts.md)
  (`FetchedPrompt::name()` and `version()`; a fallback prompt has no version and is not
  linked).

## Ids

Ids are optional: the builders generate one, and return the id Langfuse stores. With Langfuse
v4 your own ids are mapped deterministically to OpenTelemetry ids, so passing the same id
again (or the returned one) addresses the same trace or observation
([Ids in V4](api-versions.md#ids-in-v4)). `IdGenerator` derives stable ids from seeds.

See also `examples/observations.rs`, `examples/trace_with_metadata.rs` and
`examples/observation_types.rs`.
