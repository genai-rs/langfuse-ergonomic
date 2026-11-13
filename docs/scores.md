# Scores

Scores evaluate traces and observations: numbers, booleans or categories, with an optional
comment and metadata. With Langfuse v3 they are `score-create` ingestion events; with v4 they
go to `POST /api/public/scores`.

```rust,no_run
use langfuse_ergonomic::LangfuseClient;
use serde_json::json;

async fn example(client: &LangfuseClient, trace_id: &str, observation_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    // A numeric score of an observation, with a comment and metadata
    client
        .score()
        .trace_id(trace_id)
        .observation_id(observation_id)
        .name("relevance")
        .value(0.85)
        .comment("Mostly on topic")
        .metadata(json!({"judge": "gpt-4o"}))
        .call()
        .await?;

    // Helpers for common shapes
    client.binary_score(trace_id, "resolved", true).await?;
    client.rating_score(trace_id, "stars", 4, 5).await?;
    client.categorical_score(trace_id, "intent", "refund").await?;

    // Link a score to an annotation queue for human review
    client
        .score()
        .trace_id(trace_id)
        .name("manual_review_verdict")
        .queue_id("annotation-queue-123")
        .string_value("needs_follow_up")
        .call()
        .await?;
    Ok(())
}
```

Inside [`#[observe]`](observe.md) functions, `observe::score_current(name, value)` scores the
current observation and `observe::score_current_trace(..)` its trace; `value` is an `f64`, a
`bool` or a category (`ScoreValue`). [Experiments](datasets-and-experiments.md) send their
evaluators' results as scores.

See `examples/scores.rs`.
