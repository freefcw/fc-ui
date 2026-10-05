# AGENTS.md

## Project Summary

fc-ui (crate root `adabraka_ui`, repository `freefcw/fc-ui`) is a Rust GPUI component library with reusable UI primitives, demos, and docs. It builds against `fc-gpui`, imported as `gpui`.

## Local Commands

- `just build`
- `just test`
- `just clippy`
- `just fmt`
- `just run-example demo`
- `just run-example slider_styled_demo`
- `just list-examples`

`just test` runs `cargo test --all-features` so that feature-gated suites (`editor`, `markdown`, `html-render`, `audio`, `qrcode`) are actually exercised; a bare `cargo test` on default features silently skips them. `just test-ci` mirrors this for CI (`--locked`, plus the `gpui/runtime_shaders` feature for hosted macOS runners).

## GitHub Actions Commands

- `just build-ci`
- `just test-ci`
- `just clippy-ci`
- `just check-platform-ci`
- `just visual-smoke-ci`
- `just fmt-check`
- `just clippy-strict` (future hard gate, not currently wired into CI)
- `just check-boundaries`
- `just check-cargo-contract`
- `just verify-capabilities-ci`

GitHub Actions uses `gpui/runtime_shaders` on hosted macOS runners because the default GPUI Metal shader compilation path can fail when the Metal Toolchain is unavailable. CI Cargo targets use `--locked`, and `just check-platform-ci` verifies the accessibility-enabled library on Linux and Windows.

The `Justfile` is the canonical command source for local development and CI targets.

`just verify-capabilities-ci` validates the public API contract both without default features and with all package features enabled.

## Working Rules

- Keep changes small and scoped to the requested task.
- Do not delete, overwrite, or clean untracked files unless explicitly asked.
- Follow existing patterns in `src/` and `examples/`.
- Run `just fmt`, `just clippy`, and `just test` before finishing code changes.
- If you change build, test, lint, or format commands, update `Justfile`, `AGENTS.md`, `CLAUDE.md`, and `.github/workflows/ci.yml` together.
- The `agents-md` CI job validates that command docs, the Justfile, and workflow commands stay aligned.
- When intentionally changing a Cargo feature or example target, run `just update-cargo-contract`, review `scripts/cargo_contract.json`, then run `just check-cargo-contract`.

## Repository Notes

- Canonical implementation ownership lives under `src/capabilities/`; `layout` owns generic composition while `scroll` owns scrolling and virtualization.
- `components`, `display`, `charts`, `navigation`, `overlays`, and root utility modules are compatibility facades; preserve their public paths.
- Some components require initialization; `src/lib.rs` owns the ordered root sequence and capability modules own their individual initializers.
- Examples follow the `<component>_styled_demo.rs` naming pattern.
