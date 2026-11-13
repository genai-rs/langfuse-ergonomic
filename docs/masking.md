# Masking

A mask function redacts data before it leaves the process. Set it on the client:

```rust,no_run
use langfuse_ergonomic::ClientBuilder;
use serde_json::Value;

fn redact(value: Value) -> Value {
    match value {
        Value::String(text) if text.contains('@') => Value::String("[email]".to_owned()),
        Value::Array(items) => Value::Array(items.into_iter().map(redact).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let value = if key == "password" { Value::from("***") } else { redact(value) };
                    (key, value)
                })
                .collect(),
        ),
        other => other,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::from_env()?.mask(redact).build()?;
    let _ = client;
    Ok(())
}
```

The function gets each `input`, `output` and `metadata` value (and a score's metadata), whole,
and returns what to send instead. It applies to everything the client sends:

- the trace and observation builders (`trace()`, `span()`, `generation()`, `agent()`, ...);
- `Batcher::add` and `Batcher::try_add` (batchers use their client's mask);
- [`#[observe]`](observe.md) observations, which go through a batcher or the client;
- [experiments](datasets-and-experiments.md).

Clones of the client, and batchers built from it, share the function.

The function must not panic. If it does anyway, the value is replaced by `<masking failed>`
(the original is never sent) and a warning is logged. On WebAssembly a panic aborts the
module, so it cannot be caught there.

See `examples/masking.rs`.
