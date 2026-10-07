# Capability Architecture

## Purpose

fc-ui is a single Rust package with capability-oriented internal ownership. The structure keeps implementation dependencies clear without breaking the public paths used by existing applications.

The canonical implementation lives under `src/capabilities/`. Public root and category modules remain compatibility facades.

## Capability Owners

The current owners are:

- `foundation`: shared theme, initialization, utilities, fonts, icons, and infrastructure;
- `motion`: animation and transition behavior;
- `primitives`: low-level reusable UI building blocks;
- `layout`: generic composition and layout;
- `scroll`: scrolling and virtualization;
- `overlays`: dialogs, popovers, sheets, and related layers;
- `controls`: inputs and interactive controls;
- `navigation`: menus, tabs, toolbars, and navigation structures;
- `data`: data display and chart-oriented components;
- `content`: rich content and document presentation;
- `editor`: editor implementation and editor-specific parsing;
- `media`: image, audio, and video components;
- `effects`: reusable visual effects.

When ownership is unclear, prefer the capability that owns the component's behavior rather than the legacy public category where it was historically exported.

## Canonical Code And Compatibility Facades

Canonical production code must import another owner through:

```rust
crate::capabilities::<owner>
```

It must not import through public compatibility modules such as `components`, `display`, `navigation`, `overlays`, `charts`, or root utility facades. This prevents internal code from depending on paths that exist only to preserve downstream compatibility.

Existing public paths remain supported through re-exports. Moving implementation ownership must not force downstream applications to change imports.

The exact allowed dependency edges are defined and checked in `scripts/check_boundaries.py`. That file is the source of truth for the dependency matrix.

## Important Ownership Rules

- `layout` owns generic composition; `scroll` owns scrolling and virtualization.
- Compatibility facades may re-export canonical implementations, but canonical implementations may not depend on those facades.
- Cross-owner production imports use absolute `crate::capabilities::...` paths rather than parent-relative `super::` paths.
- Keep optional features and their public entry points available unless an explicit breaking change is approved.
- Preserve `src/prelude.rs` compatibility when moving implementation files.

## Initialization

`fc_ui::init(cx)` remains the one-call public entry point. Capability modules own their registrations, while `src/lib.rs` coordinates the root order.

The current order is intentional. GPUI resolves same-depth keybinding conflicts in reverse registration order, so controls and media initialization remain interleaved. Do not consolidate or reorder these calls without tests proving that registration order is unobservable.

Initialization is guarded per application. Both root initialization and individual capability initialization must remain idempotent.

## Making A Capability Change

When adding or moving implementation:

1. choose the canonical capability owner;
2. place production implementation under `src/capabilities/<owner>/`;
3. preserve existing public paths with compatibility re-exports;
4. use canonical capability imports internally;
5. update the allowed dependency matrix only when the new edge is intentional;
6. update Cargo feature/example contracts when a feature or example target changes;
7. keep initialization ownership and root ordering explicit.

Do not create a new capability only to shorten a directory. A new owner should represent a durable behavior boundary with a clear dependency direction.

## Verification

Run the normal project checks:

```bash
just fmt
just clippy
just test
```

For architecture or capability changes, also run:

```bash
just check-boundaries
just check-cargo-contract
just verify-capabilities-ci
```

If build, test, lint, or format commands change, update `Justfile`, `AGENTS.md`, `CLAUDE.md`, and `.github/workflows/ci.yml` together.

When a Cargo feature or example target changes intentionally, run `just update-cargo-contract`, review `scripts/cargo_contract.json`, and then run `just check-cargo-contract`.
