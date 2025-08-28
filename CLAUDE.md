# Instructions for Claude

This file contains important instructions and context for Claude when working on this project.

## Development Workflow

### Git Workflow
- **NEVER commit directly to main branch**
- Always create a feature branch first
- Create a pull request for review
- Example workflow:
  ```bash
  git checkout -b feat/your-feature-name
  # make changes
  git add -A
  git commit -m "feat: your commit message"
  git push origin feat/your-feature-name
  gh pr create --title "feat: your feature" --body "Description of changes"
  ```

### Pre-commit Checks
- **ALWAYS run pre-commit checks before committing**:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all
  ```
- If formatting issues are found, run `cargo fmt --all` to fix them

### Commit Messages
- Use conventional commits format:
  - `feat:` for new features
  - `fix:` for bug fixes
  - `docs:` for documentation only
  - `chore:` for maintenance tasks
  - `test:` for test additions/changes
- Include the co-authored-by trailer:
  ```
  🤖 Generated with Claude Code
  
  Co-Authored-By: Claude <noreply@anthropic.com>
  ```

## Project Structure

### Crates
- `langfuse-client-base/` - Auto-generated OpenAPI client (do not edit directly)
- `langfuse-ergonomic/` - User-friendly wrapper with builder patterns

### Important Files
- `scripts/generate-openapi-client.sh` - Regenerates the OpenAPI client
- `release-plz.toml` - Release automation configuration
- `.env` - Contains Langfuse API credentials (never commit this file)

## API Credentials

The project uses environment variables for Langfuse authentication:
- `LANGFUSE_PUBLIC_KEY` - Public API key
- `LANGFUSE_SECRET_KEY` - Secret API key  
- `LANGFUSE_HOST` - API endpoint (defaults to https://cloud.langfuse.com)

## Testing

### Running Examples
Always test examples with real credentials before committing:
```bash
cd langfuse-ergonomic
cargo run --example test_trace
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces
```

### CI/CD
- GitHub Actions runs on every push and PR
- Release-plz creates automated release PRs
- Packages are published to crates.io on release

## Current Implementation Status

### Implemented ✅
- Basic trace creation with builder pattern
- Environment-based configuration
- Trace metadata, tags, input/output
- Session and user tracking

### Not Yet Implemented ❌
- Observations (spans, generations, events)
- Scoring system (numeric, binary, categorical)
- Fetching existing traces
- Batch operations
- Dataset management
- Prompt management

## Common Tasks

### Adding a New Example
1. Create the example file in `langfuse-ergonomic/examples/`
2. Add entry to `langfuse-ergonomic/Cargo.toml`:
   ```toml
   [[example]]
   name = "your_example"
   path = "examples/your_example.rs"
   ```
3. Test the example
4. Update README with the new example

### Updating Documentation
- Keep README accurate - only document implemented features
- Mark unimplemented features as "Planned"
- Test all code examples in documentation

### Handling Generated Code Issues
- Clippy warnings in generated code are suppressed via `langfuse-client-base/Cargo.toml`
- Formatting is automatically applied after generation
- The generation script preserves our custom Cargo.toml

## Important Notes

1. **Generated Code**: The `langfuse-client-base` crate is auto-generated. Don't edit it directly.
2. **Token Scopes**: crates.io tokens must have the pattern `langfuse-*` for publishing
3. **Documentation**: docs.rs builds documentation automatically after crates.io publish
4. **Examples**: All examples must be tested with real API credentials before committing

## Useful Commands

```bash
# Initialize development environment
just init

# Generate OpenAPI client
just generate

# Run all checks
just check

# Create a release (via PR)
just release

# View all available commands
just --list
```

## Repository Links
- GitHub: https://github.com/timvw/langfuse-rs
- crates.io: https://crates.io/crates/langfuse-ergonomic
- docs.rs: https://docs.rs/langfuse-ergonomic
- Langfuse API docs: https://langfuse.com/docs/api