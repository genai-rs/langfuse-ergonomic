# langfuse-ergonomic-macros

[![Crates.io](https://img.shields.io/crates/v/langfuse-ergonomic-macros)](https://crates.io/crates/langfuse-ergonomic-macros)
[![docs.rs](https://img.shields.io/docsrs/langfuse-ergonomic-macros)](https://docs.rs/langfuse-ergonomic-macros)
[![MSRV](https://img.shields.io/crates/msrv/langfuse-ergonomic-macros)](Cargo.toml)
[![Dependencies](https://deps.rs/crate/langfuse-ergonomic-macros/latest/status.svg)](https://deps.rs/crate/langfuse-ergonomic-macros)
[![CI](https://github.com/genai-rs/langfuse-ergonomic/actions/workflows/ci.yml/badge.svg)](https://github.com/genai-rs/langfuse-ergonomic/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/langfuse-ergonomic-macros)](#license)

The `#[observe]` attribute of [langfuse-ergonomic](https://crates.io/crates/langfuse-ergonomic).

Do not depend on this crate directly: use `langfuse_ergonomic::observe` (feature `macros`, on
by default). The expansion calls `langfuse_ergonomic`'s runtime, so the two crates are released
together with the same version.

```rust,ignore
use langfuse_ergonomic::observe;

#[observe(as_type = "generation", model = "gpt-4o")]
async fn answer(question: &str) -> Result<String, std::io::Error> {
    Ok(format!("You asked: {question}"))
}
```

See [Observe](https://github.com/genai-rs/langfuse-ergonomic/blob/main/docs/observe.md) for the
options and what is recorded.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
