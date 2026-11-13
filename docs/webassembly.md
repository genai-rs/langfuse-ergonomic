# WebAssembly

The crate builds for `wasm32-unknown-unknown` in a JavaScript host (a browser, a web worker,
Node.js, Cloudflare Workers): the client, the builders, the batcher and `#[observe]` work as natively. Build
it without the TLS features, which add nothing there (the host's `fetch` does TLS):

```toml
[dependencies]
langfuse-ergonomic = { version = "0.x", default-features = false, features = ["macros"] }
# optionally also: "retry", "tracing-http", "otel-context"; see the crates.io badge for the version
```

```bash
cargo build --target wasm32-unknown-unknown --no-default-features
```

What differs, behind `#[cfg(target_family = "wasm")]`:

- **Configuration.** There is no process environment: `ClientBuilder::from_env` is not
  available. Configure the builder explicitly (`ClientBuilder::new().public_key(..)
  .secret_key(..).base_url(..)`), for example from a Worker's `env` bindings.
- **HTTP.** reqwest sends with the host's `fetch`, which owns connections, timeouts and
  compression: `timeout`, `connect_timeout` and the `compression` feature do not apply.
  `http_client` takes a `timvw_rust_runtime::Client` as natively; its service need not be
  `Send` (see the [runtime's WebAssembly guide](https://docs.rs/timvw-rust-runtime)).
- **Batcher.** The background task runs with `wasm_bindgen_futures::spawn_local` instead of
  `tokio::spawn`, and its interval, retry backoff and `Retry-After` waits use the host's
  `setTimeout` (`gloo-timers`); clocks are `performance.now()`/`Date.now()` (`web-time`), as
  `std::time::Instant::now()` panics there. Nothing needs to be `Send`, and no tokio runtime
  is involved (tokio is only used for its `sync` primitives).
- **Observations.** The current observation is entered on every `poll` of an observed future,
  so it follows a task across `.await` on the host's event loop without any runtime; wrap a
  future given to `spawn_local` with `.in_current_observation()` to keep it inside the caller's
  observation. Nothing needs to be `Send`, and observations end through `Batcher::try_add`.
  A panic aborts the module there (`panic = "abort"`), so a panicking observation is not
  recorded as such, and a panicking mask function cannot be caught.
- **Request-scoped hosts.** Cloudflare Workers (and similar hosts) stop running a request's
  tasks once its response is returned, so the batcher's interval does not keep flushing in
  the background. Flush explicitly at the end of each request, or pass the flush to
  `ctx.waitUntil`:

  ```rust,ignore
  batcher.add(event).await?;
  // ... handle the request ...
  batcher.flush().await?; // or `shutdown()` if the batcher lives only for this request
  ```

`tests/wasm.rs` runs the batcher (interval and size flushes, retries with backoff, shutdown),
`validate()`'s timeout and the observation context (nesting across `.await` and
`spawn_local`, cancellation, and `#[observe]` with the `macros` feature), an experiment with a
non-`Send` task, the prompt cache with its fallbacks and a media upload in Node.js against a scripted HTTP service, with
[`wasm-bindgen-test-runner`](https://rustwasm.github.io/wasm-bindgen/wasm-bindgen-test/index.html)
(install `wasm-bindgen-cli` of the `wasm-bindgen` version in `Cargo.lock`):

```bash
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test --target wasm32-unknown-unknown --no-default-features --test wasm
# and with the attribute:
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
  cargo test --target wasm32-unknown-unknown --no-default-features --features macros --test wasm
```

The other tests and the examples are native only (mockito, tokio).
