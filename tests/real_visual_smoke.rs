#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
)))]
fn main() {
    println!("real_visual_smoke is not supported on this platform; skipping");
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
fn main() {
    if !std::env::args().any(|arg| arg == "--ignored" || arg == "--include-ignored") {
        println!("real_visual_smoke is ignored by default; pass `-- --ignored` to run it");
        return;
    }

    if let Err(error) = run() {
        eprintln!("real_visual_smoke failed: {error:?}");
        std::process::exit(1);
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "freebsd",
    target_os = "windows"
))]
fn run() -> anyhow::Result<()> {
    use fc_ui::{
        components::input::{Input, InputState},
        overlays::dialog::{Dialog, DialogSize},
        overlays::popover::{Popover, PopoverContent},
        prelude::{
            BarChart, BarChartSeries, Button, Checkbox, GlassIntensity, GlassMorphism, LineChart,
            LineChartPoint, LineChartSeries, RangeSlider, RangeSliderState, Select, SelectOption,
            Slider, SliderState,
        },
    };
    use gpui::{
        div, point, px, rgb, size, AppContext as _, Context, IntoElement, Modifiers, MouseButton,
        MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement as _, PlatformInput,
        RealVisualTestContext, Render, Styled as _, VisualRenderArtifact, VisualTestCapabilities,
        Window,
    };
    use std::{cell::RefCell, rc::Rc, time::Duration};

    struct FormSmokeView {
        input_state: gpui::Entity<InputState>,
    }

    impl Render for FormSmokeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().bg(rgb(0xf8fafc)).p(px(24.0)).child(
                div()
                    .w(px(360.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .child(
                        Input::new(&self.input_state)
                            .aria_label("Email")
                            .placeholder("name@example.com")
                            .value("alice@example.com"),
                    )
                    .child(
                        Checkbox::new("visual-updates")
                            .label("Product updates")
                            .checked(true),
                    )
                    .child(Button::new("visual-save", "Save settings")),
            )
        }
    }

    struct PopoverSmokeView;

    impl Render for PopoverSmokeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().bg(rgb(0xf8fafc)).p(px(24.0)).child(
                Popover::new("visual-popover")
                    .trigger(Button::new("visual-popover-trigger", "Choose theme").w(px(180.0)))
                    .content(|window, cx| {
                        let select = cx.new(|cx| {
                            Select::new(cx)
                                .options(vec![
                                    SelectOption::new("light", "Light"),
                                    SelectOption::new("dark", "Dark"),
                                ])
                                .selected_index(Some(0))
                                .aria_label("Theme")
                        });
                        cx.new(|cx| {
                            PopoverContent::new(window, cx, move |_, _| {
                                div().w(px(260.0)).child(select.clone()).into_any_element()
                            })
                        })
                    }),
            )
        }
    }

    struct ChartSmokeView;

    impl Render for ChartSmokeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let labels = vec!["Q1", "Q2", "Q3", "Q4"];
            let series = vec![
                BarChartSeries::new("Revenue", vec![42.0, 58.0, 51.0, 67.0])
                    .color(rgb(0x3b82f6).into()),
                BarChartSeries::new("Costs", vec![18.0, 25.0, 21.0, 30.0])
                    .color(rgb(0x22c55e).into()),
            ];
            div()
                .size_full()
                .bg(rgb(0xffffff))
                .p(px(24.0))
                .flex()
                .flex_col()
                .gap(px(24.0))
                .child(
                    BarChart::multi_series(labels, series)
                        .grouped()
                        .show_grid(true)
                        .h(px(160.0)),
                )
                .child(LineChart::new(vec![LineChartSeries::new(
                    "Trend",
                    vec![
                        LineChartPoint::new(0.0, 3.0).label("Jan"),
                        LineChartPoint::new(1.0, 7.0).label("Feb"),
                        LineChartPoint::new(2.0, 5.0).label("Mar"),
                        LineChartPoint::new(3.0, 9.0).label("Apr"),
                    ],
                )]))
        }
    }

    struct SliderSmokeView {
        slider: gpui::Entity<SliderState>,
        range: gpui::Entity<RangeSliderState>,
    }

    impl Render for SliderSmokeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(rgb(0xf8fafc))
                .p(px(24.0))
                .flex()
                .flex_col()
                .gap(px(24.0))
                .child(Slider::new(self.slider.clone()))
                .child(RangeSlider::new(self.range.clone()))
        }
    }

    struct OverlaySmokeView {
        dialog: gpui::Entity<Dialog>,
    }

    impl Render for OverlaySmokeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(rgb(0x172033))
                .child(
                    div()
                        .absolute()
                        .left(px(40.0))
                        .top(px(40.0))
                        .w(px(320.0))
                        .h(px(180.0))
                        .child(
                            GlassMorphism::new()
                                .intensity(GlassIntensity::Heavy)
                                .noise(true)
                                .size_full()
                                .child(
                                    div()
                                        .p(px(24.0))
                                        .text_color(rgb(0xffffff))
                                        .child("Transparent surface"),
                                ),
                        ),
                )
                .child(self.dialog.clone())
        }
    }

    fn assert_scene_and_image(
        scene: VisualRenderArtifact,
        image: &image::RgbaImage,
        label: &str,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            scene.is_nonblank() && scene.quads > 0,
            "{label} produced an empty scene: {scene:?}"
        );
        anyhow::ensure!(
            image.width() > 0 && image.height() > 0,
            "{label} produced an empty screenshot"
        );

        let Some(first_pixel) = image.pixels().next().copied() else {
            anyhow::bail!("{label} screenshot has no pixels");
        };
        let nontransparent = image.pixels().filter(|pixel| pixel[3] > 0).count();
        let different = image
            .pixels()
            .filter(|pixel| **pixel != first_pixel)
            .count();
        anyhow::ensure!(
            nontransparent > 0,
            "{label} screenshot is fully transparent"
        );
        anyhow::ensure!(different > 0, "{label} screenshot is a solid color");
        Ok(())
    }

    fn draw_and_capture(
        cx: &mut RealVisualTestContext,
        window: gpui::AnyWindowHandle,
    ) -> anyhow::Result<(VisualRenderArtifact, image::RgbaImage)> {
        let artifact = cx.update_window(window, |_, window, app| {
            let clear = window.draw(app);
            let artifact = window.visual_render_artifact();
            window.present_for_visual_test();
            clear.clear();
            artifact
        })?;
        let image = cx.capture_screenshot(window)?;
        Ok((artifact, image))
    }

    let capabilities = VisualTestCapabilities::detect();
    let require_real_visual =
        std::env::var_os("CI").is_some() || std::env::var_os("FC_UI_REQUIRE_REAL_VISUAL").is_some();
    let Some(cx) = RealVisualTestContext::new_if_supported() else {
        anyhow::ensure!(
            !require_real_visual,
            "real visual renderer is required but not available"
        );
        println!("real visual renderer is not available; skipping");
        return Ok(());
    };
    anyhow::ensure!(capabilities.screenshot_capture);

    let outcome = Rc::new(RefCell::new(None));
    let outcome_in_run = outcome.clone();
    cx.run(move |cx| {
        let result = (|| -> anyhow::Result<()> {
            fc_ui::init(&mut cx.app.borrow_mut());

            let form_window = cx.open_offscreen_window(size(px(640.0), px(480.0)), |_, app| {
                let input_state = app.new(InputState::new);
                app.new(|_| FormSmokeView { input_state })
            })?;
            let form_window = form_window.into();
            let (form_scene, form_image) = draw_and_capture(cx, form_window)?;
            assert_scene_and_image(form_scene, &form_image, "form scene")?;

            let popover_window = cx.open_offscreen_window(size(px(640.0), px(480.0)), |_, app| {
                app.new(|_| PopoverSmokeView)
            })?;
            let popover_window = popover_window.into();
            let (closed_scene, _) = draw_and_capture(cx, popover_window)?;

            let trigger_center = point(px(114.0), px(44.0));
            cx.update_window(popover_window, |_, window, app| {
                window.dispatch_event(
                    PlatformInput::MouseMove(MouseMoveEvent {
                        position: trigger_center,
                        pressed_button: None,
                        modifiers: Modifiers::none(),
                    }),
                    app,
                );
                window.dispatch_event(
                    PlatformInput::MouseDown(MouseDownEvent {
                        position: trigger_center,
                        button: MouseButton::Left,
                        modifiers: Modifiers::none(),
                        click_count: 1,
                        first_mouse: false,
                    }),
                    app,
                );
                window.dispatch_event(
                    PlatformInput::MouseUp(MouseUpEvent {
                        position: trigger_center,
                        button: MouseButton::Left,
                        modifiers: Modifiers::none(),
                        click_count: 1,
                    }),
                    app,
                );
            })?;
            cx.run_until_parked();
            cx.advance_clock(Duration::from_millis(200));

            let (open_scene, open_image) = draw_and_capture(cx, popover_window)?;
            anyhow::ensure!(
                open_scene.paint_operations > closed_scene.paint_operations,
                "opening Popover with Select should add paint operations: closed={closed_scene:?}, open={open_scene:?}"
            );
            assert_scene_and_image(open_scene, &open_image, "popover scene")?;

            let overlay_window = cx.open_offscreen_window(size(px(640.0), px(480.0)), |_, app| {
                let dialog = app.new(|cx| {
                    Dialog::new(cx)
                        .size(DialogSize::Sm)
                        .title("Transparent overlay")
                        .description("The dialog is rendered above a glass surface.")
                        .child(|_, _| div().h(px(40.0)).child("Dialog content"))
                });
                app.new(|_| OverlaySmokeView { dialog })
            })?;
            let overlay_window = overlay_window.into();
            let (overlay_scene, overlay_image) = draw_and_capture(cx, overlay_window)?;
            assert_scene_and_image(overlay_scene, &overlay_image, "transparent overlay scene")?;

            // Charts (shared grid/legend/label foundations) must produce a
            // populated, non-solid scene.
            let chart_window = cx.open_offscreen_window(size(px(640.0), px(480.0)), |_, app| {
                app.new(|_| ChartSmokeView)
            })?;
            let chart_window = chart_window.into();
            let (chart_scene, chart_image) = draw_and_capture(cx, chart_window)?;
            assert_scene_and_image(chart_scene, &chart_image, "chart scene")?;

            // Sliders (shared track/thumb constructors) must render both the
            // single- and dual-thumb variants.
            let slider_window = cx.open_offscreen_window(size(px(640.0), px(480.0)), |_, app| {
                let slider = app.new(|cx| {
                    let mut state = SliderState::new(cx);
                    state.set_value(40.0, cx);
                    state
                });
                let range = app.new(|cx| {
                    let mut state = RangeSliderState::new(cx);
                    state.set_start_value(20.0, cx);
                    state.set_end_value(70.0, cx);
                    state
                });
                app.new(|_| SliderSmokeView { slider, range })
            })?;
            let slider_window = slider_window.into();
            let (slider_scene, slider_image) = draw_and_capture(cx, slider_window)?;
            assert_scene_and_image(slider_scene, &slider_image, "slider scene")?;
            Ok(())
        })();
        *outcome_in_run.borrow_mut() = Some(result);
        cx.quit();
    });

    let result = outcome.borrow_mut().take().unwrap_or_else(|| {
        Err(anyhow::anyhow!(
            "real visual smoke did not report an outcome"
        ))
    });
    result
}
