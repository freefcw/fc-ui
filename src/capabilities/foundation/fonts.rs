//! # Font Loading and Registration
//!
//! Handles embedding and registering custom fonts with GPUI for consistent typography
//! across the component library. Fonts are embedded at compile time for reliable distribution.
//! ## Font Families
//!
//! - **Inter**: Primary UI font family (sans-serif) - clean, modern, highly legible
//! - **JetBrains Mono**: Monospace font for code, terminals, and technical content
//!
//! ## Font Weights
//!
//! - **Regular (400)**: Default weight for body text and labels
//! - **Medium (500)**: Slightly heavier for emphasis and buttons
//! - **SemiBold (600)**: For headings and important UI elements
//! - **Bold (700)**: For strong emphasis and primary actions
//!
//! ## Design Decisions
//!
//! - **Compile-time Embedding**: Fonts are included in the binary for consistent rendering
//! - **Limited Weights**: Only essential weights to minimize binary size
//! - **Cross-platform**: Fonts chosen for excellent rendering across all platforms
//! - **Performance**: Fonts loaded once at startup, cached by GPUI's text system
//! - **Fallback**: System fonts used if custom fonts fail to load (a warning
//!   is logged to stderr instead of panicking)
//!
//! ## Usage
//!
//! Fonts are automatically registered when calling `adabraka_ui::init(cx)`.
//! Access font families through the theme system or utility functions.
//!
//! ```rust,ignore
//! // Access via theme (recommended)
//! let theme = use_theme(cx);
//! div().font_family(theme.tokens.font_family.clone())
//!
//! // Direct access to font families
//! ui_font_family() // -> "Inter"
//! mono_font_family() // -> "JetBrains Mono"
//! ```
//!

use gpui::*;

/// Font family names used throughout the UI
pub const UI_FONT_FAMILY: &str = "Inter";
pub const UI_MONO_FONT_FAMILY: &str = "JetBrains Mono";

// Embed font files at compile time (gated behind feature flags)
// - Inter: https://rsms.me/inter/
// - JetBrains Mono: https://www.jetbrains.com/lp/mono/

// Inter weights
#[cfg(feature = "font-inter-regular")]
const INTER_REGULAR: &[u8] = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
#[cfg(feature = "font-inter-medium")]
const INTER_MEDIUM: &[u8] = include_bytes!("../../../assets/fonts/Inter-Medium.ttf");
#[cfg(feature = "font-inter-semibold")]
const INTER_SEMIBOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-SemiBold.ttf");
#[cfg(feature = "font-inter-bold")]
const INTER_BOLD: &[u8] = include_bytes!("../../../assets/fonts/Inter-Bold.ttf");

// Monospace
#[cfg(feature = "font-mono-regular")]
const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
#[cfg(feature = "font-mono-bold")]
const JETBRAINS_MONO_BOLD: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Bold.ttf");

/// Register embedded fonts with GPUI
///
/// Should be called during application initialization before any UI is rendered.
/// Registration failures only log a warning; text rendering then falls back
/// to system fonts.
///
/// The set of fonts actually registered is controlled by Cargo feature flags
/// (e.g. `bundled-fonts`, `bundled-fonts-inter-minimal`, `font-inter-regular`, ...).
/// When all font features are disabled this function is a no-op; in that case the
/// caller is responsible for registering fonts via `cx.text_system().add_fonts(...)`
/// and pointing the theme tokens (`font_family`, `font_mono`) at the registered
/// font families.
///
/// # Example
/// ```ignore
/// use adabraka_ui::fonts;
///
/// Application::new().run(|cx| {
///     fonts::register_fonts(cx);
///     // ... rest of initialization
/// });
/// ```
pub fn register_fonts(cx: &mut App) {
    // Register Inter family (UI font)
    #[allow(unused_mut)]
    let mut inter_fonts: Vec<std::borrow::Cow<'static, [u8]>> = Vec::new();
    #[cfg(feature = "font-inter-regular")]
    inter_fonts.push(INTER_REGULAR.into());
    #[cfg(feature = "font-inter-medium")]
    inter_fonts.push(INTER_MEDIUM.into());
    #[cfg(feature = "font-inter-semibold")]
    inter_fonts.push(INTER_SEMIBOLD.into());
    #[cfg(feature = "font-inter-bold")]
    inter_fonts.push(INTER_BOLD.into());

    if !inter_fonts.is_empty() {
        if let Err(error) = cx.text_system().add_fonts(inter_fonts) {
            eprintln!(
                "adabraka-ui: failed to load Inter fonts, falling back to system fonts: {error}"
            );
        }
    }

    // Register JetBrains Mono family (monospace font)
    #[allow(unused_mut)]
    let mut mono_fonts: Vec<std::borrow::Cow<'static, [u8]>> = Vec::new();
    #[cfg(feature = "font-mono-regular")]
    mono_fonts.push(JETBRAINS_MONO_REGULAR.into());
    #[cfg(feature = "font-mono-bold")]
    mono_fonts.push(JETBRAINS_MONO_BOLD.into());

    if !mono_fonts.is_empty() {
        if let Err(error) = cx.text_system().add_fonts(mono_fonts) {
            eprintln!("adabraka-ui: failed to load JetBrains Mono fonts, falling back to system fonts: {error}");
        }
    }
}

/// Get the default UI font family
pub fn ui_font_family() -> SharedString {
    UI_FONT_FAMILY.into()
}

/// Get the default monospace font family
pub fn mono_font_family() -> SharedString {
    UI_MONO_FONT_FAMILY.into()
}
