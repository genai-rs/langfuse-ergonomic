# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of Langfuse Rust client
- Auto-generated base client from OpenAPI specification
- Ergonomic wrapper with builder patterns using Bon
- Support for traces, observations (spans/generations/events), and scores
- Environment-based client initialization with `from_env()`
- Comprehensive examples and documentation
- CI/CD with GitHub Actions and release-plz
- Pre-commit hooks for code quality

### Fixed
- Resolved all build warnings
- Added proper lifetime annotations

### Developer Experience
- Added git pre-commit hooks
- Integrated cargo fmt and clippy checks
- Created justfile for common tasks