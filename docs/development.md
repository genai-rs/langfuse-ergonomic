# Development

See also [CONTRIBUTING.md](../CONTRIBUTING.md).

## Layout

The repository is a Cargo workspace:

- `.` is `langfuse-ergonomic`, the library;
- `langfuse-ergonomic-macros/` is the `#[observe]` procedural macro (proc macros must be a crate
  of their own). Its expansion calls `langfuse_ergonomic::__private`, so both crates are
  released together with the same version and the library pins the macros crate exactly.
  release-plz publishes both (`release-plz.toml`).

## Tests

```bash
cargo test --workspace --all-features      # unit, mock (mockito), trybuild and doc tests
cargo test --no-default-features           # without the macros and TLS features
cargo clippy --workspace --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
```

- `tests/observe_test.rs`: `#[observe]` against a mock Langfuse (v3 and v4): nesting with
  parent ids, async and sync, early return, `?`, panic and cancellation, `Err` levels,
  skip and capture options, streams, sampling, no sink, spawned tasks, masking.
- `tests/observe_otel_test.rs` (feature `otel-context`): OpenTelemetry spans nest under
  observations, and root observations join an active OpenTelemetry span.
- `tests/observe_ui.rs`: compile errors for bad `#[observe]` usage (trybuild; run with
  `TRYBUILD=overwrite` to update `tests/ui/*.stderr` after an intended change).
- `tests/experiments_test.rs`, `tests/media_test.rs`, `tests/round_015_test.rs`: experiments,
  media, observation types, generation details, masking, `try_add`, backoff and prompts.
- The docs pages under `docs/` and the README are compiled as doc tests (see the
  `doc_pages` module in `src/lib.rs`).
- WebAssembly: see [WebAssembly](webassembly.md#webassembly).

## Local CI

`genai-rs/.github`'s `scripts/local-ci.sh` runs the shared checks and then
`.github/local-ci-extra.sh`, this repository's feature combinations and workspace tests. The
MSRV (the `rust-version` in `Cargo.toml`) is checked with `cargo +<msrv> clippy` and
`cargo +<msrv> test`.

## Live tests

The live tests (`tests/integration_test.rs`, `tests/generation_usage_e2e.rs`) write
to and read back from a real Langfuse server. They are `#[ignore]`d, so a plain
`cargo test` stays offline. `scripts/live-tests.sh` runs them against a
throwaway self-hosted Langfuse v4 (Docker Compose, [`docker/langfuse/compose.yml`](../docker/langfuse/compose.yml))
that starts with a known project and test API keys:

```bash
scripts/live-tests.sh            # start Langfuse, run the tests for v3 and v4, stop it
scripts/live-tests.sh all v4     # only ApiVersion::V4
scripts/live-tests.sh up         # or step by step: start and wait until healthy,
scripts/live-tests.sh test v3    #   run the tests (repeat as needed),
scripts/live-tests.sh down       #   stop and delete all data
```

`LANGFUSE_PORT` changes the host port (default 3000). The images are pinned
by tag and digest in the compose file. The UI is at `http://localhost:3000`
(`live-test@example.com` / `live-test-password`).

The server runs with `LANGFUSE_MIGRATION_V4_WRITE_MODE=dual`, which keeps the
legacy v3 ingestion and read endpoints next to the v4 ones, so one server
covers both `ApiVersion::V3` and `ApiVersion::V4` (the default `events_only`
mode removes the v3 endpoints; see the
[v3 to v4 upgrade guide](https://langfuse.com/self-hosting/upgrade/upgrade-guides/upgrade-v3-to-v4)).
CI runs the same tests for both versions on every pull request.

To run them against another server, set `LANGFUSE_PUBLIC_KEY`,
`LANGFUSE_SECRET_KEY`, `LANGFUSE_BASE_URL` and `LANGFUSE_API_VERSION` (`v3` or
`v4`; required here, so a missing variable cannot silently test the default) and run
`cargo test --test integration_test --test generation_usage_e2e -- --ignored`.
The tests create uniquely named traces, scores, datasets and prompts and read
them back (scores through `GET /api/public/v2/scores/{id}`).
