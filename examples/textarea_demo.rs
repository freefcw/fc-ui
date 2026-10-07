//! Interactive textarea demo with full editing capabilities

use fc_ui::components::input::InputVariant;
use fc_ui::prelude::*;
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
                        title: Some("Interactive Textarea Demo".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(800.0), px(600.0)),
                    })),
                    window_min_size: Some(size(px(400.0), px(300.0))),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| TextareaDemo::new(cx)),
            )
            .unwrap();
        });
}

struct TextareaDemo {
    textarea_state: Entity<TextareaState>,
}

impl TextareaDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let textarea_state = cx.new(|cx| TextareaState::new(cx));

        cx.subscribe(
            &textarea_state,
            |_this, _emitter, event: &TextareaEvent, _cx| match event {
                TextareaEvent::Change => eprintln!("Text changed"),
                TextareaEvent::Enter => eprintln!("Enter pressed"),
                TextareaEvent::ShiftEnter => eprintln!("Shift+Enter pressed"),
                _ => {}
            },
        )
        .detach();

        Self { textarea_state }
    }
}

impl Render for TextareaDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.tokens.background)
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.tokens.foreground)
                    .child("Interactive Textarea Demo"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.tokens.muted_foreground)
                    .child("Full editing: type, arrow keys, selection, copy/paste, IME support."),
            )
            .child(
                Textarea::new(&self.textarea_state)
                    .placeholder("Type your text here...")
                    .rows(8)
                    .variant(InputVariant::Outline)
                    .on_change(|value, _cx| {
                        eprintln!("Text changed: {}", value);
                    })
                    .on_shift_enter(|value, _cx| {
                        eprintln!("Shift+Enter: {}", value);
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(
                        Button::new("clear-btn", "Clear")
                            .variant(ButtonVariant::Outline)
                            .on_click(cx.listener(|this, _event, window, cx| {
                                this.textarea_state.update(cx, |state, cx| {
                                    state.set_value("", window, cx);
                                });
                            })),
                    )
                    .child(
                        Button::new("sample-btn", "Insert Sample")
                            .variant(ButtonVariant::Outline)
                            .on_click(cx.listener(|this, _event, window, cx| {
                                let sample =
                                    "This is sample text\nwith multiple lines\nand some content.";
                                this.textarea_state.update(cx, |state, cx| {
                                    state.set_value(sample, window, cx);
                                });
                            })),
                    ),
            )
            .child(
                div()
                    .mt_4()
                    .p_3()
                    .bg(theme.tokens.muted.opacity(0.3))
                    .rounded(theme.tokens.radius_md)
                    .child(
                        div()
                            .text_sm()
                            .font_family(theme.tokens.font_mono.clone())
                            .text_color(theme.tokens.foreground)
                            .child(format!(
                                "Current content:\n{}",
                                self.textarea_state.read(cx).content()
                            )),
                    ),
            )
    }
}
