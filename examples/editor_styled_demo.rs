//! Demo showing the Editor component with the Styled trait applied.
//!
//! Demonstrates that `Editor` implements the `Styled` trait, so all GPUI
//! styling methods (`.bg()`, `.border_2()`, `.rounded_lg()`, `.shadow_lg()`,
//! width/padding helpers, etc.) can be applied directly to it.

use fc_ui::{
    components::editor::{Editor, EditorState},
    prelude::*,
};
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
                        title: Some("Editor Styled Trait Demo".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(900.0), px(700.0)),
                    })),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| EditorStyledDemo::new(cx)),
            )
            .unwrap();
        });
}

struct EditorStyledDemo {
    editor_state: Entity<EditorState>,
}

impl EditorStyledDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let editor_state = cx.new(|cx| {
            let mut state = EditorState::new(cx);
            state.set_content(
                "// Editor + Styled trait demo\n\
                // Edit me — this component is a real Editor instance.\n\
                //\n\
                fn main() {\n\
                    let theme = Theme::dark();\n\
                    println!(\"hello from the editor demo\");\n\
                }\n",
                cx,
            );
            state
        });

        Self { editor_state }
    }
}

impl Render for EditorStyledDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .size_full()
            .bg(theme.tokens.background)
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(24.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        div()
                            .text_size(px(22.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.tokens.foreground)
                            .child("Editor Component — Styled Trait"),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("The Editor below is styled directly with GPUI styling methods (.bg, .border_2, .rounded_lg, .shadow_lg, .w_full)."),
                    ),
            )
            .child(
                // The Editor component implements Styled, so all GPUI styling
                // methods apply directly here.
                Editor::new(&self.editor_state)
                    .min_lines(24)
                    .show_line_numbers(true, cx)
                    .show_border(false)
                    .bg(theme.tokens.card)
                    .border_2()
                    .border_color(theme.tokens.border)
                    .rounded_lg()
                    .shadow_lg()
                    .w_full()
                    .p(px(12.0)),
            )
    }
}
