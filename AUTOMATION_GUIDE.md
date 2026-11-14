# Automation Context Guide

This file captures the background and workflow expectations for anyone (human teammates or ***REMOVED***) working on this project.

## Repository Context

This is the **langfuse-ergonomic** repository - an ergonomic wrapper for the Langfuse API using builder patterns.

- **Depends on**: [langfuse-client-base](https://github.com/genai-rs/langfuse-client-base) - the auto-generated OpenAPI client
- **Purpose**: Provides a user-friendly API using the Bon builder pattern library

## Layout

- `.`: the `langfuse-ergonomic` library; `langfuse-ergonomic-macros/`: the `#[observe]`
  procedural macro, released together with the library at the same version.
- Depends on the published `langfuse-client-base` and `timvw-rust-runtime` crates from crates.io.
- User guides live in `docs/` and are compiled as doc tests (`doc_pages` in `src/lib.rs`);
  see [docs/development.md](docs/development.md) for tests, local CI and live tests.

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
- Branch protection rules are enforced on `main`: pull requests only, linear history (rebase
  merges), and the required CI checks

### Pre-commit Checks
- **ALWAYS run the checks before committing**:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features
  ```
- If formatting issues are found, run `cargo fmt --all` to fix them
- Before pushing, run `scripts/local-ci.sh` from genai-rs/.github; its `local-ci`
  attestation lets the PR skip the heavy CI jobs (see [docs/development.md](docs/development.md#local-ci))

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
- `LANGFUSE_API_VERSION` - `v3` or `v4` (defaults to `v4`)

## Testing

### Running Examples
The examples in `examples/` send to a real Langfuse server (credentials from the environment),
for example:
```bash
cargo run --example observe
cargo run --example basic_trace
cargo run --example http_middleware_retry --features retry
```
The live tests (`scripts/live-tests.sh`) run against a throwaway self-hosted Langfuse in
Docker; see [docs/development.md](docs/development.md#live-tests).

### CI/CD
- GitHub Actions runs on every push and PR (shared `rust-ci`, feature combinations, live tests
  for v3 and v4, security)
- release-plz creates automated release PRs
- Both crates are published to crates.io when the release PR is merged

## Implementation Status

The README's feature list and the pages under `docs/` describe what is implemented; keep them
in sync with the code.

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
4. Mention the new example in the relevant page under docs/

### Updating Documentation
- Keep README and docs/ accurate - only document implemented features
- Mark unimplemented features as "Planned"
- Test all code examples in documentation

## Important Notes

1. **Base Client**: This crate depends on langfuse-client-base from crates.io
2. **Token Scopes**: crates.io tokens must have the pattern `langfuse-*` for publishing
3. **Documentation**: docs.rs builds documentation automatically after crates.io publish
4. **Examples**: Run the examples you change against a Langfuse server before committing

## Repository Links
- GitHub: https://github.com/genai-rs/langfuse-ergonomic
- crates.io: https://crates.io/crates/langfuse-ergonomic
- docs.rs: https://docs.rs/langfuse-ergonomic
- Langfuse API docs: https://langfuse.com/docs/api
