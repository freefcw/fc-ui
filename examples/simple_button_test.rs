use fc_ui::prelude::*;

fn main() {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("Simple Button Test".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point {
                        x: px(100.0),
                        y: px(100.0),
                    },
                    size: Size {
                        width: px(400.0),
                        height: px(300.0),
                    },
                })),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| SimpleButtonApp::new(window, cx)),
        )
        .unwrap();
    });
}

struct SimpleButtonApp {
    click_count: usize,
}

impl SimpleButtonApp {
    fn new(_window: &mut Window, _cx: &mut App) -> Self {
        Self { click_count: 0 }
    }
}

impl Render for SimpleButtonApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .bg(theme.tokens.background)
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(20.0))
            .child(
                div()
                    .text_size(px(24.0))
                    .text_color(theme.tokens.foreground)
                    .child(format!("Clicked {} times", self.click_count)),
            )
            .child(
                // State lives on the root view (per the demo.rs note) so the
                // click handler can use `cx.listener` and call `cx.notify()`
                // to re-render.
                Button::new("click-btn", "Click Me!").on_click(cx.listener(
                    |this, _event, _window, cx| {
                        this.click_count += 1;
                        println!("Button clicked! Count: {}", this.click_count);
                        cx.notify();
                    },
                )),
            )
    }
}
