//! Textarea component - Multi-line text input component with full editing capabilities.

use crate::capabilities::controls::input::InputVariant;
pub use crate::capabilities::controls::textarea_state::wire_actions;
use crate::capabilities::controls::textarea_state::TextareaEvent;
use crate::capabilities::controls::textarea_state::TextareaState;
use crate::capabilities::controls::textarea_state::{ValidationError, ValidationRules};
pub use crate::capabilities::controls::validation::TextValidationRules;
use crate::capabilities::controls::validation::{validation_footer, CharacterCount, FooterMessage};
use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::layout::VStack;
use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;
use std::sync::Arc;

type TextareaValidator = Rc<dyn Fn(&str) -> Result<(), String>>;
type TextareaValidationCallback = Rc<dyn Fn(Result<(), ValidationError>, &mut App)>;

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
    placeholder: Option<SharedString>,
    variant: InputVariant,
    disabled: Option<bool>,
    error: bool,
    rows: usize,
    min_rows: Option<usize>,
    max_rows: Option<usize>,
    auto_grow: bool,
    initial_value: Option<SharedString>,

    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,

    on_change: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_enter: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_shift_enter: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_focus: Option<Rc<dyn Fn(SharedString, &mut App)>>,
    on_blur: Option<Rc<dyn Fn(SharedString, &mut App)>>,

    validation_rules: Option<TextValidationRules>,
    custom_validator: Option<TextareaValidator>,
    helper_text: Option<SharedString>,
    show_character_count: bool,
    on_validate: Option<TextareaValidationCallback>,

    style: StyleRefinement,
}

impl Textarea {
    /// Builder rules, placeholder, and disabled settings temporarily override
    /// state configuration. Removing a setting restores its pre-override
    /// baseline; direct state writes remain live when that setting is unowned.
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
            placeholder: None,
            variant: InputVariant::Default,
            disabled: None,
            error: false,
            rows: 3,
            min_rows: None,
            max_rows: None,
            auto_grow: false,
            initial_value: None,
            aria_label: None,
            aria_description: None,
            on_change: None,
            on_enter: None,
            on_shift_enter: None,
            on_focus: None,
            on_blur: None,
            validation_rules: None,
            custom_validator: None,
            helper_text: None,
            show_character_count: false,
            on_validate: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.initial_value = Some(value.into());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn variant(mut self, variant: InputVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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

    /// Set the accessible label for the textarea.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set the accessible description for the textarea.
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Set legacy validation rules. Only required, length, pattern, and custom
    /// validation constraints apply; numeric bounds, filters, and formatters
    /// are ignored. Prefer [`Self::text_validation_rules`] for new code.
    pub fn validation_rules(mut self, rules: ValidationRules) -> Self {
        self.validation_rules = Some(rules.into());
        self
    }

    /// Set the constraints supported by multiline text controls.
    pub fn text_validation_rules(mut self, rules: TextValidationRules) -> Self {
        self.validation_rules = Some(rules);
        self
    }

    /// Set minimum length requirement
    pub fn min_length(mut self, min: usize) -> Self {
        self.rules_mut().min_length = Some(min);
        self
    }

    /// Set maximum length requirement (also truncates user input)
    pub fn max_length(mut self, max: usize) -> Self {
        self.rules_mut().max_length = Some(max);
        self
    }

    /// Mark field as required
    pub fn required(mut self, required: bool) -> Self {
        self.rules_mut().required = required;
        self
    }

    /// Set helper text displayed below the textarea
    pub fn helper_text(mut self, text: impl Into<SharedString>) -> Self {
        self.helper_text = Some(text.into());
        self
    }

    /// Show character count indicator
    pub fn show_character_count(mut self, show: bool) -> Self {
        self.show_character_count = show;
        self
    }

    /// Set a custom validation function
    pub fn custom_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        self.custom_validator = Some(Rc::new(validator));
        self
    }

    /// Set callback for validation events
    pub fn on_validate<F>(mut self, callback: F) -> Self
    where
        F: Fn(Result<(), ValidationError>, &mut App) + 'static,
    {
        self.on_validate = Some(Rc::new(callback));
        self
    }

    fn rules_mut(&mut self) -> &mut TextValidationRules {
        self.validation_rules
            .get_or_insert_with(TextValidationRules::default)
    }

    #[allow(clippy::arc_with_non_send_sync)]
    fn validation_rules_for_state(&self) -> Option<TextValidationRules> {
        let mut rules = self.validation_rules.clone().or_else(|| {
            self.custom_validator
                .is_some()
                .then(TextValidationRules::default)
        })?;

        if let Some(custom_validator) = self.custom_validator.as_ref() {
            let validator = custom_validator.clone();
            rules.custom_validator = Some(Arc::new(move |value| match validator(value) {
                Ok(()) => Ok(()),
                Err(message) => Err(ValidationError {
                    message: message.into(),
                    field_name: None,
                }),
            }));
        }

        Some(rules)
    }

    fn install_event_subscription(&self, cx: &mut App) {
        let on_change = self.on_change.clone();
        let on_enter = self.on_enter.clone();
        let on_shift_enter = self.on_shift_enter.clone();
        let on_focus = self.on_focus.clone();
        let on_blur = self.on_blur.clone();
        let on_validate = self.on_validate.clone();

        let has_callbacks = on_change.is_some()
            || on_enter.is_some()
            || on_shift_enter.is_some()
            || on_focus.is_some()
            || on_blur.is_some()
            || on_validate.is_some();
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
                    TextareaEvent::Validate(result) => {
                        if let Some(callback) = on_validate.as_ref() {
                            callback(result.clone(), cx);
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
            // Apply rules before the initial value so `set_value` truncates
            // against `max_length` on first render too.
            state.sync_builder_config(
                self.placeholder.clone(),
                self.disabled,
                self.validation_rules_for_state(),
            );

            if !state.initialized {
                state.initialized = true;
                if let Some(value) = self.initial_value.clone() {
                    state.set_value(value, window, cx);
                }
            }
        });

        self.install_event_subscription(cx);
        let disabled = self.state.read(cx).disabled;
        let placeholder = self.state.read(cx).placeholder.clone();

        let (validation_error, success_message, content_length, max_length, has_value, composing) = {
            let state = self.state.read(cx);
            let composing = state.composition_char_count();
            (
                state.validation_error.clone(),
                state.success_message.clone(),
                // Exclude the in-flight IME composition from the committed
                // count; the footer shows it separately.
                state.content.chars().count() - composing,
                state.validation_rules.max_length,
                !state.content.is_empty(),
                composing,
            )
        };

        // A failed validation (e.g. on blur) shows the destructive border just
        // like an explicit `.error(true)`.
        let has_error = self.error || validation_error.is_some();

        let (bg_color, border_color, text_color) =
            crate::capabilities::controls::input_shell_colors(
                self.variant,
                disabled,
                has_error,
                &theme.tokens,
            );

        let is_focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let user_style = self.style;

        let has_footer = self.helper_text.is_some()
            || validation_error.is_some()
            || success_message.is_some()
            || self.show_character_count;
        let footer_message = if let Some(error) = validation_error.as_ref() {
            FooterMessage::Error(error)
        } else if let Some(success) = success_message.as_ref() {
            if has_value {
                FooterMessage::Success(success)
            } else {
                FooterMessage::Blank
            }
        } else if let Some(helper) = self.helper_text.as_ref() {
            FooterMessage::Helper(helper)
        } else {
            FooterMessage::Blank
        };
        let character_count = self.show_character_count.then_some(CharacterCount {
            committed: content_length,
            composing,
            max: max_length,
        });

        let field = wire_actions(&self.state, window, cx)
            .debug_selector(|| "textarea-field".into())
            .role(Role::MultilineTextInput)
            .when_some(self.aria_label.clone(), |this, label| {
                this.aria_label(label)
            })
            .when_some(self.aria_description.clone(), |this, description| {
                this.aria_description(description)
            })
            .aria_value(self.state.read(cx).content.clone())
            .aria_placeholder(placeholder)
            .aria_disabled(disabled)
            .aria_required(self.state.read(cx).validation_rules.required)
            .when(has_error, |this| {
                this.aria_invalid(gpui::accesskit::Invalid::True)
            })
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
            .when(!disabled, |this| {
                this.hover(|style| {
                    style.border_color(if has_error {
                        theme.tokens.destructive
                    } else {
                        theme.tokens.ring
                    })
                })
            })
            .when(is_focused && !disabled, |this| {
                this.border_color(if has_error {
                    theme.tokens.destructive
                } else {
                    theme.tokens.ring
                })
            })
            .when(!disabled, |this| this.cursor(gpui::CursorStyle::IBeam))
            .child(self.state.clone());

        if has_footer {
            VStack::new()
                .debug_selector(|| "textarea-root".into())
                .w_full()
                .gap(px(4.0))
                .child(field)
                .child(validation_footer(
                    &theme.tokens,
                    footer_message,
                    character_count,
                ))
                .map(|this| {
                    let mut vstack = this;
                    vstack.style().refine(&user_style);
                    vstack
                })
                .into_any_element()
        } else {
            field
                .map(|this| {
                    let mut div = this;
                    div.style().refine(&user_style);
                    div
                })
                .into_any_element()
        }
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
    struct FooterTextareaTestView {
        state: Entity<TextareaState>,
        validate_errors: Rc<Cell<usize>>,
    }

    impl Render for FooterTextareaTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let validate_errors = self.validate_errors.clone();
            Textarea::new(&self.state)
                .required(true)
                .max_length(3)
                .helper_text("hint")
                .show_character_count(true)
                .on_validate(move |result, _| {
                    if result.is_err() {
                        validate_errors.set(validate_errors.get() + 1);
                    }
                })
        }
    }

    #[gpui::test]
    fn footer_renders_and_blur_validation_reaches_on_validate() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let validate_errors = Rc::new(Cell::new(0));
        let state = app.update(|cx| cx.new(TextareaState::new));
        let mut window = app.open_window(|_, _| FooterTextareaTestView {
            state: state.clone(),
            validate_errors: validate_errors.clone(),
        });
        window.draw();
        window.draw();
        window.update(|_, window, cx| state.read(cx).focus_handle(cx).focus(window));
        window.draw();
        // Moving focus elsewhere drives the blur -> validate_on_blur path.
        window.update(|_, window, cx| cx.focus_handle().focus(window));
        window.draw();

        assert_eq!(validate_errors.get(), 1);
        app.update(|cx| assert!(state.read(cx).validation_error.is_some()));
    }

    #[gpui::test]
    fn max_length_truncates_typed_input() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx).max_length(3)));
        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });
        window.draw();
        window.update(|_, window, cx| state.read(cx).focus_handle(cx).focus(window));
        window.draw();
        let handle: AnyWindowHandle = window.handle().into();
        app.raw_context_mut().simulate_keystrokes(handle, "abcde");
        app.flush();

        app.update(|cx| assert_eq!(state.read(cx).content(), "abc"));
    }

    #[gpui::test]
    fn set_value_truncates_to_max_length() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx).max_length(3)));
        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });
        window.draw();
        window.update(|_, window, cx| {
            state.update(cx, |state, cx| state.set_value("abcdef", window, cx));
        });

        app.update(|cx| assert_eq!(state.read(cx).content(), "abc"));
    }

    #[gpui::test]
    fn rejected_max_length_input_does_not_emit_change() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx).max_length(3).value("abc")));
        let changes = Rc::new(Cell::new(0));
        let changes_for_subscription = changes.clone();
        app.update(|cx| {
            cx.subscribe(&state, move |_, event: &TextareaEvent, _| {
                if matches!(event, TextareaEvent::Change) {
                    changes_for_subscription.set(changes_for_subscription.get() + 1);
                }
            })
            .detach();
        });

        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });
        window.draw();
        window.update(|_, window, cx| {
            state.update(cx, |state, cx| {
                state.replace_text_in_range(None, "d", window, cx)
            });
        });

        app.update(|cx| {
            assert_eq!(state.read(cx).content(), "abc");
            assert_eq!(changes.get(), 0);
        });
    }

    #[gpui::test]
    fn validate_on_change_runs_after_set_value() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        let state = app.update(|cx| {
            cx.new(|cx| {
                let mut state = TextareaState::new(cx).required(true).value("x");
                state.validate_on_change = true;
                state
            })
        });
        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });
        window.draw();
        window.update(|_, window, cx| {
            state.update(cx, |state, cx| state.set_value("", window, cx));
        });

        app.update(|cx| {
            assert_eq!(
                state
                    .read(cx)
                    .validation_error
                    .as_ref()
                    .unwrap()
                    .message
                    .as_ref(),
                "This field is required"
            );
        });
    }

    #[gpui::test]
    fn auto_validation_stays_silent_without_rules() {
        let mut app = TestApp::new();
        app.update(|cx| {
            install_theme(cx, Theme::light());
            crate::capabilities::controls::textarea_state::init(cx);
        });
        // No rules configured: blur and change must not produce validation
        // messages, so existing plain textareas render exactly as before.
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx).value("hello")));
        let mut window = app.open_window(|_, _| BareTextareaTestView {
            state: state.clone(),
        });
        window.draw();
        window.update(|_, window, cx| state.read(cx).focus_handle(cx).focus(window));
        window.draw();
        window.update(|_, window, cx| cx.focus_handle().focus(window));
        window.draw();
        window.update(|_, window, cx| {
            state.update(cx, |state, cx| state.set_value("changed", window, cx));
        });

        app.update(|cx| {
            let state = state.read(cx);
            assert!(state.validation_error.is_none());
            assert!(state.success_message.is_none());
        });
    }
    #[gpui::test]
    fn removing_builder_rules_restores_textarea_state_rules() {
        struct RulesView {
            state: Entity<TextareaState>,
            override_rules: bool,
        }
        impl Render for RulesView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Textarea::new(&self.state).when(self.override_rules, |area| {
                    area.max_length(1).required(true)
                })
            }
        }
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| RulesView {
            state: cx.new(|cx| TextareaState::new(cx).min_length(3)),
            override_rules: true,
        });
        window.draw();
        window
            .read(|view, cx| assert_eq!(view.state.read(cx).validation_rules.max_length, Some(1)));
        window.update(|view, _, cx| {
            assert!(view
                .state
                .update(cx, |state, cx| state.validate(cx))
                .is_err());
        });
        window.update(|view, _, cx| {
            view.override_rules = false;
            cx.notify();
        });
        window.draw();
        window.read(|view, cx| {
            let rules = &view.state.read(cx).validation_rules;
            assert_eq!(rules.max_length, None);
            assert_eq!(rules.min_length, Some(3));
            assert!(view.state.read(cx).validation_error.is_none());
        });
    }

    #[gpui::test]
    fn rebuilt_builder_validator_keeps_textarea_feedback_across_redraws() {
        struct ValidatorView {
            state: Entity<TextareaState>,
        }
        impl Render for ValidatorView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Textarea::new(&self.state).custom_validator(|value| {
                    if value.len() < 3 {
                        Err("too short".to_string())
                    } else {
                        Ok(())
                    }
                })
            }
        }
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| ValidatorView {
            state: cx.new(TextareaState::new),
        });
        window.draw();
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("ab", window, cx);
                assert!(state.validate(cx).is_err());
                state.set_value("abcd", window, cx);
            });
        });
        for _ in 0..2 {
            window.update(|_, _, cx| cx.notify());
            window.draw();
        }
        window.read(|view, cx| {
            assert!(
                view.state.read(cx).validation_error.is_some(),
                "a rebuilt closure is the same policy; rendering must not revalidate"
            );
        });
    }

    #[gpui::test]
    fn typed_text_rules_drive_builder_validation_and_initial_value_truncation() {
        struct TypedRulesView {
            state: Entity<TextareaState>,
        }
        impl Render for TypedRulesView {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                Textarea::new(&self.state)
                    .text_validation_rules(TextValidationRules {
                        min_length: Some(2),
                        max_length: Some(3),
                        required: true,
                        pattern: Some("^[a-z]+$".into()),
                        ..Default::default()
                    })
                    .value("abcd")
            }
        }
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| TypedRulesView {
            state: cx.new(TextareaState::new),
        });
        window.draw();
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                assert_eq!(state.content(), "abc");
                assert!(state.validate(cx).is_ok());
                state.set_value("12", window, cx);
                assert!(state.validate(cx).is_err());
            });
        });
    }

    #[gpui::test]
    fn textarea_legacy_rules_ignore_numeric_bounds_and_transformations() {
        let mut app = TestApp::new();
        let state = app.update(|cx| {
            cx.new(|cx| {
                TextareaState::new(cx).validation_rules(ValidationRules {
                    min_value: Some(100.0),
                    max_value: Some(200.0),
                    custom_filter: Some(Arc::new(|_| panic!("textarea must not filter text"))),
                    custom_formatter: Some(Arc::new(|_| panic!("textarea must not format text"))),
                    ..Default::default()
                })
            })
        });
        let typed_state = app.update(|cx| {
            cx.new(|cx| {
                TextareaState::new(cx).text_validation_rules(TextValidationRules {
                    max_length: Some(2),
                    ..Default::default()
                })
            })
        });
        let mut window = app.open_window(|_, _| gpui::Empty);
        window.update(|_, window, cx| {
            state.update(cx, |state, cx| {
                state.set_value("words", window, cx);
                assert_eq!(state.content(), "words");
                assert!(state.validate(cx).is_ok());
            });
            typed_state.update(cx, |state, cx| {
                state.set_value("你好呀", window, cx);
                assert_eq!(state.content(), "你好");
            });
        });
    }
}
