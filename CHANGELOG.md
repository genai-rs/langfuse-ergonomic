# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.2](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.6.1...v0.6.2) - 2025-12-23

### Other

- install latest cargo-deny
- *(deps)* Update rust patch updates
- *(deps)* Update actions/cache action to v5
- *(deps)* Update Rust crate reqwest to ^0.12.25
- *(deps)* Update rust minor updates
- *(deps)* Update Rust crate reqwest-retry to ^0.8.0
- *(deps)* Update rust patch updates

## [0.6.1](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.6.0...v0.6.1) - 2025-11-24

### Added

- support queue ids and new ingestion variants

### Other

- *(deps)* Update actions/checkout action to v6
- *(deps)* Update Rust crate langfuse-client-base to ^0.6.1
- allow Cargo.lock tracking for release automation
- unblock security workflow by aligning dataset schema
- *(deps)* Update rust minor updates
- *(deps)* Update rust patch updates
- remove RENOVATE_ANALYSIS.md
- Update MSRV badge from 1.82 to 1.83
- Update MSRV from 1.82 to 1.83
- Add issues:write permission to cargo-audit workflow (genai-rs-41)
- fix Renovate strategy and improve version clarity (genai-rs-11)

## [0.6.0](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.5.0...v0.6.0) - 2025-10-12

### Other

- Remove emojis from example code
- Add reqwest-middleware support to langfuse-ergonomic
- *(deps)* Update github/codeql-action action to v4

## [0.5.0](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.4.0...v0.5.0) - 2025-10-03

### Changed

- **BREAKING**: replace JSON values with strongly-typed API responses for all read methods
  - `list_traces()` now returns `langfuse_client_base::models::Traces`
  - `get_trace()` now returns `langfuse_client_base::models::TraceWithFullDetails`
  - `get_observations()` now returns `langfuse_client_base::models::ObservationsViews`
  - `get_observation()` now returns `langfuse_client_base::models::ObservationsView`
  - `create_dataset()`, `get_dataset()`, `list_datasets()` now return typed `Dataset` structs
  - `get_dataset_run()`, `get_dataset_runs()` now return typed dataset run structs
  - `create_dataset_item()`, `get_dataset_item()`, `list_dataset_items()` now return typed item structs
  - `get_prompt()`, `list_prompts()` now return typed prompt structs

### Fixed

- update README to use comments for equivalent imports example
- mark duplicate import example as ignored in doc test

### Other

- run cargo fmt

## [0.4.0](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.3.0...v0.4.0) - 2025-10-03

### Added

- expose env builder and client batcher helper

### Fixed

- align example formatting for CI
- update jitter rng for rand 0.9

### Other

- let release-plz retrigger CI runs
- mark codecov statuses as informational
- remove emojis from documentation and scripts
- migrate to dedicated client builder
- introduce dedicated client builder
- adopt langfuse-client-base builders
- *(deps)* Update Rust crate thiserror to v2
- *(deps)* Update Rust crate rand to 0.9
- *(deps)* migrate config renovate.json5
- replace README emoji with plain text
- *(deps)* Update github-actions to v5
- align renovate config with library defaults
- rename***REMOVED*** to ***REMOVED*** guide

## [0.3.0](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.2.1...v0.3.0) - 2025-08-30

### Added

- improve CI/CD and tooling
- improve error handling with better status mapping
- improve public API ergonomics
- improve batcher with better documentation and metrics
- add stricter branch protection with quality checks
- harden CI with comprehensive testing and checks
- add dependency management with cargo-deny
- add comprehensive batch test example
- add security workflows for secrets and vulnerability scanning
- add optional compression support via feature flag
- add final polish to batch processing
- enhance batching with comprehensive improvements
- implement batching with 207 Multi-Status and auto-chunking
- add Trace URLs and BYO IDs support

### Fixed

- suppress false positive security alerts for session_id/user_id
- resolve compilation error in delete_multiple_traces
- improve error messages with context
- documentation inconsistencies and code cleanup
- remove missing_docs lint to fix CI
- remove clippy pedantic lints to fix CI
- remove deprecated unmaintained key from cargo-deny config
- add CDLA-Permissive-2.0 to allowed licenses
- resolve clippy pedantic warnings
- update rate limit test to match new error format
- resolve clippy warnings
- formatting issues
- resolve clippy unused variable warning in test
- apply cargo fmt
- resolve clippy bool_assert_comparison warning
- resolve clippy redundant_closure warning
- apply cargo fmt
- implement validate method and improve correctness
- align TLS features with reqwest dependencies
- apply cargo fmt
- clean up dependencies and use tracing instead of eprintln
- update branch protection check names to match CI output
- remove problematic MSRV and minimal-versions tests
- correct doctest compilation errors
- remove unused pretty_assertions dependency
- format code to pass CI checks
- resolve flaky test_shutdown_idempotency test
- production-ready batching improvements
- apply cargo fmt formatting
- address production-readiness feedback
- resolve clippy warnings
- apply cargo fmt formatting

### Other

- add comprehensive mock tests for API endpoints
- improve crate-level documentation
- add Renovate configuration for automated dependency updates
- add comprehensive batch processing documentation
- fix formatting issues for CI
- fix formatting issues

## [0.2.1](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.2.0...v0.2.1) - 2025-08-29

### Added

- update release-plz config to match langfuse-client-base
- add MSRV label for minimum supported Rust version tracking
- enforce linear history with rebase-only merge strategy ([#12](https://github.com/genai-rs/langfuse-ergonomic/pull/12))

### Fixed

- critical security and CI improvements
- update documentation and add missing files
- add comparison links section to PR body template
- add PR body and release body templates to release-plz config ([#11](https://github.com/genai-rs/langfuse-ergonomic/pull/11))
- update branch protection rules to include beta rust checks ([#10](https://github.com/genai-rs/langfuse-ergonomic/pull/10))

### Other

- add repository settings configuration and branch management docs

## [0.2.0](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.1.1...v0.2.0) - 2025-08-29

### Added

- Add production-ready features for batch processing ([#8](https://github.com/genai-rs/langfuse-ergonomic/pull/8))

### Other

- migrate to Bon builder pattern ([#6](https://github.com/genai-rs/langfuse-ergonomic/pull/6))

## [0.1.1](https://github.com/genai-rs/langfuse-ergonomic/compare/v0.1.0...v0.1.1) - 2025-08-29

### Added

- implement observations and scores APIs ([#5](https://github.com/genai-rs/langfuse-ergonomic/pull/5))
- Add comprehensive examples and update README
- Add working example for trace creation

### Fixed

- correct release-plz configuration syntax
- remove workspace configuration from release-plz ([#2](https://github.com/genai-rs/langfuse-ergonomic/pull/2))
- Make codecov checks informational only

### Other

- add migration context to ***REMOVED*** guide ([#4](https://github.com/genai-rs/langfuse-ergonomic/pull/4))
- convert to standalone langfuse-ergonomic repository
- Merge pull request #7 from timvw/docs/improve-readme-badges
- Merge pull request #6 from timvw/chore/configure-renovate-comprehensive
- Configure comprehensive Renovate ***REMOVED***
- Remove unused top-level docs and examples

## [0.1.0](https://github.com/timvw/langfuse-rs/releases/tag/langfuse-ergonomic-v0.1.0) - 2025-08-28

### Added

- Add automatic formatting to CI pipeline for generated code
- add code coverage with Codecov integration
- add CI/CD with release-plz and GitHub Actions
- initial implementation of langfuse-rs

### Fixed

- resolve clippy warnings in generated and ergonomic code
- update ergonomic crate to work with generated OpenAPI client
- resolve all build warnings and add from_env constructor

### Other

- add pre-commit hooks and development workflow
- enhance API documentation and references
