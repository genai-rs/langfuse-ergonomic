# List available recipes
default:
    @just --list

# Install dependencies and setup environment
init:
    rustup update
    cargo --version
    @echo "Checking for openapi-generator-cli..."
    @which openapi-generator-cli > /dev/null || npm install -g @openapitools/openapi-generator-cli
    @echo "Setting up git hooks..."
    ./scripts/setup-hooks.sh
    @echo "✅ Development environment ready!"

# Generate OpenAPI client
generate:
    ./scripts/generate-openapi-client.sh

# Build all crates
build:
    cargo build --all

# Run all tests
test:
    cargo test --all

# Run linting checks
lint:
    cargo clippy --all -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check code formatting without changing files
fmt-check:
    cargo fmt --all -- --check

# Run all checks (format, lint, test)
check: fmt-check lint test
    cargo check --all

# Run pre-commit checks (format, clippy, build, test)
pre-commit:
    @echo "🔍 Running pre-commit checks..."
    @echo "📝 Formatting code..."
    cargo fmt --all
    @echo "🔎 Running clippy..."
    cargo clippy --all -- -D warnings
    @echo "🔨 Building..."
    cargo build --all
    @echo "🧪 Running tests..."
    cargo test --all
    @echo "✅ All pre-commit checks passed!"

# Run a specific example
run-example name:
    cargo run --example {{name}}

# Run the basic usage example
example-basic:
    cargo run --example basic_usage

# Run the LLM chain example
example-chain:
    cargo run --example llm_chain

# Clean build artifacts
clean:
    cargo clean
    rm -f scripts/openapi.yml

# Generate documentation
doc:
    cargo doc --all --no-deps --open

# Publish to crates.io (dry run)
publish-dry:
    cargo publish --dry-run -p langfuse-client-base
    cargo publish --dry-run -p langfuse-ergonomic

# Watch for changes and run checks
watch:
    cargo watch -x check -x test

# Generate code coverage report
coverage:
    @echo "🔍 Generating code coverage..."
    cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out html --out xml --exclude-files "*/tests/*" --exclude-files "*/examples/*"
    @echo "📊 Coverage report generated: tarpaulin-report.html"
    @echo "📊 Coverage XML generated: cobertura.xml"

# Generate coverage and open in browser
coverage-html: coverage
    @open tarpaulin-report.html || xdg-open tarpaulin-report.html 2>/dev/null || echo "Please open tarpaulin-report.html manually"