# Contributing to langfuse-ergonomic

Thank you for your interest in contributing to langfuse-ergonomic! This guide will help you get started.

## Development Setup

### Prerequisites

- Rust stable, and the MSRV toolchain (the `rust-version` in `Cargo.toml`, shown by the MSRV
  badge) to run the MSRV checks
- Docker, only for the [live tests](docs/development.md#live-tests)

### Getting Started

1. Fork and clone the repository:
```bash
git clone https://github.com/YOUR_USERNAME/langfuse-ergonomic.git
cd langfuse-ergonomic
```

2. Set up the development environment:
```bash
# Point git at .githooks/ (the pre-commit hook runs fmt, clippy, build and tests)
./scripts/setup-hooks.sh
```

3. Create a `.env` file with your Langfuse credentials:
```bash
cp .env.example .env
# Edit .env with your credentials
```

## Development Workflow

### Making Changes

1. **Create a feature branch:**
```bash
git checkout -b feat/your-feature-name
# or: fix/your-bug-fix
# or: docs/your-docs-change
```

2. **Make your changes**
   - Follow the existing code style
   - Add tests for new functionality
   - Update documentation as needed

3. **Run the checks:**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Before pushing, run the organization's local CI (`scripts/local-ci.sh` from
[genai-rs/.github](https://github.com/genai-rs/.github)): it pushes and attests the commit
as `local-ci`, which lets the pull request skip the heavy CI jobs. See
[Local CI](docs/development.md#local-ci).

4. **Commit your changes:**
```bash
git add -A
git commit -m "feat: your descriptive commit message"
```

Use [conventional commits](https://www.conventionalcommits.org/):
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `chore:` for maintenance tasks
- `test:` for test changes
- `refactor:` for code refactoring

5. **Push and create a PR:**
```bash
git push origin feat/your-feature-name
gh pr create  # or use GitHub web UI
```

## Code Guidelines

### Rust Code

- Follow Rust naming conventions
- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Write idiomatic Rust code
- Add documentation comments for public APIs

### Generated Code

The `langfuse-client-base` crate contains auto-generated code from the OpenAPI specification. 

This repository does not contain it: it depends on the published crate from crates.io. Fixes
to the generated client belong in
[langfuse-client-base](https://github.com/genai-rs/langfuse-client-base).

### Examples

When adding new features, please include examples:

1. Create an example file in `examples/`
2. Add it to `Cargo.toml`:
```toml
[[example]]
name = "your_example"
path = "examples/your_example.rs"
```
3. Test the example:
```bash
cargo run --example your_example
```

## Testing

### Unit Tests

Add unit tests for new functionality:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_feature() {
        // Your test
    }
}
```

### Integration Tests

The live tests in `tests/integration_test.rs` and `tests/generation_usage_e2e.rs`
are ignored in normal runs because they need a Langfuse server. Run them
against a throwaway self-hosted Langfuse (Docker required) for both API
versions with:

```bash
scripts/live-tests.sh
```

See [Running the live tests](docs/development.md#live-tests) for the options
and for running them against another server. CI runs them on every pull
request.

`generation_usage_e2e` writes a uniquely named synthetic trace and five
generations, then polls the read API for persisted token usage. It checks
supplied counts, server-derived totals, explicit totals, partial usage, and
zeros. It does not call an LLM provider.

For features that require API calls, add integration tests that can run with real credentials:

```rust
mod common;

#[tokio::test]
#[ignore = "requires a Langfuse server; run scripts/live-tests.sh"]
async fn test_api_feature() {
    let client = common::live_client();
    // Your test; use common::wait_for_observation / wait_for_trace to read data back,
    // since Langfuse ingests asynchronously.
}
```

## Pull Request Process

1. **Ensure CI passes:** All GitHub Actions checks must pass
2. **Update documentation:** Include any necessary documentation updates
3. **Add tests:** New features should include tests
4. **Update CHANGELOG:** Note your changes if they're user-facing
5. **Request review:** Tag maintainers for review

## Project Structure

```
langfuse-ergonomic/
 src/                        # Hand-written ergonomic API
 langfuse-ergonomic-macros/  # The #[observe] procedural macro
 docs/                       # Guides (compiled as doc tests)
 examples/                   # Usage examples
 scripts/                    # Hook setup and live tests
 docker/langfuse/            # Self-hosted Langfuse for the live tests
 tests/                      # Unit, mock, trybuild, wasm and live tests
 .github/workflows/          # CI/CD configuration
```

The project depends on `langfuse-client-base` from crates.io, which contains the auto-generated OpenAPI client.

## Release Process

Releases are automated using [release-plz](https://release-plz.ieni.dev/):

1. Merge changes to `main`
2. release-plz creates a release PR automatically
3. Review and merge the release PR
4. Both crates (`langfuse-ergonomic` and `langfuse-ergonomic-macros`, same version) are
   published to crates.io automatically

Before merging a release PR, verify the manifest and lockfile agree on the
version, review the changelog (including any MSRV change), and check the current
required CI results. Validate the release candidate locally with:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
cargo test --locked --no-default-features
cargo +<msrv> test --locked --all-features   # <msrv>: the rust-version in Cargo.toml
cargo publish --locked --dry-run
```

The dry run packages and builds the crate without uploading it. Merging the
release PR triggers the `Release-plz` workflow on `main`, which publishes to
crates.io and creates the version tag and GitHub release. Treat that merge as
the publication step.
Confirm the published crate version and workflow result before closing the
release request.

## Getting Help

- Open an [issue](https://github.com/genai-rs/langfuse-ergonomic/issues) for bugs or feature requests
- Check existing issues before creating a new one
- Join the [Langfuse community](https://langfuse.com/docs/community) for general questions

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive community.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
