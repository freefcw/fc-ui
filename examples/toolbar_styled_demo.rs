use fc_ui::{components::scrollable::scrollable_vertical, navigation::toolbar::*, prelude::*};
use gpui::*;
use std::path::PathBuf;

struct Assets {
    base: PathBuf,
}

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        std::fs::read(self.base.join(path))
            .map(|data| Some(std::borrow::Cow::Owned(data)))
            .map_err(|err| err.into())
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        std::fs::read_dir(self.base.join(path))
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        entry
                            .ok()
                            .and_then(|entry| entry.file_name().into_string().ok())
                            .map(SharedString::from)
                    })
                    .collect()
            })
            .map_err(|err| err.into())
    }
}

fn main() {
    Application::new()
        .with_assets(Assets {
            base: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        })
        .run(|cx| {
            fc_ui::init(cx);
            fc_ui::set_icon_base_path("assets/icons");
            install_theme(cx, Theme::dark());

            cx.open_window(
                WindowOptions {
                    titlebar: Some(TitlebarOptions {
                        title: Some("Toolbar Styled Trait Demo".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(1000.0), px(900.0)),
                    })),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| ToolbarStyledDemo::new()),
            )
            .unwrap();
        });
}

struct ToolbarStyledDemo;

impl ToolbarStyledDemo {
    fn new() -> Self {
        Self
    }
}

impl Render for ToolbarStyledDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .size_full()
            .bg(theme.tokens.background)
            .overflow_hidden()
            .child(
                scrollable_vertical(
                    div()
                        .flex()
                        .flex_col()
                        .text_color(theme.tokens.foreground)
                        .gap(px(24.0))
            .child(
                VStack::new()
                    .gap(px(8.0))
                    .p(px(32.0))
                    .child(
                        div()
                            .text_size(px(24.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Toolbar Styled Trait Customization Demo")
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Demonstrating full GPUI styling control via Styled trait")
                    )
            )
            // 1. Default Toolbar
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("1. Default Toolbar (No Custom Styling)")
                    )
                    .child(cx.new(|_| {
                        Toolbar::new()
                            .size(ToolbarSize::Md)
                            .group(
                                ToolbarGroup::new()
                                    .button(
                                        ToolbarButton::new("save", "save")
                                    )
                                    .button(
                                        ToolbarButton::new("copy", "copy")
                                    )
                                    .button(
                                        ToolbarButton::new("paste", "clipboard")
                                    )
                            )
                            .group(
                                ToolbarGroup::new()
                                    .button(
                                        ToolbarButton::new("undo", "undo")
                                    )
                                    .button(
                                        ToolbarButton::new("redo", "redo")
                                    )
                            )
                    }))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Standard toolbar with default background and border")
                    )
            )
            // 2. Custom Background Color
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("2. Custom Background Color")
                    )
                    .child(cx.new(|_| {
                        Toolbar::new()
                            .size(ToolbarSize::Md)
                            .group(
                                ToolbarGroup::new()
                                    .button(ToolbarButton::new("file", "file"))
                                    .button(ToolbarButton::new("folder", "folder"))
                                    .button(ToolbarButton::new("search", "search"))
                            )
                            .group(
                                ToolbarGroup::new()
                                    .button(ToolbarButton::new("settings", "settings"))
                                    .button(ToolbarButton::new("user", "user"))
                            )
                            .bg(rgb(0x1e3a8a))  // Deep blue background
                    }))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Toolbar with .bg(rgb(0x1e3a8a)) - deep blue background")
                    )
            )
            // 3. Custom Padding and Border Radius
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("3. Custom Padding and Border Radius")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Lg)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("bold", "bold"))
                                            .button(ToolbarButton::new("italic", "italic"))
                                            .button(ToolbarButton::new("underline", "underline"))
                                    )
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("align-left", "align-left"))
                                            .button(ToolbarButton::new("align-center", "align-center"))
                                            .button(ToolbarButton::new("align-right", "align-right"))
                                    )
                                    .px(px(24.0))  // Extra horizontal padding
                                    .py(px(12.0))  // Extra vertical padding
                                    .rounded(px(12.0))  // Rounded corners
                                    .bg(theme.tokens.muted)
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Applied: .px(px(24.0)), .py(px(12.0)), .rounded(px(12.0))")
                    )
            )
            // 4. Custom Border Styling
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("4. Custom Border Styling")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Md)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("image", "image"))
                                            .button(ToolbarButton::new("video", "video"))
                                            .button(ToolbarButton::new("music", "music"))
                                    )
                                    .border_2()  // 2px border
                                    .border_color(rgb(0x8b5cf6))  // Purple border
                                    .rounded(px(8.0))
                                    .bg(gpui::transparent_black())
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Applied: .border_2(), .border_color(rgb(0x8b5cf6)), .rounded(px(8.0))")
                    )
            )
            // 5. Shadow Effects
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("5. Shadow Effects")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Md)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("download", "download"))
                                            .button(ToolbarButton::new("upload", "upload"))
                                            .button(ToolbarButton::new("share", "share-2"))
                                    )
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("heart", "heart"))
                                            .button(ToolbarButton::new("star", "star"))
                                    )
                                    .shadow_lg()  // Large shadow
                                    .rounded(px(8.0))
                                    .bg(theme.tokens.background)
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Applied: .shadow_lg(), .rounded(px(8.0))")
                    )
            )
            // 6. Width Control
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("6. Width Control")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Sm)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("zoom-in", "zoom-in"))
                                            .button(ToolbarButton::new("zoom-out", "zoom-out"))
                                    )
                                    .w(px(300.0))  // Fixed width
                                    .bg(theme.tokens.accent)
                                    .rounded(px(8.0))
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Applied: .w(px(300.0)) - fixed width toolbar")
                    )
            )
            // 7. Combined Advanced Styling
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("7. Combined Advanced Styling")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Lg)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("play", "play"))
                                            .button(ToolbarButton::new("pause", "pause"))
                                            .button(ToolbarButton::new("stop", "square"))
                                    )
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("skip-back", "skip-back"))
                                            .button(ToolbarButton::new("skip-forward", "skip-forward"))
                                    )
                                    .px(px(32.0))  // Extra padding
                                    .py(px(16.0))
                                    .rounded(px(16.0))  // Large radius
                                    .bg(rgb(0x059669))  // Green background
                                    .shadow_md()  // Medium shadow
                                    .border_1()  // Subtle border
                                    .border_color(rgb(0x10b981))
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Combined: .px(32), .py(16), .rounded(16), .bg(), .shadow_md(), .border_1()")
                    )
            )
            // 8. Compact Toolbar
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("8. Compact Toolbar with Custom Styling")
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .flex()
                            .justify_center()
                            .child(cx.new(|_| {
                                Toolbar::new()
                                    .size(ToolbarSize::Sm)
                                    .group(
                                        ToolbarGroup::new()
                                            .button(ToolbarButton::new("menu", "menu"))
                                            .button(ToolbarButton::new("home", "home"))
                                            .button(ToolbarButton::new("bell", "bell"))
                                    )
                                    .p_2()  // Minimal padding
                                    .rounded(px(999.0))  // Pill shape
                                    .bg(rgb(0xef4444))  // Red background
                                    .shadow_sm()
                                    .w(px(200.0))
                            }))
                    )
                    .child(
                        div()
                            .px(px(32.0))
                            .text_size(px(12.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Applied: .p_2(), .rounded(px(999.0)) pill shape, .w(px(200.0))")
                    )
            )
            // Info Box
            .child(
                div()
                    .px(px(32.0))
                    .pb(px(32.0))
                    .child(
                        div()
                            .mt(px(16.0))
                            .p(px(16.0))
                            .bg(theme.tokens.accent)
                            .rounded(px(8.0))
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(theme.tokens.accent_foreground)
                                    .child("All customizations above use the Styled trait for full GPUI styling control!")
                            )
                            .child(
                                div()
                                    .mt(px(8.0))
                                    .text_size(px(12.0))
                                    .text_color(theme.tokens.accent_foreground)
                                    .child("Methods used: .bg(), .px(), .py(), .p_2(), .border_1/2(), .border_color(), .rounded(), .w(), .shadow_sm/md/lg()")
                            )
                    )
            )
                )
            )
    }
}
