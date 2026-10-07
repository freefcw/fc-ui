#![allow(missing_docs)]

//! # fc-ui: Professional UI Component Library for GPUI
//!
//! A comprehensive, themeable component library inspired by shadcn/ui, designed specifically
//! for building polished desktop applications using GPUI. Provides a complete set of
//! reusable components with consistent styling, smooth animations, and progressively tested
//! accessibility support.
//! ## Architecture Overview
//!
//! The library is organized into several key modules:
//! - `theme`: Design tokens and theming system with light/dark variants
//! - `components`: Core interactive elements (buttons, inputs, selects, etc.)
//! - `display`: Presentation components (tables, cards, badges, etc.)
//! - `navigation`: Navigation components (sidebars, menus, tabs, etc.)
//! - `overlays`: Modal dialogs, popovers, tooltips, and command palettes
//! - `animations`: Professional animation presets and easing functions
//!
//! ## Key Features
//!
//! - **Theme System**: Design tokens across 18 preset themes with light and dark
//!   variants; the active theme is installed explicitly via `theme::install_theme`
//!   (the library does not track the system appearance automatically)
//! - **Accessibility**: Keyboard support across controls, with tested AccessKit semantics for
//!   Button, Checkbox, Select, Input, and Dialog
//! - **Performance**: Optimized rendering with virtual scrolling for large datasets
//! - **Animation**: Smooth, professional animations using spring physics and easing curves
//! - **Type Safety**: Strong typing throughout with compile-time guarantees
//!
//! ## Design Philosophy
//!
//! Components follow shadcn/ui principles with GPUI-specific optimizations:
//! - Composition over inheritance for flexible component APIs
//! - Builder pattern for ergonomic component construction
//! - Entity-based state management for complex interactive components
//! - Consistent naming and styling patterns across all components
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use fc_ui::{prelude::*, theme};
//!
//! // Initialize theme and components
//! fn init_app(cx: &mut gpui::App) {
//!     theme::install_theme(cx, theme::Theme::dark());
//!     fc_ui::init(cx);
//! }
//!
//! // Use components in your views
//! fn render(cx: &mut gpui::App) -> impl gpui::IntoElement {
//!     div()
//!         .child(Button::new("Click me").on_click(|_, _, _| println!("Clicked!")))
//!         .child(Input::new(&input_state).placeholder("Enter text..."))
//! }
//! ```
//!

extern crate gpui;

mod capabilities;

#[cfg(test)]
mod init_contract_tests;

pub mod animate;
pub mod animated_state;
pub mod animation_coordinator;
pub mod animations;
pub mod charts;
pub mod components;
pub mod content_transition;
pub mod display;
pub mod gestures;
pub mod gpui_ext;
pub mod layout;
pub mod navigation;
pub mod overlays;
pub mod prelude;
pub mod responsive;
pub mod scroll_physics;
pub mod spring;
pub mod styled_ext;
pub mod theme;
pub mod transitions;
pub mod virtual_list;

/// Extension traits for common types
pub mod util;

/// Font loading and registration
pub mod fonts;

/// Icon configuration for custom asset paths
pub mod icon_config;

/// HTTP client for remote image loading
pub mod http;

// Re-export commonly used icon configuration functions
pub use icon_config::set_icon_base_path;

// Re-export HTTP client functions
pub use http::{init_http, init_http_with_user_agent, DEFAULT_USER_AGENT};
#[cfg(feature = "http")]
pub use http::{try_init_http, try_init_http_with_user_agent, HttpInitError, HttpSetup};

/// Error returned by explicit root initialization.
#[cfg(feature = "http")]
#[derive(Debug)]
pub enum InitError {
    /// A root initializer has already completed for this application.
    AlreadyInitialized,
    /// The requested built-in HTTP client could not be constructed.
    Http(HttpInitError),
}

#[cfg(feature = "http")]
impl std::fmt::Display for InitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => formatter.write_str("fc-ui is already initialized"),
            Self::Http(error) => write!(formatter, "{error}"),
        }
    }
}

#[cfg(feature = "http")]
impl std::error::Error for InitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::AlreadyInitialized => None,
            Self::Http(error) => Some(error),
        }
    }
}

#[cfg(feature = "http")]
impl From<HttpInitError> for InitError {
    fn from(error: HttpInitError) -> Self {
        Self::Http(error)
    }
}

fn init_capabilities(cx: &mut gpui::App) {
    capabilities::foundation::init(cx);

    // GPUI resolves same-depth keybinding conflicts in reverse registration order.
    // Keep this historical cross-capability sequence stable for downstream applications.
    capabilities::controls::init_before_image_viewer(cx);
    capabilities::media::init_image(cx);
    capabilities::controls::init_after_image_viewer(cx);
    capabilities::media::init_video(cx);
    #[cfg(feature = "editor")]
    capabilities::editor::init(cx);
    capabilities::navigation::init(cx);
    capabilities::overlays::init(cx);
}

/// Initialize the UI library without changing GPUI's HTTP client.
///
/// This registers all necessary keybindings, fonts, and component systems. Use
/// [`try_init_with`] when the built-in HTTP client is required for remote images.
pub fn init(cx: &mut gpui::App) {
    if !capabilities::foundation::initialization::begin(cx, "fc-ui") {
        return;
    }

    init_capabilities(cx);
}

/// Initialize the UI library with an explicit HTTP-client policy.
///
/// Unlike [`init`], this reports an error when root initialization has already
/// completed, so a requested HTTP policy is never silently ignored.
#[cfg(feature = "http")]
pub fn try_init_with(cx: &mut gpui::App, http: HttpSetup) -> Result<(), InitError> {
    if capabilities::foundation::initialization::is_initialized(cx, "fc-ui") {
        return Err(InitError::AlreadyInitialized);
    }

    http::try_init_http_with_setup(cx, http)?;

    if !capabilities::foundation::initialization::begin(cx, "fc-ui") {
        return Err(InitError::AlreadyInitialized);
    }

    init_capabilities(cx);
    Ok(())
}
