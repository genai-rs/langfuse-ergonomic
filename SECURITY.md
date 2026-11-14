# Security Policy

## Supported Versions

Security fixes go into the latest release (see the
[crates.io badge](https://crates.io/crates/langfuse-ergonomic)); older releases are not
patched.

## Reporting a Vulnerability

If you discover a security vulnerability, please:

1. **DO NOT** create a public GitHub issue
2. Email the details to the maintainer (see GitHub profile)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

## Security Updates

- Security fixes are given highest priority
- Patches will be released as soon as possible
- Renovate bot automatically creates PRs for dependency security updates

## Best Practices

When using this library:

- **Never commit API keys** to version control
- Use environment variables for credentials
- Rotate API keys regularly
- Use the minimum required permissions for API keys
- Review dependencies regularly for vulnerabilities

## Dependencies

This project uses:
- Automated dependency updates via Renovate
- Security scanning in CI (the shared `rust-security` workflow and TruffleHog)
- `cargo audit` for Rust dependency vulnerabilities

Run security audit locally:
```bash
cargo install cargo-audit
cargo audit
```