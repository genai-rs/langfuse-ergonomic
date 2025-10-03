# Contributing to langfuse-ergonomic

Thank you for your interest in contributing to langfuse-ergonomic! This guide will help you get started.

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Node.js 18+ (for OpenAPI generator)
- Java 11+ (for OpenAPI generator v7.x)

### Getting Started

1. Fork and clone the repository:
```bash
git clone https://github.com/YOUR_USERNAME/langfuse-ergonomic.git
cd langfuse-ergonomic
```

2. Set up the development environment:
```bash
# Install git hooks and verify dependencies
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

3. **Run pre-commit checks:**
```bash
# Run formatting, linting, and tests
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Or individually:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

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

**DO NOT** edit files in `langfuse-client-base/src/` directly. The langfuse-ergonomic repository uses the langfuse-client-base crate from crates.io as a dependency.

### Examples

When adding new features, please include examples:

1. Create an example file in `langfuse-ergonomic/examples/`
2. Add it to `langfuse-ergonomic/Cargo.toml`:
```toml
[[example]]
name = "your_example"
path = "examples/your_example.rs"
```
3. Test the example:
```bash
cd langfuse-ergonomic
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

For features that require API calls, add integration tests that can run with real credentials:

```rust
#[tokio::test]
async fn test_api_feature() {
    dotenv::dotenv().ok();
    let client = langfuse_ergonomic::ClientBuilder::from_env()
        .and_then(|builder| builder.build())
        .unwrap();
    // Your test
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
 src/                    # Hand-written ergonomic API
 examples/               # Usage examples  
 scripts/                # Build and setup scripts
 tests/                  # Integration tests
 .github/workflows/      # CI/CD configuration
```

The project depends on `langfuse-client-base` from crates.io, which contains the auto-generated OpenAPI client.

## Release Process

Releases are automated using [release-plz](https://release-plz.ieni.dev/):

1. Merge changes to `main`
2. release-plz creates a release PR automatically
3. Review and merge the release PR
4. Packages are published to crates.io automatically

## Getting Help

- Open an [issue](https://github.com/genai-rs/langfuse-ergonomic/issues) for bugs or feature requests
- Check existing issues before creating a new one
- Join the [Langfuse community](https://langfuse.com/docs/community) for general questions

## Code of Conduct

Please be respectful and constructive in all interactions. We aim to maintain a welcoming and inclusive community.

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
