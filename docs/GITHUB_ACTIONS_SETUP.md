# GitHub Actions Setup for release-plz

## Repository Settings Required

For the release-plz workflow to create pull requests, you need to configure your repository settings:

### Option 1: Enable GitHub Actions to Create PRs (Recommended)

1. Go to your repository settings: https://github.com/timvw/langfuse-rs/settings/actions
2. Scroll down to "Workflow permissions"
3. Select "Read and write permissions"
4. Check the box "Allow GitHub Actions to create and approve pull requests"
5. Click "Save"

### Option 2: Use a Personal Access Token (PAT)

If Option 1 doesn't work or you prefer more control:

1. Create a Personal Access Token:
   - Go to https://github.com/settings/tokens/new
   - Give it a descriptive name like "release-plz-langfuse-rs"
   - Select these scopes:
     - `repo` (full control of private repositories)
     - `workflow` (update GitHub Action workflows)
   - Click "Generate token"
   - Copy the token (it won't be shown again!)

2. Add the token as a repository secret:
   - Go to https://github.com/timvw/langfuse-rs/settings/secrets/actions
   - Click "New repository secret"
   - Name: `RELEASE_PLZ_TOKEN`
   - Value: [paste your PAT]
   - Click "Add secret"

3. Update the workflow to use the PAT:
   ```yaml
   - name: Run release-plz
     uses: MarcoIeni/release-plz-action@v0.5
     env:
       GITHUB_TOKEN: ${{ secrets.RELEASE_PLZ_TOKEN }}
       CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
   ```

## Troubleshooting

### Error: "GitHub Actions is not permitted to create or approve pull requests"

This error occurs when GitHub Actions doesn't have permission to create PRs. Solutions:

1. **Check repository settings**: Ensure you've enabled "Allow GitHub Actions to create and approve pull requests" as described in Option 1 above.

2. **Use a PAT**: If the repository settings don't work, create a Personal Access Token as described in Option 2.

3. **Check organization settings**: If your repository is in an organization, the organization may have policies that prevent GitHub Actions from creating PRs. Check with your organization admins.

### Error: "Workflow permissions insufficient"

Ensure your workflow has the correct permissions:

```yaml
permissions:
  contents: write
  pull-requests: write
```

### Release-plz doesn't create a PR

If release-plz runs successfully but doesn't create a PR, it might be because:

1. **No changes to release**: There are no conventional commits since the last release
2. **Already up to date**: The versions are already correct
3. **Configuration issue**: Check your `release-plz.toml` configuration

Run release-plz locally to debug:
```bash
# Install release-plz
cargo install release-plz

# Check what changes would be made
release-plz release-pr --dry-run

# See detailed output
release-plz release-pr -v
```

## Current Status

The workflow is configured to use the default `GITHUB_TOKEN`. If you continue to see permission errors, you'll need to either:
1. Verify the repository settings are correctly configured (Option 1)
2. Create and use a Personal Access Token (Option 2)