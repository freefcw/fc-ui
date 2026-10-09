use fc_ui::{
    components::input::InputVariant, components::scrollable::scrollable_vertical,
    components::textarea::Textarea, prelude::*,
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
                        title: Some("Textarea Styled Trait Demo".into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(900.0), px(900.0)),
                    })),
                    ..Default::default()
                },
                |_, cx| cx.new(|cx| TextareaStyledDemo::new(cx)),
            )
            .unwrap();
        });
}

struct TextareaStyledDemo {
    states: Vec<Entity<TextareaState>>,
}

impl TextareaStyledDemo {
    fn new(cx: &mut Context<Self>) -> Self {
        let states: Vec<Entity<TextareaState>> = (0..21)
            .map(|_| cx.new(|cx| TextareaState::new(cx)))
            .collect();
        Self { states }
    }
}

impl Render for TextareaStyledDemo {
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
                        .p(px(32.0))
                        .gap(px(32.0))
            .child(
                VStack::new()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(24.0))
                            .font_weight(FontWeight::BOLD)
                            .child("Textarea Styled Trait Customization Demo")
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.tokens.muted_foreground)
                            .child("Demonstrating full GPUI styling control via Styled trait")
                    )
            )
            // 1. Custom Width Examples
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("1. Custom Width (via Styled trait)")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[0])
                                    .placeholder("Default width textarea")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                            )
                            .child(
                                Textarea::new(&self.states[1])
                                    .placeholder("Full width textarea")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .w_full()
                            )
                            .child(
                                Textarea::new(&self.states[2])
                                    .placeholder("Custom width (600px)")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .w(px(600.0))
                            )
                    )
            )
            // 2. Custom Padding
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("2. Custom Padding")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[3])
                                    .placeholder("Default padding")
                                    .variant(InputVariant::Outline)
                                    .rows(3)
                            )
                            .child(
                                Textarea::new(&self.states[4])
                                    .placeholder("Extra padding - p_4()")
                                    .variant(InputVariant::Outline)
                                    .rows(3)
                                    .p_4()
                            )
                            .child(
                                Textarea::new(&self.states[5])
                                    .placeholder("More padding - p_8()")
                                    .variant(InputVariant::Outline)
                                    .rows(3)
                                    .p_8()
                            )
                    )
            )
            // 3. Custom Background Colors
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("3. Custom Background Colors")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[6])
                                    .placeholder("Blue background textarea")
                                    .variant(InputVariant::Ghost)
                                    .rows(4)
                                    .bg(rgb(0x1e3a8a))
                                    .text_color(gpui::white())
                            )
                            .child(
                                Textarea::new(&self.states[7])
                                    .placeholder("Purple background textarea")
                                    .variant(InputVariant::Ghost)
                                    .rows(4)
                                    .bg(rgb(0x581c87))
                                    .text_color(gpui::white())
                            )
                    )
            )
            // 4. Custom Border Radius
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("4. Custom Border Radius")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[8])
                                    .placeholder("No radius (sharp corners)")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .rounded(px(0.0))
                            )
                            .child(
                                Textarea::new(&self.states[9])
                                    .placeholder("Large radius (20px)")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .rounded(px(20.0))
                            )
                    )
            )
            // 5. Custom Height and Rows
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("5. Custom Height and Rows")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[10])
                                    .placeholder("Small textarea (2 rows)")
                                    .variant(InputVariant::Default)
                                    .rows(2)
                            )
                            .child(
                                Textarea::new(&self.states[11])
                                    .placeholder("Large textarea (6 rows)")
                                    .variant(InputVariant::Default)
                                    .rows(6)
                            )
                            .child(
                                Textarea::new(&self.states[12])
                                    .placeholder("Custom min height via Styled trait")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .min_h(px(150.0))
                            )
                    )
            )
            // 6. Combined Styling
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("6. Combined Styling (Multiple Styled Trait Methods)")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[13])
                                    .placeholder("Fully customized textarea with shadow and gradient")
                                    .variant(InputVariant::Ghost)
                                    .rows(4)
                                    .w_full()
                                    .p(px(20.0))
                                    .bg(rgb(0x047857))
                                    .text_color(gpui::white())
                                    .rounded(px(12.0))
                                    .shadow_lg()
                            )
                            .child(
                                Textarea::new(&self.states[14])
                                    .placeholder("Ultra custom with border and special styling")
                                    .variant(InputVariant::Outline)
                                    .rows(5)
                                    .w_full()
                                    .px(px(24.0))
                                    .py(px(16.0))
                                    .bg(hsla(43.0 / 360.0, 0.96, 0.56, 0.2))
                                    .rounded(px(16.0))
                                    .border_2()
                                    .border_color(rgb(0xfbbf24))
                            )
                    )
            )
            // 7. Different Variants with Styling
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("7. Different Variants with Custom Styling")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[15])
                                    .placeholder("Default variant with custom width")
                                    .variant(InputVariant::Default)
                                    .rows(3)
                                    .w(px(500.0))
                            )
                            .child(
                                Textarea::new(&self.states[16])
                                    .placeholder("Outline variant with custom background")
                                    .variant(InputVariant::Outline)
                                    .rows(3)
                                    .bg(hsla(0.0, 0.0, 0.95, 1.0))
                            )
                            .child(
                                Textarea::new(&self.states[17])
                                    .placeholder("Ghost variant with custom border")
                                    .variant(InputVariant::Ghost)
                                    .rows(3)
                                    .border_1()
                                    .border_color(theme.tokens.primary)
                            )
                    )
            )
            // 8. Validation & Character Count
            .child(
                VStack::new()
                    .gap(px(16.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("8. Validation & Character Count")
                    )
                    .child(
                        VStack::new()
                            .gap(px(12.0))
                            .child(
                                Textarea::new(&self.states[18])
                                    .placeholder("Required field (blur to validate)")
                                    .required(true)
                                    .helper_text("This field is required")
                                    .rows(3)
                            )
                            .child(
                                Textarea::new(&self.states[19])
                                    .placeholder("Max 140 characters with live count")
                                    .max_length(140)
                                    .show_character_count(true)
                                    .rows(3)
                            )
                            .child(
                                Textarea::new(&self.states[20])
                                    .placeholder("At least 10 characters")
                                    .min_length(10)
                                    .helper_text("Tell us a bit more (10+ characters)")
                                    .rows(3)
                            )
                    )
            )
            // Info Box
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
                            .child("Methods used: .w_full(), .w(), .p_4(), .p_8(), .p(), .px(), .py(), .bg(), .rounded(), .border_1(), .border_2(), .shadow_lg(), .min_h()")
                    )
            )
                )
            )
    }
}
