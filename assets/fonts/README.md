# Fonts Directory

This directory contains the font files used by the fc-ui component library.

## Bundled Fonts

| File | Size | Weight | Usage |
|------|------|--------|-------|
| Inter-Regular.ttf | 402K | Regular (400) | Body text, labels |
| Inter-Medium.ttf | 408K | Medium (500) | Buttons, badges, avatars |
| Inter-SemiBold.ttf | 410K | SemiBold (600) | Headings, dialog titles, selected state |
| Inter-Bold.ttf | 411K | Bold (700) | H1, editor line numbers, emphasis |
| JetBrainsMono-Regular.ttf | 267K | Regular | Code editor, OTP, kbd, code blocks |
| JetBrainsMono-Bold.ttf | 271K | Bold | Editor current line number highlight |

**Total: ~2.1 MB**

## Feature Flags

Font embedding is controlled via Cargo feature flags, allowing downstream crates to include only the fonts they need.

### Convenience Features

| Feature | Includes | Size |
|---------|----------|------|
| `bundled-fonts` (default) | All 6 fonts | ~2.1 MB |
| `bundled-fonts-inter` | Inter Regular + Medium + SemiBold + Bold | ~1.6 MB |
| `bundled-fonts-inter-minimal` | Inter Regular + SemiBold | ~812 KB |
| `bundled-fonts-mono` | JetBrains Mono Regular | ~267 KB |
| `bundled-fonts-mono-full` | JetBrains Mono Regular + Bold | ~538 KB |

### Individual Font Features

- `font-inter-regular`
- `font-inter-medium`
- `font-inter-semibold`
- `font-inter-bold`
- `font-mono-regular`
- `font-mono-bold`

### Usage Examples

```toml
# Full fonts (default, ~2.1 MB embedded)
[dependencies]
fc-ui = "0.9"

# Lightweight: only Regular + SemiBold + Mono Regular (~1.07 MB, saves ~1 MB)
[dependencies]
fc-ui = { version = "0.9", default-features = false, features = ["http", "bundled-fonts-inter-minimal", "bundled-fonts-mono"] }

# No bundled fonts (downstream registers its own fonts)
[dependencies]
fc-ui = { version = "0.9", default-features = false, features = ["http"] }
```

### Registering Custom Fonts (No Bundled Fonts)

When bundled fonts are disabled, register fonts manually before rendering UI. The theme
must also point at the registered font family names. If you keep the default theme tokens,
your custom fonts need internal family names of `Inter` and `JetBrains Mono`.

```rust
use fc_ui::theme::{install_theme, Theme};
use gpui::*;

Application::new().run(|cx| {
    // Register your own fonts
    cx.text_system()
        .add_fonts(vec![
            include_bytes!("path/to/YourFont-Regular.ttf").as_slice().into(),
        ])
        .expect("Failed to load fonts");

    let mut theme = Theme::dark();
    theme.tokens.font_family = "Your Font".into();
    theme.tokens.font_mono = "Your Mono Font".into();
    install_theme(cx, theme);

    // Then initialize UI
    fc_ui::init(cx);
});
```

## Font Sources

**Inter** (UI Font):
- Download from: https://rsms.me/inter/
- License: SIL Open Font License 1.1

**JetBrains Mono** (Monospace Font):
- Download from: https://www.jetbrains.com/lp/mono/
- License: SIL Open Font License 1.1

## Applying Fonts to Components

Once fonts are registered, you can use them with GPUI's font APIs:

```rust
use gpui::*;

// Use the UI font (via theme - recommended)
let theme = fc_ui::theme::use_theme(cx);
div().font_family(theme.tokens.font_family.clone())

// Use the mono font
div().font_family(theme.tokens.font_mono.clone())

// Direct access to font family names
fc_ui::fonts::ui_font_family()   // -> "Inter"
fc_ui::fonts::mono_font_family() // -> "JetBrains Mono"
```

The theme system will automatically use these fonts throughout the component library once registered.
