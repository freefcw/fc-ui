# CLAUDE.md

## Build & Development Commands

```bash
# Local build
just build

# Local tests (runs `cargo test --all-features`)
just test

# Local lint
just clippy

# Format
just fmt

# GitHub Actions build/test/lint path
just build-ci
just test-ci
just clippy-ci
just check-platform-ci
just visual-smoke-ci
just fmt-check

# Future strict clippy gate
just clippy-strict

# Capability architecture contracts
just check-boundaries
just check-cargo-contract
just verify-capabilities-ci

# After an intentional Cargo feature or example-target change
just update-cargo-contract

# Run a specific example
just run-example <example_name>

# List available examples
just list-examples

# Common examples
just run-example demo
just run-example slider_styled_demo
```

GitHub Actions uses `gpui/runtime_shaders` on hosted macOS runners because the default GPUI Metal shader compilation path can fail when the Metal Toolchain is unavailable. CI Cargo targets use `--locked`, and `just check-platform-ci` verifies the accessibility-enabled library on Linux and Windows. `just clippy-strict` exists as the future hard gate, but CI currently uses `just clippy-ci` while the existing clippy debt is being paid down.

`just test` passes `--all-features`, because a default-feature `cargo test` silently skips the feature-gated `editor`, `markdown`, `html-render`, `audio`, and `qrcode` suites. `just test-ci` mirrors this for CI (`--locked`, plus the `gpui/runtime_shaders` feature).

The `Justfile` is the canonical command source for local development and CI targets.

`just verify-capabilities-ci` validates the public API contract both without default features and with all package features enabled.

## Release

Pushing a `v*` tag runs `.github/workflows/release.yml`: it verifies the tag matches the `Cargo.toml` package version, runs `just test-ci`, publishes to crates.io with the `CARGO_REGISTRY_TOKEN` repository secret, and creates a GitHub Release. Before tagging, bump `version` in `Cargo.toml`, finalize the CHANGELOG `[Unreleased]` section, and commit as `release: prepare fc-ui vX.Y.Z`.
