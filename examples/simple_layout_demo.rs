use fc_ui::prelude::*;

fn main() {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions {
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Layout System Demo".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1200.0), px(800.0)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| SimpleLayoutDemo::new(window, cx)),
        )
        .unwrap();
    });
}

struct SimpleLayoutDemo;

impl SimpleLayoutDemo {
    fn new(_window: &mut Window, cx: &mut App) -> Self {
        let theme = Theme::dark();
        install_theme(cx, theme.clone());
        Self
    }
}

impl Render for SimpleLayoutDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .bg(theme.tokens.background)
            .size_full()
            .flex()
            .flex_col()
            .child(
                // Header - Direct child of root flex container
                HStack::new()
                    .padding(24.0)
                    .align(Align::Center)
                    .justify(Justify::Between)
                    .child(
                        div()
                            .text_size(px(24.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.tokens.foreground)
                            .child("Layout System Demo"),
                    )
                    .child(
                        HStack::new()
                            .spacing(8.0)
                            .child(Button::new("docs-btn", "Docs").variant(ButtonVariant::Ghost))
                            .child(
                                Button::new("settings-btn", "Settings")
                                    .variant(ButtonVariant::Ghost),
                            ),
                    ),
            )
            .child(
                // Main content - Direct child of root flex container with .flex_1()
                div()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(scrollable_vertical(
                    div()
                        .flex()
                        .flex_col()
                        .p(px(24.0))
                        .gap(px(32.0))
                        // VStack Demo
                        .child(section_card(
                            theme.clone(),
                            "VStack - Vertical Stacking",
                            VStack::new()
                                .spacing(12.0)
                                .child(colored_box(theme.clone(), "Item 1", theme.tokens.primary))
                                .child(colored_box(theme.clone(), "Item 2", theme.tokens.secondary))
                                .child(colored_box(theme.clone(), "Item 3", theme.tokens.accent)),
                        ))
                        // HStack Demo
                        .child(section_card(
                            theme.clone(),
                            "HStack - Horizontal Stacking",
                            VStack::new()
                                .spacing(16.0)
                                .child(
                                    VStack::new()
                                        .spacing(8.0)
                                        .child(label(theme.clone(), "Justify: Between"))
                                        .child(
                                            HStack::new()
                                                .justify(Justify::Between)
                                                .child(colored_box(
                                                    theme.clone(),
                                                    "Left",
                                                    theme.tokens.primary,
                                                ))
                                                .child(colored_box(
                                                    theme.clone(),
                                                    "Right",
                                                    theme.tokens.primary,
                                                )),
                                        ),
                                )
                                .child(
                                    VStack::new()
                                        .spacing(8.0)
                                        .child(label(theme.clone(), "Justify: Center"))
                                        .child(
                                            HStack::new()
                                                .justify(Justify::Center)
                                                .spacing(12.0)
                                                .child(colored_box(
                                                    theme.clone(),
                                                    "A",
                                                    theme.tokens.secondary,
                                                ))
                                                .child(colored_box(
                                                    theme.clone(),
                                                    "B",
                                                    theme.tokens.secondary,
                                                ))
                                                .child(colored_box(
                                                    theme.clone(),
                                                    "C",
                                                    theme.tokens.secondary,
                                                )),
                                        ),
                                ),
                        ))
                        // Grid Demo
                        .child(section_card(
                            theme.clone(),
                            "Grid - Grid Layout",
                            Grid::new()
                                .columns(3)
                                .gap(16.0)
                                .child(colored_box(theme.clone(), "Grid 1", theme.tokens.primary))
                                .child(colored_box(theme.clone(), "Grid 2", theme.tokens.secondary))
                                .child(colored_box(theme.clone(), "Grid 3", theme.tokens.accent))
                                .child(colored_box(
                                    theme.clone(),
                                    "Grid 4",
                                    theme.tokens.destructive,
                                ))
                                .child(colored_box(theme.clone(), "Grid 5", theme.tokens.primary))
                                .child(colored_box(
                                    theme.clone(),
                                    "Grid 6",
                                    theme.tokens.secondary,
                                )),
                        ))
                        // Flow Demo
                        .child(section_card(
                            theme.clone(),
                            "Flow - Wrapping Layout",
                            Flow::new()
                                .spacing(8.0)
                                .child(tag(theme.clone(), "React"))
                                .child(tag(theme.clone(), "Vue"))
                                .child(tag(theme.clone(), "Angular"))
                                .child(tag(theme.clone(), "Svelte"))
                                .child(tag(theme.clone(), "Solid"))
                                .child(tag(theme.clone(), "Qwik"))
                                .child(tag(theme.clone(), "Preact"))
                                .child(tag(theme.clone(), "Alpine")),
                        ))
                        // Cluster Demo
                        .child(section_card(
                            theme.clone(),
                            "Cluster - Inline Grouping",
                            VStack::new()
                                .spacing(16.0)
                                .child(
                                    Cluster::new()
                                        .spacing(8.0)
                                        .align(Align::Center)
                                        .child(avatar(theme.clone(), "JD", theme.tokens.primary))
                                        .child(label(theme.clone(), "John Doe"))
                                        .child(tag(theme.clone(), "Admin")),
                                )
                                .child(
                                    Cluster::new()
                                        .spacing(8.0)
                                        .align(Align::Center)
                                        .child(avatar(theme.clone(), "JS", theme.tokens.secondary))
                                        .child(label(theme.clone(), "Jane Smith"))
                                        .child(tag(theme.clone(), "User")),
                                ),
                        ))
                        // Spacer Demo
                        .child(section_card(
                            theme.clone(),
                            "Spacer - Flexible Spacing",
                            VStack::new()
                                .spacing(16.0)
                                .child(
                                    HStack::new()
                                        .child(Button::new("left-btn", "Left"))
                                        .child(Spacer::new())
                                        .child(Button::new("right-btn", "Right")),
                                )
                                .child(
                                    HStack::new()
                                        .child(Button::new("first-btn", "First"))
                                        .child(Spacer::new())
                                        .child(Button::new("middle-btn", "Middle"))
                                        .child(Spacer::new())
                                        .child(Button::new("last-btn", "Last")),
                                ),
                        ))
                        // Nested Layouts
                        .child(section_card(
                            theme.clone(),
                            "Nested Layouts - Complex Compositions",
                            HStack::new()
                                .spacing(16.0)
                                .child(
                                    VStack::new()
                                        .spacing(12.0)
                                        .child(colored_box(
                                            theme.clone(),
                                            "Header",
                                            theme.tokens.primary,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Content",
                                            theme.tokens.muted,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Footer",
                                            theme.tokens.accent,
                                        )),
                                )
                                .child(
                                    Grid::new()
                                        .columns(2)
                                        .gap(12.0)
                                        .child(colored_box(
                                            theme.clone(),
                                            "1",
                                            theme.tokens.secondary,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "2",
                                            theme.tokens.secondary,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "3",
                                            theme.tokens.secondary,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "4",
                                            theme.tokens.secondary,
                                        )),
                                ),
                        ))
                        // More examples to ensure scrolling
                        .child(section_card(
                            theme.clone(),
                            "Custom Scrollbar",
                            VStack::new()
                                .spacing(12.0)
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .text_color(theme.tokens.muted_foreground)
                                        .child(
                                            "This demo uses our custom animated scrollbar with:",
                                        ),
                                )
                                .child(colored_box(
                                    theme.clone(),
                                    "✓ Hover & drag states",
                                    theme.tokens.primary,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "✓ Auto fade-in/fade-out",
                                    theme.tokens.secondary,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "✓ Click-to-jump support",
                                    theme.tokens.accent,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "✓ Smooth animations",
                                    theme.tokens.destructive,
                                )),
                        ))
                        // Additional Grid examples
                        .child(section_card(
                            theme.clone(),
                            "Grid - 4 Columns",
                            Grid::new()
                                .columns(4)
                                .gap(12.0)
                                .child(colored_box(theme.clone(), "A", theme.tokens.primary))
                                .child(colored_box(theme.clone(), "B", theme.tokens.secondary))
                                .child(colored_box(theme.clone(), "C", theme.tokens.accent))
                                .child(colored_box(theme.clone(), "D", theme.tokens.destructive))
                                .child(colored_box(theme.clone(), "E", theme.tokens.primary))
                                .child(colored_box(theme.clone(), "F", theme.tokens.secondary))
                                .child(colored_box(theme.clone(), "G", theme.tokens.accent))
                                .child(colored_box(theme.clone(), "H", theme.tokens.destructive)),
                        ))
                        // More VStack examples
                        .child(section_card(
                            theme.clone(),
                            "VStack with Different Alignments",
                            HStack::new()
                                .spacing(16.0)
                                .child(
                                    VStack::new()
                                        .spacing(8.0)
                                        .align(Align::Start)
                                        .child(label(theme.clone(), "Align: Start"))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Item 1",
                                            theme.tokens.primary,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Item 2",
                                            theme.tokens.secondary,
                                        )),
                                )
                                .child(
                                    VStack::new()
                                        .spacing(8.0)
                                        .align(Align::Center)
                                        .child(label(theme.clone(), "Align: Center"))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Item 1",
                                            theme.tokens.accent,
                                        ))
                                        .child(colored_box(
                                            theme.clone(),
                                            "Item 2",
                                            theme.tokens.destructive,
                                        )),
                                ),
                        ))
                        // Final section
                        .child(section_card(
                            theme.clone(),
                            "Scroll to See More!",
                            VStack::new()
                                .spacing(12.0)
                                .child(colored_box(
                                    theme.clone(),
                                    "Try scrolling with mouse wheel",
                                    theme.tokens.primary,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "Hover over the scrollbar",
                                    theme.tokens.secondary,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "Click and drag the thumb",
                                    theme.tokens.accent,
                                ))
                                .child(colored_box(
                                    theme.clone(),
                                    "You've reached the end!",
                                    theme.tokens.destructive,
                                )),
                        )),
                )),
            )
            .child(
                // Footer - Direct child of root flex container
                HStack::new().padding(16.0).justify(Justify::Center).child(
                    div()
                        .text_size(px(12.0))
                        .text_color(use_theme(cx).tokens.muted_foreground)
                        .child("Layout System: Semantic • Composable • Type-Safe"),
                ),
            )
    }
}

// Helper functions
fn section_card(
    theme: Theme,
    title: impl Into<SharedString>,
    content: impl IntoElement,
) -> impl IntoElement {
    let title: SharedString = title.into();

    VStack::new()
        .spacing(12.0)
        .child(
            div()
                .text_size(px(18.0))
                .font_weight(FontWeight::BOLD)
                .text_color(theme.tokens.foreground)
                .child(title),
        )
        .child(
            div()
                .bg(theme.tokens.card)
                .border_1()
                .border_color(theme.tokens.border)
                .rounded(theme.tokens.radius_lg)
                .p(px(24.0))
                .child(content),
        )
}

fn colored_box(theme: Theme, text: impl Into<SharedString>, color: Hsla) -> impl IntoElement {
    let text: SharedString = text.into();

    div()
        .bg(color)
        .rounded(theme.tokens.radius_md)
        .p(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.tokens.primary_foreground)
        .font_weight(FontWeight::MEDIUM)
        .child(text)
}

fn tag(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    let text: SharedString = text.into();

    div()
        .bg(theme.tokens.secondary)
        .rounded(theme.tokens.radius_md)
        .px(px(12.0))
        .py(px(6.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.tokens.secondary_foreground)
        .child(text)
}

fn avatar(theme: Theme, initials: impl Into<SharedString>, color: Hsla) -> impl IntoElement {
    let initials: SharedString = initials.into();

    div()
        .size(px(32.0))
        .rounded(px(16.0))
        .bg(color)
        .flex()
        .items_center()
        .justify_center()
        .text_color(theme.tokens.primary_foreground)
        .text_size(px(12.0))
        .font_weight(FontWeight::BOLD)
        .child(initials)
}

fn label(theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
    let text: SharedString = text.into();

    div()
        .text_size(px(14.0))
        .text_color(theme.tokens.foreground)
        .child(text)
}
