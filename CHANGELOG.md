# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Form submission now receives Enter from single-line `Input` controls while multiline `Textarea` keeps Enter for newline insertion.
- `ToastManager::add_toast` and `add_toast_no_dismiss` return the manager-assigned toast id; `Select` and `Combobox` expose dedicated clear callbacks instead of reporting the removed value as a new selection.
- Data-table numeric sorting and virtual-data resets clear stale index-based row selections; cell-edit blur behavior now matches `use_edit_dialog` documentation.
- Tag input backspace edits the pending text by grapheme before removing the last committed tag, and countdown completion uses the window that rendered the countdown.
- Bar charts now render finite positive and negative values around an explicit zero axis; non-finite values are treated as zero, and stacked charts keep positive and negative accumulations separate.
- The GPUI dependency now uses the published `fc-gpui` 0.9.2 crates.io release. The uniform and variable virtual lists snap self-computed padding to device pixels, matching the upstream `clamp_scroll_position` fix for phantom scrollbars on fractionally padded containers.
- Physics-driven scrolling (`ScrollContainer`, virtual lists with `.with_physics(...)`) is now clamped to the real scroll range instead of drifting unbounded past the end; `overscroll` bounce may now transiently engage at both ends, and reaching the content end pulls the position back instead of coasting into a dead zone.
- `InputState` focus handling is wired end to end: `InputEvent::Focus`/`Blur`, focus/blur callbacks, `select_on_focus`, `trim_on_blur` and `validate_on_blur` (the latter two on by default) now actually run on real focus transitions. Blurring an input may therefore trim its content and surface validation messages where it previously did nothing.
- `Input::new(&state)` no longer overwrites the state's placeholder with an empty string; an unset builder placeholder leaves `InputState`'s own placeholder in place.
- Form submission listens to registered single-line input `Enter` events instead of a bare key listener or action bubbling: Enter submits from single-line inputs while multi-line `Textarea` keeps inserting newlines. Enter while focus is on a control with no enter binding no longer submits.
- `Form` no longer depends on Enter action bubbling; registered fields emit the submit path directly, so an input `on_enter` callback and the form submit callback each run once.
- `Sidebar` no longer binds `cmd-b`/`ctrl-b` globally. Applications that relied on the global binding should bind `ToggleSidebar` themselves.
- Data tables clear the row selection when sorting or replacing data (selection was index-based and drifted onto the wrong rows), and sort numeric-looking strings numerically. `use_edit_dialog` now defaults to `false` (blur saves the edit) and its documentation matches the implemented behavior; the name is retained for API compatibility.
- Data tables expose window-aware `set_data_with_window` and `virtual_reset_with_window` variants so programmatic data replacement can notify selection observers after stale index-based selections are cleared.
- Slider, range slider, audio/video player progress and volume tracks, and the crop area now reset their drag states on mouse-up outside the element, and slider/range slider support arrow-key/Home/End adjustment when focused. Checkbox and toggle respond to Space only, per ARIA.
- `ToastManager` assigns toast ids internally (a caller-supplied id is overwritten). `split_pane` emits a final `Resized` event when a drag ends. `ImageViewerState` mutators (`set_images`, `next`, `prev`, `zoom_in`, `zoom_out`, `reset_zoom`, `set_zoom`, `toggle_thumbnails`, `go_to`) take `&mut Context<Self>` and notify observers; the viewer only grabs focus once when it opens.
- Video player fullscreen goes through the window's `toggle_fullscreen`, and control-bar auto-hide runs on an internal timer. Audio player `stop()` pauses and seeks to the start so playback can resume; seeking the progress bar calls the audio backend.
- Video player auto-hide tasks are armed by interaction and invalidated by later interaction or pause/stop, rather than polling permanently every 100ms.
- Overlays that open now move focus into the overlay and restore the previously focused element on close (`Dialog`, `AlertDialog`, `Sheet`); `AlertDialog` and `Sheet` respond to Escape. Several decorative effect and animation identifiers are now per-instance, so multiple instances on one screen no longer share animation state.
- Text input `max_length`/character counting is character-based rather than byte-based, matching the validation messages.
- `just test` (and its CI mirror `just test-ci`) now runs `cargo test --all-features`, so the feature-gated `editor`, `markdown`, `html-render`, `audio`, and `qrcode` suites are actually executed; a default-feature `cargo test` silently skipped ~35 tests. The CI job named "test" is now the full suite instead of relying on the capability-contract job's lib-only run.
- `just run-example <name>` forwards extra arguments to cargo, so feature-gated examples are runnable: `just run-example editor_demo --features editor,editor-languages`.
- Theme documentation no longer claims "automatic light/dark mode support": all 18 presets are installed explicitly via `theme::install_theme` and the library does not track the system appearance. The README theme list now covers all 18 presets instead of only light/dark.

### Fixed
- Editor: `min_lines`/`max_lines` and line-based horizontal wheel scrolling used a hardcoded 20px line height, so an editor resized via `set_font_size` (which sets `line_height = size * 1.5`) computed the wrong height and scroll step.
- `SvgRenderer`'s path parser treated unsupported commands (`A`, `S`, `T`) as unknown characters and nibbled through their arguments one character at a time, silently swallowing the command that followed them too; unsupported commands now skip their arguments cleanly and the next command still parses.
- `Input`/`Textarea` caret and selection quads were hardcoded blue (`0x0066ff`/`0x3311ff30`) and ignored the theme; they now use `theme.tokens.primary` (`opacity(0.25)` for the selection) like `MentionInput`, `InlineEdit`, and `Editor` already did.
- Editor: cursor movement stays on UTF-8 character boundaries (left/right arrows no longer panic on CJK text and Enter/typing after moving through CJK is safe), stale search matches can no longer panic replace-current/replace-all, and paste clears the IME marked range.
- Text input: the IME `new_selected_range` mapping no longer targets the wrong byte offset (fixed in input, textarea, mention input, and inline edit); `MentionInput` tracks IME marked text properly so composing CJK text replaces instead of appending; disabled inputs ignore IME edits; mention triggers work for multi-byte characters; `TextField` no longer slices UTF-16 ranges as byte offsets.
- `Alert` success/warning/error default icons referenced non-existent asset names (`check-circle` etc.) and rendered blank; corrected to the bundled `circle-check`/`triangle-alert`/`circle-alert` assets, along with the same wrong names in `Timeline`, `Toast`, `NotificationCenter`, and demos.
- `ColorPicker` treated hue degrees as GPUI's 0..1 turns, so most default swatches rendered red and HEX/RGB readouts were wrong; hue is now handled consistently in turns with ground-truth color tests.
- `DatePicker`'s "today" was hardcoded to 2025-01-23 and `disable_weekends()` was a no-op; both now work.
- `ToggleGroup` multiple mode never fired `on_multiple_change`; `Rating` with `allows_half` could not select full stars by mouse; `Slider`/`RangeSlider` `set_min`/`set_max` could panic on inverted bounds; `Select`/`Combobox` clear buttons reopened the dropdown and skipped the change callback, and their search fields rejected spaces and multi-byte input (now uses `key_char`).
- `DropZone`'s `on_drop` callback was never invoked; `TagInput` had no working text input path; `Carousel` auto-play spawned a new timer per render; `SortableList` left items stuck semi-transparent when a drag ended outside the list.
- Overlays: `Sheet`/`CommandPalette`/`ContextMenu` closed when clicking inside their panels; `Popover` leaked a window subscription per opening; `ContextMenu` ignored item icons; `Tooltip`'s right placement anchored to the wrong corner; `Dialog`'s `on_close` ran during render.
- Navigation: non-ASCII labels panicked the tree filter highlight (byte offsets used as char indices); `Dock` magnification compared mixed coordinate systems and never engaged; `SegmentedNav`'s sliding indicator was a no-op identity; `ViewRouter` and drawer timers raced on rapid transitions; `MenuBar` never rendered its dropdown; `Pagination` mishandled out-of-range current pages.
- Data: `Table` panicked when a row had more cells than columns; dragging a column-resize handle also re-sorted the column; `Countdown`'s `on_complete` fired on every render after completion; bar/heatmap charts now guard non-finite values; pie/donut legend percentages sum to 100.
- Content: nested HTML lists duplicated their text, nested Markdown lists lost their structure and inline images escaped their paragraph, and HTML inline whitespace is collapsed.
- Motion: the `spring` easing curve started at 1.0 instead of 0.0; overshoot easings are no longer clamped to 0..1; `ContentTransition` can replay (pass `.key(...)`); `Transition` default ids are unique per call site; gesture long-press no longer emits a trailing tap; scroll physics decay is frame-rate independent.
- Effects: `MagneticButton`/`TiltCard`/`Spotlight` used window-absolute coordinates and threw content off screen — they now position relative to the element; `Confetti` stacked tick loops on repeat bursts; `Meteors` ran ~6x slower than designed; several zero-division hangs and NaN layout inputs are clamped.
- Media: `SvgRenderer` drew all curve commands incorrectly (cubic commands went through the quadratic path with parameters swapped); `FileUpload`'s drop zone actually accepts dragged files and single-selection mode replaces instead of appending; audio time display rolls over into hours.
- Layout: `Justify::Evenly` silently degraded to space-around instead of real space-evenly; `Grid`'s last row no longer stretches cells; foundation: `install_theme` now repaints windows, font registration failures warn instead of panicking, and the `z_sticky`/`z_dropdown` token order is corrected.

### Added
- `SvgRenderer` now understands relative path commands (`m l h v c q z`), resolving them against the current point while parsing; after a close, the current point returns to the start of the subpath, so the `z m…` continuation emitted by SVG optimizers lands where the spec says it does.

### Deprecated
- `TextField` (use `Input`), the legacy `VirtualList` in `capabilities::scroll::variable_virtual_list` (use `uniform_virtual_list`), the crate-local `Spring` (use GPUI's `SpringAnimation`/`with_spring`), `Button::tooltip` (wrap with `overlays::tooltip`), `Label::for_id`, the `icon_button()` free function (use `IconButton`), and `ScrollContainer::no_auto_size()` (no-op).

### Removed
- `capabilities::controls::text_field` (`TextField`, `TextFieldState`, `TextFieldSize`) — deprecated as incomplete and never constructed anywhere in the crate; use `Input` instead.
- `capabilities::scroll::variable_virtual_list` — every public item was deprecated and it had no callers. The `adabraka_ui::navigation::virtual_list` path is kept and now aliases `uniform_virtual_list`, but `VirtualList`/`v_virtual_list` and the module path itself are gone.
- The editor's internal `VerticalScrollbar`, which carried `#[allow(dead_code)]` because only `HorizontalScrollbar` was ever constructed.

## [0.8.1] - 2026-09-05

### Added
- Re-exported GPUI 0.9's `container_query` element constructor and its `ContainerQuery` element type from `gpui_ext` (and therefore the prelude), so size-aware subtrees can be built without depending on `gpui` directly.
- Re-exported GPUI 0.9's analytic spring API from `gpui_ext`: `SpringConfig`, `SpringState`, `SpringAnimation`, `SpringPlayback`, `SpringTarget`, `AnimationPhase`, `Interpolate`, and `sampled_easing`. These make the already re-exported `AnimationExt::with_spring` usable and let `sampled_easing` feed `Animation::with_easing`.
- Re-exported GPUI 0.9's `QuitMode` from `gpui_ext`, which the already re-exported `App::set_quit_mode` and `Application::with_quit_mode` require.

### Changed
- Renamed the published package from `adabraka-ui` to `fc-ui`. The crate root stays `adabraka_ui` via an explicit `[lib] name`, so `use adabraka_ui::*;` is unchanged. `repository` and `homepage` now point at `https://github.com/freefcw/fc-ui` (the GitHub repository was renamed from `freefcw/adabraka-ui`, which still redirects) and `documentation` points at `https://docs.rs/fc-ui`. The crates.io package named `adabraka-ui` remains the unrelated upstream Augani 0.3.9 release line.
- Renamed the GPUI dependency from `adabraka-gpui` to `fc-gpui`, now resolved from crates.io as `fc-gpui` 0.9.0 rather than a git revision pin. The import name stays `gpui`.
- Updated the `adabraka-gpui` dependency baseline from v0.8.1 (`63fb8d8`) to the 0.9 development line at `237df3455c9ce10d4fc04b7cac4b9650ca27ad97`, which also moves the layout engine to Taffy 0.13.
- Raised `rust-version` from 1.85 to 1.90, the floor that fc-gpui 0.9 and its dependency tree (`ordered-float` 5.5, `cosmic-text` 0.19) actually require. The `fc-ui` 0.8.0 release declared 1.97.1, which only mirrored fc-gpui's contributor `rust-toolchain.toml` pin rather than a real consumer requirement; building against crates.io `fc-gpui` 0.9.0 on rustc 1.90.0 was verified on 2026-09-05. Contributor tooling may still pin a newer toolchain. The crate itself still uses edition 2021.

### Migration Notes
- Downstream crates should depend on `fc-ui` instead of `adabraka-ui` and on `gpui = { package = "fc-gpui", version = "0.9.2" }` instead of `adabraka-gpui`. No `use` statements change, since both crate roots keep their previous names.
- `Window::blur` and `Window::disable_focus` now take `&mut App` in GPUI 0.9, so blurring also clears pending keystrokes and keybinding-indicator observers. Applications calling these directly must pass the app context, for example `window.blur(cx)`.
- GPUI 0.9 removed `App::set_keep_alive_without_windows`. Applications that kept themselves alive without windows should use `App::set_quit_mode` or `Application::with_quit_mode` with `QuitMode::Explicit` (previously `true`) or `QuitMode::LastWindowClosed` (previously `false`). On macOS, `QuitMode::Default` now keeps the app alive after the last window closes.
- `GpuResourceBudget` gained a required `instance_buffer_max_size` field, so struct literals that listed only the atlas and initial instance-buffer sizes no longer compile. Use `GpuResourceBudget::new(atlas, initial)` or add `..GpuResourceBudget::default()`.
- GPUI 0.9 dropped its pass-through dependency-name Cargo features. This crate already forwards only semantic GPUI features (`accessibility`, `font-kit`, `wayland`, `x11`, `windows-manifest`, `image-format-*`, `image-rayon`), so no feature flags change here.
- The crate-local `Spring` type in `adabraka_ui::spring` is unchanged and is still what the prelude's `Spring` refers to. GPUI's `SpringConfig` / `SpringAnimation` are a separate analytic API re-exported alongside it; the built-in components continue to animate with the crate's own springs.
- `Responsive` and the `responsive_*` helpers are unchanged. `container_query` is offered as an additional element-level option and does not replace breakpoint-based responsiveness.

## [0.8.0] - 2026-07-28

### Added
- Added `HttpSetup` and `try_init_with` so applications can explicitly install the built-in HTTP client, provide a user agent, or preserve GPUI's current client.
- Added crate-owned `InitError` and `HttpInitError` types for diagnosable initialization failures without exposing the HTTP backend in public signatures.
- Added the default `accessibility` Cargo feature and integration-tested AccessKit semantics for Button, Checkbox, Select, Input, and Dialog.
- Added explicit Select expand/collapse/select accessibility actions, list/option semantics, and readable labels for icon-only dialog controls.
- Added macOS and Windows real-renderer smoke coverage for a core form and a Popover containing Select.

### Changed
- `adabraka_ui::init(cx)` now leaves GPUI's application-wide HTTP client unchanged instead of replacing it.
- The built-in client's default user agent now uses the current crate version.
- Updated the dependency baseline to the official `adabraka-gpui` v0.8.1 release.
- The default feature set now includes `accessibility`; use `default-features = false` to opt out of native accessibility adapters.
- `Dialog::header`, `child`, `children`, and `footer` now take render builders so their elements are recreated on every redraw.

### Fixed
- Password inputs now enable masking when they first transition to the password type, while preserving later user-driven visibility toggles.
- Loading buttons are now reported as unavailable to assistive technology.
- Unselected Select controls now expose placeholder text without reporting it as a selected value.
- Dedicated visual-smoke CI jobs now fail instead of silently skipping when a real renderer is unavailable.
- Dialog close handlers now run exactly once after dismissal, even across later redraws.
- Dialog header, body, and footer content now remain rendered and accessible after focus or animation redraws.

### Migration Notes
- Applications that relied on `adabraka_ui::init(cx)` to install the built-in client must use `try_init_with(cx, HttpSetup::Default)` or call `init_http(cx)` explicitly.
- Use `try_init_with(cx, HttpSetup::UserAgent(...))` for a custom user agent. A second explicit root initialization now returns `InitError::AlreadyInitialized` instead of silently ignoring the requested policy.
- Wrap Dialog slot content in a render builder, for example `.child(|_, _| div().child("Content"))` and `.footer(|_, _| Button::new("save", "Save"))`.

## 0.7.0 - 2026-07-22

### Changed
- `use_theme` now requires an `&App`/GPUI context and stores themes per application instead of in process-global state.
- The crate version is now `0.7.0` to reflect this public API change and the shared rich-text callback representation.
- Variable virtual-list extents are snapshotted once at construction; non-finite and negative extents are treated as zero.

### Fixed
- Replaced render-time detached input subscriptions with one state-owned subscription for Input, Textarea, MentionInput, and OTPInput.
- Prevented stale Editor parse results from landing after content, language, or file changes.
- Made OTP digit-count configuration preserve focus-handle invariants, including repeated shrinking and growing within the construction capacity.
- Made `InputState::set_value` replace existing content instead of inserting at the current selection.
- Routed Markdown and HTML links through custom handlers when provided.
- Made keyframe and chart range APIs robust to non-finite or degenerate numeric inputs, ignoring invalid chart points and defaulting an entirely invalid data set.
- Made root and component initialization complete, per-App, and idempotent.
- Centralized shared UTF-8/UTF-16 offset conversion logic used by text editing components.

### Migration Notes
- Replace `use_theme()` with `use_theme(cx)` at render sites.
- `LinkClickHandler` is shared with `Rc`; construct custom handlers through the Markdown/HTML builder APIs where possible.

## 0.6.0 - 2026-05-10

This release focuses on **shrinking the default binary footprint** and **modernizing
the dependency stack**, while keeping the out-of-the-box runtime behavior unchanged.

### Added
- **Font feature flags** ([`bundled-fonts`], [`bundled-fonts-inter`],
  [`bundled-fonts-inter-minimal`], [`bundled-fonts-mono`], [`bundled-fonts-mono-full`],
  plus per-weight flags `font-inter-regular` / `font-inter-medium` /
  `font-inter-semibold` / `font-inter-bold` / `font-mono-regular` / `font-mono-bold`).
  Downstream crates can now embed only the font weights they actually use, or fully
  disable bundled fonts (~2.1 MB) and register their own.
- **`editor` and `qrcode` Cargo features** that gate the editor stack
  (`ropey`, `tree-sitter*`) and QR rendering (`qrcode`) as opt-in dependencies.
  `editor-languages` now depends on `editor`.
- **Pass-through image-decoder features** (`image-format-*`,
  `image-default-formats`, `image-rayon`) so downstream apps can pick only the
  image decoders they need.

### Changed
- **Default features**: `default = ["http", "bundled-fonts"]` (was `["http"]`).
  Apps using the default feature set get the same fonts as before, but apps that
  build with `default-features = false` will no longer embed fonts unless they opt
  in via a `bundled-fonts*` flag.
- **HTTP backend** switched from `isahc` to `zed-reqwest`. `SimpleHttpClient` is
  now built on `reqwest::blocking` driven through `smol::unblock`.
- **GPUI default features disabled**: `adabraka-ui` no longer pulls in
  `adabraka-gpui`'s default image formats. Only platform-required features stay on.
- Replaced direct `once_cell` usage with `std::sync::{OnceLock, LazyLock}`.
- Bumped `adabraka-gpui` and refreshed `Cargo.lock` accordingly.

### Fixed
- Silenced spurious `feature` and platform `cfg` warnings.
- Updated docs to align with `adabraka-gpui 0.6` version references.

### Migration Notes
- If you build with `default-features = false` **and** rely on bundled Inter /
  JetBrains Mono fonts, add a feature flag explicitly, e.g.
  `features = ["http", "bundled-fonts"]` (or a smaller subset such as
  `bundled-fonts-inter-minimal`). Otherwise either register your own fonts via
  `cx.text_system().add_fonts(...)` and override the theme's `font_family` /
  `font_mono` tokens, or accept system-font fallback.
- If you used the editor or QR components, enable the new `editor` /
  `editor-languages` / `qrcode` features in your `Cargo.toml`.
- If you decoded specific image formats through this crate, enable the matching
  `image-format-*` (or `image-default-formats`) feature.

## 0.5.0 - 2026-04-20

### Changed
- Updated the repository package version to `0.5.0`
- Aligned the current branch with `adabraka-gpui v0.6.0`
- Kept the documented installation flow on `git` dependencies so it matches the repository state before the next crates.io release
- Refreshed README and docs site version references to remove stale `0.3.x` instructions

## [0.3.4] - 2026-02-18

### Fixed
- docs.rs build failure: gated `Arc` import behind `#[cfg(feature = "audio")]` in `audio_player.rs`
- Added `[package.metadata.docs.rs]` with `all-features = true` for reliable documentation builds

## [0.3.3] - 2026-02-18

### Fixed
- Editor: use char indices for rope insert/remove operations, fixing cursor drift and incorrect text placement in files with multi-byte UTF-8 characters

## [0.3.2] - 2026-02-17

### Fixed
- Suppress all compiler warnings across charts and components

## [0.3.1] - 2026-02-16

### Fixed
- Editor cursor positioning with horizontal scroll offset
- Editor UTF-8 backspace/delete handling

## [0.3.0] - 2026-02-06

### Added - Major Release: crates.io Publishing & GPUI Fork Enhancements

#### Published to crates.io
- All 12 adabraka-gpui ecosystem crates published under `adabraka_*` namespace
- adabraka-ui v0.3.0 published - `cargo add adabraka-ui` now works with no git deps
- Proper Zed attribution in all crate descriptions

#### GPUI Fork Enhancements (adabraka-gpui v0.3.0)
- **Inset shadows**: `BoxShadow.inset` field supported across Metal, WGSL, HLSL shader backends
- **Letter spacing**: `TextStyle.letter_spacing` with `tracking_tight()`, `tracking_wide()` convenience methods
- **Animation cancellation**: `AnimationHandle` and `with_cancellable_animation()` for interruptible animations
- **Squircle corners**: `continuous_corners` on `Styled` for iOS-style superellipse corners
- **Text shadow**: `TextShadow` struct with `text_shadow_sm/md/lg()` presets

#### New Components
- **Form** (`form.rs`) - Declarative form builder with validation, field groups, submit
- **InfiniteScroll** (`infinite_scroll.rs`) - Auto-loading paginated data on scroll
- **SortableList** (`sortable_list.rs`) - Drag-to-reorder list with smooth animations
- **DataGrid** (`data_grid.rs`) - Spreadsheet-style grid with inline editing, sorting, column resize
- **Animation Builder** (`animate.rs`) - Preset animations, keyframes, stagger, transitions

#### Animation & Polish System (v2)
- Exit animations for all overlay components (dismissing state + timer pattern)
- Input shake on validation failure (`trigger_shake()` on InputState)
- Ripple effect on Button and IconButton (`.ripple(true)`)
- 30+ easing functions: cubic-bezier, steps, elastic, back, circ, expo, quint
- Spring physics engine (`spring.rs`)
- Gesture recognizers (`gestures.rs`) - swipe, pinch, long-press
- Content transitions (`content_transition.rs`) - animated content swapping
- Responsive utilities (`responsive.rs`) - breakpoint-aware layouts
- Animation coordinator (`animation_coordinator.rs`) - multi-element orchestration
- ScrollPhysics integration with momentum scrolling in layout.rs + virtual_list.rs
- Smooth scroll-to: `scroll_to_y_animated`, `scroll_to_x_animated`
- Elevation shadows, inset shadows, layered gradients in theme tokens

#### Developer Experience
- GPUI re-exports via `gpui_ext.rs`
- `StyledExt` trait: `.center()`, `.stack()`, `.row()`, `.glass()`, `.elevated()`, `.ring()`
- Spacing, duration, z-index tokens on all 18 themes
- Expanded prelude with 15+ additional exports
- Init fixes for Sheet and AlertDialog components

### Changed
- **BREAKING**: GPUI dependency changed from `gpui` to `adabraka-gpui` (fork with enhancements)
- Version bumped from 0.2.4 to 0.3.0 to reflect ecosystem publishing
- Component count increased from 80+ to 85+

## [0.2.4] - 2026-01-15

### Added
- AudioPlayer component with real audio playback via rodio
- VideoPlayer component with flexible video backend integration
- Rating component with half-star support
- Sparkline component (Line, Bar, Area variants)
- MentionInput component with @mention dropdown
- MasonryGrid layout component
- Countdown timer component
- Optional `audio` feature flag for real audio playback

## [0.2.3] - 2025-11-13

### Added
- Vertical slider orientation support
  - New `SliderAxis` enum with `Horizontal` and `Vertical` variants
  - `.vertical()` and `.horizontal()` builder methods
  - Separate rendering logic for horizontal and vertical orientations
  - Vertical sliders grow from bottom to top (0% at bottom, 100% at top)
  - Adaptive thumb shape: horizontal oval for horizontal sliders, vertical oval for vertical sliders
  - `update_from_position_vertical()` method for proper vertical drag handling

### Fixed
- Slider thumb vertical centering
  - Container height now matches thumb height for proper alignment
  - Thumb positioned at `top: 0` instead of calculated offset
  - Track perfectly centered within container using flex layout
  - Thumb now sits centered on the track line instead of above it

### Improved
- Slider component architecture with separate `render_horizontal()` and `render_vertical()` methods
- Cleaner positioning logic using container dimensions matching thumb dimensions
- Better visual consistency across all slider sizes (Sm, Md, Lg)

### Examples
- Updated `slider_styled_demo.rs` with 10 comprehensive examples
  - 7 horizontal slider variations demonstrating sizes, styling, and features
  - 3 vertical slider examples showcasing the new orientation support
  - All examples fully interactive with drag support and onChange handlers

## [0.2.2] - 2025-10-28

### Added
- Tab/Shift-Tab keyboard navigation between form inputs
  - Implemented `tab()` and `shift_tab()` handlers in InputState
  - Proper FocusHandle configuration with `.tab_index(0).tab_stop(true)`
  - Window-level focus navigation using `window.focus_next()` and `window.focus_prev()`
- Comprehensive ROADMAP.md with 90+ component inventory
  - Complete component categorization and status tracking
  - Phase-based development plan with desktop integration features
  - Prioritized quick wins and improvements

### Fixed
- Password input eye icon toggle functionality
  - Icon now properly toggles between "eye" and "eye-off"
  - Password masking correctly switches between bullets (••••) and actual text
  - Immediate UI updates with `window.refresh()` after state changes
  - Fixed state reading to use dynamic `input_state.masked` value

### Improved
- Code quality improvements with removal of 13 unnecessary inline comments across 6 files
  - Removed comments from: color_picker.rs, input_state.rs, input.rs, text.rs, lib.rs, transitions.rs
  - Cleaner, more production-ready codebase

### Examples
- Added `password_test.rs` - Demonstrates password toggle functionality with clear instructions

## [0.2.1] - 2025-10-23

### Added - 🎉 Three New Production-Ready Components!

#### ColorPicker Component 🎨
- Full-featured color picker with HSL, RGB, and HEX mode switching
- Recent colors history (stores last 10 colors automatically)
- Custom color swatches support
- Optional alpha/opacity slider
- Copy to clipboard functionality (HEX format)
- Popover-based clean UI integration
- Immediate UI updates with `cx.notify()`

#### DatePicker Component 📅
- Single date and date range selection modes
- Visual range highlighting with colored backgrounds
  - Range endpoints: bold primary color
  - Range middle dates: light background (15% opacity)
- Disabled dates with greyed-out visual styling
- Weekend disabling helper method (`disable_weekends()`)
- Auto-close popover after selection
- Multiple date formats (ISO, US, EU, custom)
- Locale support for internationalization
- Month navigation with year selection
- Today button for quick selection
- Immediate UI updates without mouse movement

#### Combobox Component 🔍
- Single and multi-select modes
- Real-time search/filter with immediate updates
- Full keyboard navigation (arrow keys, Enter, Escape)
- Custom display and search functions
- Clear selection button
- Badge display for multi-select items
- Popover-based dropdown UI
- Empty state handling
- Disabled state support

#### Calendar Component Enhancements
- Added `DateRange` support with visual styling
- Disabled dates checker function (`is_date_disabled`)
- Range endpoints with bold styling
- Range middle dates with light background
- Improved date selection feedback

### Changed
- Updated component count from 70+ to 73+
- Updated examples count from 50+ to 53+
- Enhanced Calendar component with range selection capabilities

### Fixed
- Fixed all compiler warnings (unused fields, variables, methods)
- Removed unused `is_open` field from ColorPickerState
- Fixed GPUI state lifecycle issues using proper `Entity<T>` pattern
- Proper `DismissEvent` emission for popover closing
- Added `cx.notify()` calls throughout for immediate UI updates

### Improved
- Zero compiler warnings - completely clean build
- Immediate UI updates across all new components (no mouse movement required)
- Comprehensive documentation with code examples for all new components
- Updated README with detailed component documentation
- Updated GitHub Pages with new component listings
- Professional visual styling with theme integration
- Full keyboard navigation support for all new components

### Examples
- Added `color_picker_demo.rs` - Demonstrates all ColorPicker features
- Added `date_picker_demo.rs` - Shows single date and range selection
- Added `combobox_demo.rs` - Illustrates search and multi-select

## [0.2.0] - 2025-10-23

### Added - 🎉 MAJOR RELEASE: 100% Styled Trait Coverage!

#### Icon System Enhancements
- **Icon Phase 1**: Consolidated IconSource module across all components
- **Icon Phase 2**: Added IconSize enum with named sizes (XSmall, Small, Medium, Large, Custom)
- Added rotation support for Icon component using Transformation API
- Improved icon path detection with separator-first logic
- Added comprehensive unit tests for IconSource

#### Component Enhancements
- **Text Component**: Fixed italic and strikethrough rendering using HighlightStyle API
- **Button Component**: Improved API with better ID parameter handling
- **Checkbox Component**: Replaced emoji icons with customizable Icon components
- **Calendar Component**: Added full internationalization (i18n) support with CalendarLocale
  - Built-in locales: English, French, Spanish, German, Portuguese, Italian
  - Support for custom locales

#### Styled Trait Implementation - **ALL 54 COMPONENTS!**
- **Components (14)**: Button, Input, Checkbox, IconButton, Label, Radio, Toggle, Textarea, Avatar, Progress, Slider, Separator, SearchInput, Select
- **Display (6)**: Card, Badge, Accordion, Table, DataTable, Collapsible
- **Navigation (9)**: Menu, Tabs, Toolbar, Sidebar, Breadcrumbs, NavigationMenu, StatusBar, Tree, AppMenu
- **Overlays (11)**: Dialog, Sheet, AlertDialog, Toast, BottomSheet, CommandPalette, ContextMenu, HoverCard, Popover, PopoverMenu, Tooltip
- **Advanced (9)**: TextField, Pagination, ToggleGroup, KeyboardShortcuts, Calendar, Resizable, Editor, Draggable, DropZone

#### 54 New Styled Demos
Created comprehensive styled demonstration examples for every component showing full customization capabilities

### Changed
- **BREAKING**: Icon component now returns AnyElement instead of Div for non-clickable icons (performance improvement)
- All components now support full GPUI styling methods via Styled trait
- User styles now properly override component defaults using `.refine()` pattern
- Removed 3,274 inline comments for cleaner, production-ready code

### Fixed
- Fixed Editor component `.when()` pattern to `.map()` for style application
- Fixed DropZone naming conflict by renaming internal `style` field to `drop_style`
- Fixed Button API usage across 21 example files
- Fixed various component compilation errors and import issues
- Fixed sidebar_demo and menu_demo import paths

### Improved
- **shadcn Philosophy Alignment**: All components now follow "good defaults with complete user control"
- Every component supports customization: `.bg()`, `.border_2()`, `.rounded_lg()`, `.p_4()`, `.shadow_lg()`, and hundreds more
- Added SHADCN_DESIGN_PHILOSOPHY.md documenting design principles
- Better developer experience with consistent Styled trait API across all components
- Production-ready code quality with clean, documented implementations

## [0.1.1] - 2025-10-22

### Changed
- **BREAKING**: Icons are no longer bundled with the library (reduces package size by 95%)
- Added configurable icon path system with `set_icon_base_path()` function
- Users must now provide their own icon assets (see README for setup instructions)

### Fixed
- Fixed 20+ examples with incorrect API usage
- Fixed `scroll` module imports (changed to `scrollable`)
- Fixed VStack compatibility with scrollable_vertical
- Fixed Menu and MenuItem API usage
- Fixed toolbar click handlers to use `on_mouse_down`
- Removed 3 broken test examples

### Improved
- Removed unnecessary inline comments for cleaner, production-ready code
- Added comprehensive icon setup documentation in README
- All 53 working examples now compile successfully
- Updated examples with proper AssetSource configuration

## [0.1.0] - 2025-10-21

### Added
- Initial release of adabraka-ui
- 70+ UI components organized into categories:
  - Core components (Button, Input, Checkbox, Toggle, Select, Slider, etc.)
  - Display components (Card, Badge, Table, DataTable, Accordion)
  - Navigation components (Tabs, Breadcrumbs, Tree, Sidebar, Menu, Toolbar, StatusBar)
  - Overlay components (Dialog, Popover, Toast, CommandPalette, Sheet, etc.)
  - Advanced components (Editor, Scrollable, Resizable, DragDrop, Progress)
- Complete theme system with light and dark modes
- Semantic color tokens inspired by shadcn/ui
- Professional animation system with cubic-bezier easing and spring physics
- Typography system with semantic text variants
- Code editor with syntax highlighting support
- Virtual scrolling for large datasets
- Full keyboard navigation and accessibility support
- Comprehensive documentation and examples

### Features
- Builder pattern API for ergonomic component construction
- Entity-based state management for complex components
- Type-safe APIs with compile-time guarantees
- Performance-optimized for GPUI's retained-mode rendering
- Consistent styling across all components
- Platform-aware UI elements
- Responsive layout utilities (VStack, HStack, Grid)

[Unreleased]: https://github.com/freefcw/fc-ui/compare/v0.8.1...HEAD
[0.8.1]: https://github.com/freefcw/fc-ui/compare/v0.8.0...v0.8.1
[0.8.0]: https://github.com/Augani/adabraka-ui/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/Augani/adabraka-ui/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Augani/adabraka-ui/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Augani/adabraka-ui/compare/v0.3.4...v0.5.0
[0.3.4]: https://github.com/Augani/adabraka-ui/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/Augani/adabraka-ui/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/Augani/adabraka-ui/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/Augani/adabraka-ui/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Augani/adabraka-ui/releases/tag/v0.3.0
[0.2.4]: https://github.com/Augani/adabraka-ui/releases/tag/v0.2.4
[0.2.3]: https://github.com/Augani/adabraka-ui/releases/tag/v0.2.3
[0.2.2]: https://github.com/Augani/adabraka-ui/releases/tag/v0.2.2
[0.2.1]: https://github.com/Augani/adabraka-ui/releases/tag/v0.2.1
[0.2.0]: https://github.com/Augani/adabraka-ui/releases/tag/v0.2.0
[0.1.1]: https://github.com/Augani/adabraka-ui/releases/tag/v0.1.1
[0.1.0]: https://github.com/Augani/adabraka-ui/releases/tag/v0.1.0
