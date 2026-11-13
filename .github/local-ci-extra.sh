#!/usr/bin/env bash
# Local equivalents of this repo's own heavy CI jobs, run by
# genai-rs/.github scripts/local-ci.sh after the shared checks.
set -euo pipefail

# ci.yml `features` job (Feature Combinations).
cargo +stable test --no-default-features
cargo +stable test
cargo +stable test --no-default-features --features rustls
cargo +stable test --no-default-features --features native-tls
cargo +stable test --features compression
cargo +stable test --no-default-features --features "compression,native-tls"
cargo +stable test --all-features
# The workspace: the macros crate and the `otel-context` interop.
cargo +stable test --workspace --all-features
