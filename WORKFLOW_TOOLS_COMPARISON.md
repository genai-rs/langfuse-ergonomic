# Detailed Workflow Tools Comparison

Comparing tools and workflows between `langfuse-ergonomic` and `langfuse-client-base` repositories.

## Workflow Files Overview

| Workflow | langfuse-ergonomic | langfuse-client-base | Purpose |
|----------|-------------------|---------------------|---------|
| ci.yml | ✅ | ✅ | Main CI pipeline for PRs and pushes |
| dependencies.yml | ✅ | ✅ | Check for unused dependencies |
| security.yml | ✅ | ✅ | Comprehensive security scanning |
| secrets.yml | ✅ | ✅ | Scan for leaked secrets |
| release-plz.yml | ✅ | ✅ | Automated releases |
| generate-client.yml | ❌ | ✅ | Auto-generate from OpenAPI (base only) |

## Security Tools Distribution

Let me check where each tool is used...