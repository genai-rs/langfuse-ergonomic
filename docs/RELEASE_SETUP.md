# Release Setup Guide

This guide explains how to set up automated releases using GitHub Actions and release-plz.

## Prerequisites

1. A crates.io account with API token
2. Repository admin access on GitHub

## Setting up GitHub Secrets

### 1. Add the CARGO_REGISTRY_TOKEN

1. Go to your repository on GitHub: https://github.com/timvw/langfuse-rs
2. Navigate to **Settings** > **Secrets and variables** > **Actions**
3. Click **New repository secret**
4. Add the following secret:
   - **Name:** `CARGO_REGISTRY_TOKEN`
   - **Value:** Your crates.io API token

**Important:** Never commit your API token to the repository!

### 2. Verify GitHub Token Permissions

The default `GITHUB_TOKEN` should have sufficient permissions. If not:
1. Go to **Settings** > **Actions** > **General**
2. Under "Workflow permissions", ensure:
   - "Read and write permissions" is selected
   - "Allow GitHub Actions to create and approve pull requests" is checked

## How It Works

### Automated Release Flow

1. **Development:** You merge changes to `main` branch
2. **Release PR:** release-plz automatically creates a PR with:
   - Version bumps in `Cargo.toml` files
   - Updated `CHANGELOG.md`
   - Conventional commit parsing
3. **Review:** You review and merge the release PR
4. **Publishing:** Upon merge, release-plz:
   - Creates GitHub releases
   - Publishes to crates.io
   - Tags the version

### Version Strategy

- The workspace uses unified versioning
- Both crates are published together
- Semantic versioning based on conventional commits:
  - `fix:` → patch version bump (0.0.X)
  - `feat:` → minor version bump (0.X.0)
  - `BREAKING CHANGE:` → major version bump (X.0.0)

## Manual Release Process

If you need to release manually:

```bash
# Update versions
cargo release version patch  # or minor, major

# Create git tag
cargo release tag

# Publish to crates.io
cargo publish -p langfuse-client-base
cargo publish -p langfuse-ergonomic

# Push tags
git push origin --tags
```

## Troubleshooting

### Release-plz PR not created
- Check the Actions tab for workflow runs
- Ensure you have changes in `main` that affect version

### Publishing fails
- Verify `CARGO_REGISTRY_TOKEN` is set correctly
- Check that crate names are available on crates.io
- Ensure all dependencies are published

### Version conflicts
- Check that version numbers in Cargo.toml files are consistent
- Ensure no version is already published on crates.io

## Security Notes

- API tokens are sensitive - treat them like passwords
- Use GitHub Secrets, never commit tokens
- Rotate tokens periodically
- Revoke tokens if compromised