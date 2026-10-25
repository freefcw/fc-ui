//! Canonical private implementation ownership for the component library.
//!
//! Put new code in the capability that owns its primary user-facing behavior, not in the
//! legacy public facade that happens to expose it. The capability boundaries are:
//!
//! | Capability | Owns | Does not own |
//! | --- | --- | --- |
//! | `foundation` | Theme, fonts, GPUI extensions, HTTP, initialization guard, utilities | Component workflows and feature keybindings |
//! | `layout` | Reusable stacks, grids, panels, containers, and masonry | Scrolling, virtualization, or component-specific interactions |
//! | `motion` | Animation and physics mechanics | Component presentation or interaction policies |
//! | `primitives` | Reusable leaf UI building blocks | Forms, navigation flows, or overlays |
//! | `scroll` | Scroll containers, scrollbars, resizing, split panes, and virtual lists | Generic stacks, grids, panels, or containers |
//! | `overlays` | Transient contextual surfaces such as dialogs, popovers, sheets, and toasts | App navigation and persistent content |
//! | `controls` | User value entry, selection, and form controls | Rope-backed document editing or media playback |
//! | `navigation` | App navigation, menus, trees, routes, and command-driven navigation | Transient dialog surfaces or generic layout |
//! | `data` | Structured-data presentation such as charts, tables, and timelines | Data fetching, persistence, or generic layout |
//! | `content` | Read-only or rich content rendering | Interactive value entry or document editing state |
//! | `editor` | Rope-backed document editing and language-aware editing | String-based input controls |
//! | `media` | Audio, images, video, canvas, and user-supplied media assets | Generic file-system navigation |
//! | `effects` | Decorative and transition effects | Core component interaction state |
//!
//! Canonical modules may import only the capability dependencies allowed by
//! `scripts/check_boundaries.py`; they must not import through legacy public
//! facades. Production capability code must use `crate::capabilities::...`
//! paths, never `super::...`, so dependency direction has one clear, checked
//! spelling.

// GPUI component APIs intentionally expose callback and rendering-context
// signatures that Clippy cannot express compactly without adding aliases at
// every component boundary. The callback shapes are part of the component
// contracts, and the render helpers naturally need several context values.
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::wrong_self_convention)]

pub mod content;
pub mod controls;
pub mod data;
pub mod editor;
pub mod effects;
pub mod foundation;
pub mod layout;
pub mod media;
pub mod motion;
pub mod navigation;
pub mod overlays;
pub mod primitives;
pub mod scroll;
