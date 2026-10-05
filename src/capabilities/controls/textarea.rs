//! Textarea component - Multi-line text input component with full editing capabilities.

use crate::capabilities::controls::input::InputVariant;
pub use crate::capabilities::controls::textarea_state::wire_actions;
use crate::capabilities::controls::textarea_state::TextareaEvent;
use crate::capabilities::controls::textarea_state::TextareaState;
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

/// Initialize textarea component with key bindings
pub fn init(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "textarea") {
        return;
    }
    crate::capabilities::controls::textarea_state::init(cx);
}

#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
    placeholder: SharedString,
    variant: InputVariant,
    disabled: bool,
    error: bool,
    rows: usize,
    min_rows: Option<usize>,
    max_rows: Option<usize>,
    auto_grow: bool,
    initial_value: Option<SharedString>,

    on_change: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_enter: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_shift_enter: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_focus: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_blur: Option<Rc<dyn Fn(SharedString, &mut App)>>,

    style: StyleRefinement,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
            placeholder: "".into(),
            variant: InputVariant::Default,
            disabled: false,
            error: false,
            rows: 3,
            min_rows: None,
            max_rows: None,
            auto_grow: false,
            initial_value: None,
            on_change: None,
            on_enter: None,
            on_shift_enter: None,
            on_focus: None,
            on_blur: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.initial_value = Some(value.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn variant(mut self, variant: InputVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = rows.max(1);
        self
    }

    pub fn min_rows(mut self, min_rows: usize) -> Self {
        self.min_rows = Some(min_rows.max(1));
        self
    }

    pub fn max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = Some(max_rows.max(1));
        self
    }

    pub fn auto_grow(mut self, auto_grow: bool) -> Self {
        self.auto_grow = auto_grow;
        self
    }

    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_change = Some(Rc::new(callback));
        self
    }

    pub fn on_enter<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_enter = Some(Rc::new(callback));
        self
    }

    pub fn on_shift_enter<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_shift_enter = Some(Rc::new(callback));
        self
    }

    pub fn on_focus<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_focus = Some(Rc::new(callback));
        self
    }

    pub fn on_blur<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_blur = Some(Rc::new(callback));
        self
    }

    fn install_event_subscription(&self, cx: &mut App) {
        let on_change = self.on_change.clone();
        let on_enter = self.on_enter.clone();
        let on_shift_enter = self.on_shift_enter.clone();
        let on_focus = self.on_focus.clone();
        let on_blur = self.on_blur.clone();

        let has_callbacks = on_change.is_some()
            || on_enter.is_some()
            || on_shift_enter.is_some()
            || on_focus.is_some()
            || on_blur.is_some();
        let subscription = has_callbacks.then(|| {
            cx.subscribe(
                &self.state,
                move |emitter, event: &TextareaEvent, cx| match event {
                    TextareaEvent::Change => {
                        if let Some(callback) = on_change.as_ref() {
                            callback(emitter.read(cx).content.clone(), cx);
                        }
                    }
                    TextareaEvent::Enter => {
                        if let Some(callback) = on_enter.as_ref() {
                            callback(emitter.read(cx).content.clone(), cx);
                        }
                    }
                    TextareaEvent::ShiftEnter => {
                        if let Some(callback) = on_shift_enter.as_ref() {
                            callback(emitter.read(cx).content.clone(), cx);
                        }
                    }
                    TextareaEvent::Focus => {
                        if let Some(callback) = on_focus.as_ref() {
                            callback(emitter.read(cx).content.clone(), cx);
                        }
                    }
                    TextareaEvent::Blur => {
                        if let Some(callback) = on_blur.as_ref() {
                            callback(emitter.read(cx).content.clone(), cx);
                        }
                    }
                    TextareaEvent::Tab | TextareaEvent::ShiftTab => {}
                },
            )
        });

        self.state.update(cx, |state, _| {
            state.replace_event_subscription(subscription);
        });
    }

    fn calculate_height(&self, window: &Window, cx: &App) -> Pixels {
        let line_height = window.line_height().to_f64() as f32;
        let padding_y = 8.0;
        let base_height = self.rows as f32 * line_height + padding_y * 2.0;

        if self.auto_grow {
            let visual_lines = self.state.read(cx).visual_line_count_for_layout().max(1);
            let content_height = visual_lines as f32 * line_height + padding_y * 2.0;

            let final_height = if let Some(min_rows) = self.min_rows {
                let min_height = min_rows as f32 * line_height + padding_y * 2.0;
                content_height.max(min_height)
            } else {
                content_height.max(base_height)
            };

            if let Some(max_rows) = self.max_rows {
                let max_height = max_rows as f32 * line_height + padding_y * 2.0;
                px(final_height.min(max_height))
            } else {
                px(final_height)
            }
        } else {
            px(base_height)
        }
    }
}

impl Styled for Textarea {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Textarea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let height = self.calculate_height(window, cx);

        // Update state with properties
        self.state.update(cx, |state, cx| {
            state.disabled = self.disabled;
            state.placeholder = self.placeholder.clone();

            if !state.initialized {
                state.initialized = true;
                if let Some(value) = self.initial_value.clone() {
                    state.set_value(value, window, cx);
                }
            }
        });

        self.install_event_subscription(cx);

        let (bg_color, border_color, text_color) = if self.disabled {
            (
                theme.tokens.muted.opacity(0.5),
                theme.tokens.border,
                theme.tokens.muted_foreground,
            )
        } else if self.error {
            match self.variant {
                InputVariant::Default => (
                    theme.tokens.background,
                    theme.tokens.destructive,
                    theme.tokens.foreground,
                ),
                InputVariant::Outline => (
                    theme.tokens.background,
                    theme.tokens.destructive,
                    theme.tokens.foreground,
                ),
                InputVariant::Ghost => (
                    gpui::transparent_black(),
                    theme.tokens.destructive.opacity(0.3),
                    theme.tokens.foreground,
                ),
            }
        } else {
            match self.variant {
                InputVariant::Default => (
                    theme.tokens.background,
                    theme.tokens.input,
                    theme.tokens.foreground,
                ),
                InputVariant::Outline => (
                    theme.tokens.background,
                    theme.tokens.border,
                    theme.tokens.foreground,
                ),
                InputVariant::Ghost => (
                    gpui::transparent_black(),
                    theme.tokens.border.opacity(0.3),
                    theme.tokens.foreground,
                ),
            }
        };

        let is_focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let user_style = self.style;

        wire_actions(&self.state, window, cx)
            .w_full()
            .h(height)
            .flex_shrink_0()
            .when(self.auto_grow, |this| this.min_h(height))
            .px(px(12.0))
            .py(px(8.0))
            .bg(bg_color)
            .border_1()
            .border_color(border_color)
            .rounded(theme.tokens.radius_md)
            .text_size(px(14.0))
            .font_family(theme.tokens.font_mono.clone())
            .text_color(text_color)
            .when(!self.disabled, |this| {
                this.hover(|style| {
                    style.border_color(if self.error {
                        theme.tokens.destructive
                    } else {
                        theme.tokens.ring
                    })
                })
            })
            .when(is_focused && !self.disabled, |this| {
                this.border_color(if self.error {
                    theme.tokens.destructive
                } else {
                    theme.tokens.ring
                })
            })
            .when(!self.disabled, |this| this.cursor(gpui::CursorStyle::IBeam))
            .map(|this| {
                let mut div = this;
                div.style().refine(&user_style);
                div
            })
            .child(self.state.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::foundation::theme::{install_theme, Theme};
    use std::{cell::Cell, rc::Rc};

    struct TextareaRenderTestView {
        state: Entity<TextareaState>,
        callback_calls: Option<Rc<Cell<usize>>>,
    }

    impl Render for TextareaRenderTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            Textarea::new(&self.state).when_some(self.callback_calls.clone(), |textarea, calls| {
                textarea.on_change(move |_, _| calls.set(calls.get() + 1))
            })
        }
    }

    #[gpui::test]
    fn repeated_render_replaces_textarea_event_subscription() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let stale_calls = Rc::new(Cell::new(0));
        let current_calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| TextareaRenderTestView {
            state: cx.new(TextareaState::new),
            callback_calls: Some(stale_calls.clone()),
        });

        window.draw();
        window.update(|view, _, cx| {
            view.callback_calls = Some(current_calls.clone());
            cx.notify();
        });
        window.draw();
        window.update(|view, _, cx| {
            view.state
                .update(cx, |_, cx| cx.emit(TextareaEvent::Change));
        });
        assert_eq!(stale_calls.get(), 0);
        assert_eq!(current_calls.get(), 1);

        window.update(|view, _, cx| {
            view.callback_calls = None;
            cx.notify();
        });
        window.draw();
        window.update(|view, _, cx| {
            view.state
                .update(cx, |_, cx| cx.emit(TextareaEvent::Change));
        });
        assert_eq!(current_calls.get(), 1);
    }

    struct BareTextareaTestView {
        state: Entity<TextareaState>,
    }

    impl Render for BareTextareaTestView {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            wire_actions(&self.state, window, cx)
                .h(px(96.0))
                .px(px(12.0))
                .rounded(px(8.0))
                .child(self.state.clone())
        }
    }

    #[gpui::test]
    fn wire_actions_gives_custom_textarea_containers_full_keyboard_editing() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx).value("hello")));
        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });

        window.draw();
        window.update(|_, window, cx| state.read(cx).focus_handle(cx).focus(window));
        window.draw();
        let handle: AnyWindowHandle = window.handle().into();
        app.raw_context_mut()
            .simulate_keystrokes(handle, "backspace");
        app.flush();

        app.update(|cx| {
            assert_eq!(state.read(cx).content(), "hell");
        });
    }
}
