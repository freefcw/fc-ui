# Canonical command entry points for local development and CI.
# GitHub Actions uses gpui/runtime_shaders on hosted macOS runners because the
# default GPUI Metal shader compilation path can fail when the Metal Toolchain
# is unavailable.

cargo := env_var_or_default('CARGO', 'cargo')
runtime_shaders_feature := env_var_or_default('RUNTIME_SHADERS_FEATURE', 'gpui/runtime_shaders')

default:
    @just --list

build:
    {{ cargo }} build

test:
    # `--all-features` so feature-gated suites (editor, markdown, html-render,
    # audio, qrcode) actually run; a default-feature `cargo test` silently
    # skips ~35 of them.
    {{ cargo }} test --all-features

clippy:
    {{ cargo }} clippy --all-targets

fmt:
    {{ cargo }} fmt

run-example example='':
    #!/usr/bin/env bash
    set -euo pipefail

    example="{{ example }}"
    if [ -z "$example" ]; then
      example="${EXAMPLE:-}"
    fi

    if [ -z "$example" ]; then
      echo "example is required, e.g. just run-example demo" >&2
      exit 1
    fi

    {{ cargo }} run --example "$example"

list-examples:
    #!/usr/bin/env bash
    set -euo pipefail

    find examples -maxdepth 1 -name '*.rs' -print \
      | sed 's#^examples/##; s#\.rs$##' \
      | sort

build-ci:
    {{ cargo }} build --locked --features {{ runtime_shaders_feature }}

test-ci:
    # Mirror `just test` (--all-features) so the CI job named "test" is the full
    # suite; the capability-contract job only exercises the lib target.
    {{ cargo }} test --locked --all-features --features {{ runtime_shaders_feature }}

clippy-ci:
    {{ cargo }} clippy --locked --all-targets --features {{ runtime_shaders_feature }}

# Strict clippy exists as the future hard gate, but CI uses clippy-ci while
# historical warnings are being paid down.
clippy-strict:
    {{ cargo }} clippy --locked --all-targets --features {{ runtime_shaders_feature }} -- -D warnings

fmt-check:
    {{ cargo }} fmt --all --check

check-boundaries:
    python3 -m unittest discover -s scripts/tests -p 'test_check_boundaries.py'
    python3 scripts/check_boundaries.py

check-cargo-contract:
    python3 -m unittest discover -s scripts/tests -p 'test_check_cargo_contract.py'
    python3 scripts/check_cargo_contract.py

update-cargo-contract:
    python3 scripts/check_cargo_contract.py --update

check-platform-ci:
    {{ cargo }} check --locked --lib --no-default-features --features accessibility

visual-smoke-ci:
    {{ cargo }} test --locked --test real_visual_smoke --features {{ runtime_shaders_feature }} -- --ignored

verify-capabilities-ci:
    {{ cargo }} test --locked --no-default-features --features {{ runtime_shaders_feature }} --test public_api
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},markdown --example markdown_demo
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},html-render --example html_demo
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},audio --example audio_player_demo
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},qrcode --example components_showcase
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},editor --example editor_scroll_test
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},editor-languages --example editor_demo
    {{ cargo }} check --locked --no-default-features --features {{ runtime_shaders_feature }},bundled-fonts-inter-minimal,bundled-fonts-mono --lib
    {{ cargo }} test --locked --all-features --features {{ runtime_shaders_feature }} --lib --test public_api
    {{ cargo }} check --locked --all-features --features {{ runtime_shaders_feature }} --examples

validate-agent-docs:
    python3 -m unittest discover -s scripts/tests -p 'test_validate_agent_docs.py'
    python3 scripts/validate_agent_docs.py

check-clippy-policy:
    #!/usr/bin/env bash
    set -euo pipefail

    summary="$(just --summary | tr ' ' '\n')"

    grep -Fq -- "target: clippy-ci" .github/workflows/ci.yml
    grep -Fx 'clippy-ci' <<<"$summary" >/dev/null
    grep -Fx 'clippy-strict' <<<"$summary" >/dev/null

    if grep -Fq -- "target: clippy-strict" .github/workflows/ci.yml; then
      echo "CI should continue to use the staged clippy-ci target until the warning debt is cleaned up."
      exit 1
    fi
