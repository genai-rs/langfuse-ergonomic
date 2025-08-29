# Instructions for***REMOVED***

This file contains important instructions and context for***REMOVED*** when working on this project.

## Repository Context

This is the **langfuse-ergonomic** repository - an ergonomic wrapper for the Langfuse API using builder patterns.

- **Depends on**: [langfuse-client-base](https://github.com/genai-rs/langfuse-client-base) - the auto-generated OpenAPI client
- **Purpose**: Provides a user-friendly API using the Bon builder pattern library

## Recent Migration (2025-08-29)

This repository was migrated from the monorepo at timvw/langfuse-rs to a standalone repository in the genai-rs organization. Key changes:

1. **Standalone Repository**: Converted from workspace member to standalone package
2. **Dependencies**: Now depends on published langfuse-client-base crate from crates.io (v0.1)
3. **Simplified Configuration**: Removed workspace configuration from release-plz.toml
4. **Structure**: All code moved from subdirectory to repository root
5. **CI/CD**: Retained full test matrix as this contains manually written code

### Migration Notes
- Repository is still technically a fork but has full permissions via organization settings
- Observations and scores modules are temporarily disabled (commented out in lib.rs) pending API updates
- Only traces module is currently active and tested

## Development Workflow

### Git Workflow
- **NEVER commit directly to main branch**
- Always create a feature branch first
- Create a pull request for review
- **IMPORTANT**: Before creating PRs, verify remotes are correct:
  ```bash
  git remote -v  # Should only show 'origin' pointing to genai-rs/langfuse-ergonomic
  ```
- If there are incorrect remotes (like 'upstream'), remove them:
  ```bash
  git remote remove upstream
  ```
- Example workflow:
  ```bash
  git checkout -b feat/your-feature-name
  # make changes
  git add -A
  git commit -m "feat: your commit message"
  git push --set-upstream origin feat/your-feature-name
  gh pr create --title "feat: your feature" --body "Description of changes"
  ```
- If `gh pr create` fails, use the web URL provided by git push to create the PR manually
- **Branches are automatically deleted after merge** (configured in repository settings)

### Branch Management
- The repository is configured to automatically delete branches after PR merge
- Branch protection rules are enforced on `main`:
  - Required status checks must pass (CI tests, security audit)
  - Branches must be up to date before merging
- Repository settings are documented in `.github/settings.yml`

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

## API Credentials

The project uses environment variables for Langfuse authentication:
- `LANGFUSE_PUBLIC_KEY` - Public API key
- `LANGFUSE_SECRET_KEY` - Secret API key  
- `LANGFUSE_BASE_URL` - API endpoint (defaults to https://cloud.langfuse.com)

## Testing

### Running Examples
Always test examples with real credentials before committing:
```bash
cargo run --example test_trace
cargo run --example basic_trace
cargo run --example trace_with_metadata
cargo run --example multiple_traces
```

### CI/CD
- GitHub Actions runs on every push and PR
- release-plz creates automated release PRs
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
1. Create the example file in `examples/`
2. Add entry to `Cargo.toml`:
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

## Important Notes

1. **Base Client**: This crate depends on langfuse-client-base from crates.io
2. **Token Scopes**: crates.io tokens must have the pattern `langfuse-*` for publishing
3. **Documentation**: docs.rs builds documentation automatically after crates.io publish
4. **Examples**: All examples must be tested with real API credentials before committing

## Repository Links
- GitHub: https://github.com/genai-rs/langfuse-ergonomic
- crates.io: https://crates.io/crates/langfuse-ergonomic
- docs.rs: https://docs.rs/langfuse-ergonomic
- Langfuse API docs: https://langfuse.com/docs/api