//! Input component - Advanced text input with validation, masking, and accessibility.

pub use crate::capabilities::controls::input_state::{
    Backspace, Copy, Cut, Delete, End, Enter, Escape, Home, InputEvent, InputMask, InputState,
    InputType, Left, Paste, Right, SelectAll, SelectLeft, SelectRight, SelectWordLeft,
    SelectWordRight, ShiftTab, Tab, ValidationError, ValidationRules, WordLeft, WordRight,
};
use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::layout::{HStack, VStack};
use crate::capabilities::motion::animations::{easings, shake_offset};
use crate::capabilities::primitives::icon::Icon;
use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

type InputValidator = Rc<dyn Fn(&str) -> Result<(), String>>;
type InputTransformer = Rc<dyn Fn(&str) -> String>;
type InputValueCallback = Rc<dyn Fn(SharedString, &mut App)>;
type InputValidationCallback = Rc<dyn Fn(Result<(), ValidationError>, &mut App)>;

pub fn init(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "input") {
        return;
    }
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("Input")),
        KeyBinding::new("delete", Delete, Some("Input")),
        KeyBinding::new("left", Left, Some("Input")),
        KeyBinding::new("right", Right, Some("Input")),
        KeyBinding::new("shift-left", SelectLeft, Some("Input")),
        KeyBinding::new("shift-right", SelectRight, Some("Input")),
        KeyBinding::new("home", Home, Some("Input")),
        KeyBinding::new("end", End, Some("Input")),
        KeyBinding::new("enter", Enter, Some("Input")),
        KeyBinding::new("tab", Tab, Some("Input")),
        KeyBinding::new("shift-tab", ShiftTab, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some("Input")),
        KeyBinding::new("escape", Escape, Some("Input")),
        // Word navigation
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-left", WordLeft, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-right", WordRight, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", WordLeft, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", WordRight, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-left", SelectWordLeft, Some("Input")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-right", SelectWordRight, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, Some("Input")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-right", SelectWordRight, Some("Input")),
    ]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputVariant {
    Default,
    Outline,
    Ghost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputSize {
    Sm,
    #[default]
    Md,
    Lg,
}

#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    placeholder: Option<SharedString>,
    variant: InputVariant,
    size: InputSize,
    disabled: bool,
    error: bool,
    password: bool,
    clearable: bool,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    initial_value: Option<SharedString>,
    value: Option<SharedString>,

    // Enhanced features
    input_type: Option<InputType>,
    validation_rules: Option<ValidationRules>,
    helper_text: Option<SharedString>,
    show_character_count: bool,
    aria_label: Option<SharedString>,
    aria_description: Option<SharedString>,
    autocomplete: Option<SharedString>,
    required: bool,

    // Custom functions for extensibility
    custom_validator: Option<InputValidator>,
    custom_filter: Option<InputTransformer>,
    custom_formatter: Option<InputTransformer>,

    // Callbacks
    on_change: Option<InputValueCallback>,
    on_enter: Option<InputValueCallback>,
    on_focus: Option<InputValueCallback>,
    on_blur: Option<InputValueCallback>,
    on_validate: Option<InputValidationCallback>,

    // Style refinement for Styled trait
    style: StyleRefinement,
}

impl Input {
    /// Create a new input with an InputState entity
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            placeholder: None,
            variant: InputVariant::Default,
            size: InputSize::default(),
            disabled: false,
            error: false,
            password: false,
            clearable: false,
            prefix: None,
            suffix: None,
            initial_value: None,
            value: None,

            // Enhanced features
            input_type: None,
            validation_rules: None,
            helper_text: None,
            show_character_count: false,
            aria_label: None,
            aria_description: None,
            autocomplete: None,
            required: false,

            // Custom functions
            custom_validator: None,
            custom_filter: None,
            custom_formatter: None,

            // Callbacks
            on_change: None,
            on_enter: None,
            on_focus: None,
            on_blur: None,
            on_validate: None,

            // Style refinement
            style: StyleRefinement::default(),
        }
    }

    /// Set the value applied when this input state is first rendered.
    ///
    /// Later renders leave the state unchanged. To replace an existing value,
    /// update the associated [`InputState`] with [`InputState::set_value`].
    pub fn initial_value(mut self, value: impl Into<SharedString>) -> Self {
        self.initial_value = Some(value.into());
        self.value = None;
        self
    }

    /// Set a value controlled by the parent view.
    ///
    /// The input updates when the supplied value changes. Use [`Input::initial_value`]
    /// when the value should only be applied once.
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.value = Some(value.into());
        self.initial_value = None;
        self
    }

    /// Set placeholder text
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Enable the clear button when the input has a value.
    ///
    /// This spelling is kept as a compatibility alias for older callers.
    #[deprecated(note = "use clearable instead")]
    pub fn cleanable(self) -> Self {
        self.clearable(true)
    }

    /// Set the input variant
    pub fn variant(mut self, variant: InputVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the input size
    pub fn size(mut self, size: InputSize) -> Self {
        self.size = size;
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set error state (shows error styling)
    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    /// Enable password masking
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Enable the clear button when the input has a value.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Add a prefix element (icon, label, etc.)
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Add a suffix element (icon, label, etc.)
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set callback when value changes.
    ///
    /// The callback receives the canonical value before any custom display
    /// formatter is applied.
    pub fn on_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_change = Some(Rc::new(callback));
        self
    }

    /// Set callback when Enter key is pressed
    pub fn on_enter<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_enter = Some(Rc::new(callback));
        self
    }

    /// Set callback when input gains focus
    pub fn on_focus<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_focus = Some(Rc::new(callback));
        self
    }

    /// Set callback when input loses focus
    pub fn on_blur<F>(mut self, callback: F) -> Self
    where
        F: Fn(SharedString, &mut App) + 'static,
    {
        self.on_blur = Some(Rc::new(callback));
        self
    }

    /// Set the input type (email, number, tel, etc.)
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = Some(input_type);
        self
    }

    /// Set validation rules
    pub fn validation_rules(mut self, rules: ValidationRules) -> Self {
        self.validation_rules = Some(rules);
        self
    }

    /// Set minimum length requirement
    pub fn min_length(mut self, min: usize) -> Self {
        if self.validation_rules.is_none() {
            self.validation_rules = Some(ValidationRules::default());
        }
        if let Some(ref mut rules) = self.validation_rules {
            rules.min_length = Some(min);
        }
        self
    }

    /// Set maximum length requirement
    pub fn max_length(mut self, max: usize) -> Self {
        if self.validation_rules.is_none() {
            self.validation_rules = Some(ValidationRules::default());
        }
        if let Some(ref mut rules) = self.validation_rules {
            rules.max_length = Some(max);
        }
        self
    }

    /// Mark field as required
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        if self.validation_rules.is_none() {
            self.validation_rules = Some(ValidationRules::default());
        }
        if let Some(ref mut rules) = self.validation_rules {
            rules.required = required;
        }
        self
    }

    /// Set helper text displayed below the input
    pub fn helper_text(mut self, text: impl Into<SharedString>) -> Self {
        self.helper_text = Some(text.into());
        self
    }

    /// Show character count indicator
    pub fn show_character_count(mut self, show: bool) -> Self {
        self.show_character_count = show;
        self
    }

    /// Set ARIA label for accessibility
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Set ARIA description for accessibility
    pub fn aria_description(mut self, description: impl Into<SharedString>) -> Self {
        self.aria_description = Some(description.into());
        self
    }

    /// Set autocomplete attribute
    pub fn autocomplete(mut self, autocomplete: impl Into<SharedString>) -> Self {
        self.autocomplete = Some(autocomplete.into());
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

    /// Set a custom validation function
    ///
    /// # Example
    /// ```rust,ignore
    /// Input::new(&state)
    ///     .custom_validator(|value| {
    ///         if value.contains("@company.com") {
    ///             Ok(())
    ///         } else {
    ///             Err("Must be a company email".to_string())
    ///         }
    ///     })
    /// ```
    pub fn custom_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + 'static,
    {
        self.custom_validator = Some(Rc::new(validator));
        self
    }

    /// Set a custom filter function to control which characters are allowed
    ///
    /// # Example
    /// ```rust,ignore
    /// Input::new(&state)
    ///     .custom_filter(|input| {
    ///         // Only allow alphanumeric and underscores
    ///         input.chars()
    ///             .filter(|c| c.is_alphanumeric() || *c == '_')
    ///             .collect()
    ///     })
    /// ```
    pub fn custom_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&str) -> String + 'static,
    {
        self.custom_filter = Some(Rc::new(filter));
        self
    }

    /// Set a custom formatter function to format the input value
    ///
    /// # Example
    /// ```rust,ignore
    /// Input::new(&state)
    ///     .custom_formatter(|input| {
    ///         // Format as currency
    ///         format!("${:.2}", input.parse::<f64>().unwrap_or(0.0))
    ///     })
    /// ```
    pub fn custom_formatter<F>(mut self, formatter: F) -> Self
    where
        F: Fn(&str) -> String + 'static,
    {
        self.custom_formatter = Some(Rc::new(formatter));
        self
    }

    fn install_event_subscription(&self, cx: &mut App) {
        let on_change = self.on_change.clone();
        let on_enter = self.on_enter.clone();
        let on_focus = self.on_focus.clone();
        let on_blur = self.on_blur.clone();
        let on_validate = self.on_validate.clone();

        let has_callbacks = on_change.is_some()
            || on_enter.is_some()
            || on_focus.is_some()
            || on_blur.is_some()
            || on_validate.is_some();
        let subscription = has_callbacks.then(|| {
            cx.subscribe(&self.state, move |emitter, event: &InputEvent, cx| {
                // Custom formatters only change the rendered representation.
                // Propagate the canonical value so a controlled input can feed
                // the callback back into `value` without formatting it twice.
                let callback_value = || emitter.read(cx).raw_value().into();
                match event {
                    InputEvent::Change => {
                        if let Some(callback) = on_change.as_ref() {
                            callback(callback_value(), cx);
                        }
                    }
                    InputEvent::Enter => {
                        if let Some(callback) = on_enter.as_ref() {
                            callback(callback_value(), cx);
                        }
                    }
                    InputEvent::Focus => {
                        if let Some(callback) = on_focus.as_ref() {
                            callback(callback_value(), cx);
                        }
                    }
                    InputEvent::Blur => {
                        if let Some(callback) = on_blur.as_ref() {
                            callback(callback_value(), cx);
                        }
                    }
                    InputEvent::Validate(result) => {
                        if let Some(callback) = on_validate.as_ref() {
                            callback(result.clone(), cx);
                        }
                    }
                    InputEvent::Tab | InputEvent::ShiftTab => {}
                }
            })
        });

        self.state.update(cx, |state, _| {
            state.replace_event_subscription(subscription);
        });
    }

    /// Get height based on size
    fn height(&self) -> Pixels {
        match self.size {
            InputSize::Sm => px(32.0),
            InputSize::Md => px(40.0),
            InputSize::Lg => px(48.0),
        }
    }

    /// Get horizontal padding based on size
    fn padding_x(&self) -> Pixels {
        match self.size {
            InputSize::Sm => px(8.0),
            InputSize::Md => px(12.0),
            InputSize::Lg => px(16.0),
        }
    }

    /// Get font size based on size
    fn font_size(&self) -> Pixels {
        match self.size {
            InputSize::Sm => px(13.0),
            InputSize::Md => px(14.0),
            InputSize::Lg => px(16.0),
        }
    }

    /// Get gap between elements based on size
    fn element_gap(&self) -> Pixels {
        match self.size {
            InputSize::Sm => px(6.0),
            InputSize::Md => px(8.0),
            InputSize::Lg => px(10.0),
        }
    }

    /// Merge builder callbacks into the public ValidationRules representation.
    ///
    /// ValidationRules keeps Arc-based callback fields for API compatibility,
    /// while Input builders own UI-thread Rc callbacks. The conversion is
    /// intentionally kept in this one boundary rather than duplicated during
    /// every render branch.
    #[allow(clippy::arc_with_non_send_sync)]
    fn validation_rules_for_state(&self) -> Option<ValidationRules> {
        let has_builder_callbacks = self.custom_validator.is_some()
            || self.custom_filter.is_some()
            || self.custom_formatter.is_some();
        let mut rules = self
            .validation_rules
            .clone()
            .or_else(|| has_builder_callbacks.then(ValidationRules::default))?;

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

        if let Some(custom_filter) = self.custom_filter.as_ref() {
            let filter = custom_filter.clone();
            rules.custom_filter = Some(Arc::new(move |input| filter(input)));
        }

        if let Some(custom_formatter) = self.custom_formatter.as_ref() {
            let formatter = custom_formatter.clone();
            rules.custom_formatter = Some(Arc::new(move |input| formatter(input)));
        }

        Some(rules)
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let height = self.height();
        let padding_x = self.padding_x();
        let font_size = self.font_size();
        let gap = self.element_gap();

        self.state.update(cx, |state, cx| {
            state.disabled = self.disabled;
            if let Some(placeholder) = self.placeholder.clone() {
                if state.placeholder != placeholder {
                    state.placeholder = placeholder;
                    if state.content().is_empty() {
                        state.invalidate_layout_cache();
                    }
                }
            }

            // Enable masking when this builder first transitions the state to a password input.
            // Later renders preserve user-driven visibility toggles.
            if self.password && state.input_type != InputType::Password {
                state.input_type = InputType::Password;
                state.masked = true;
            }

            if let Some(input_type) = self.input_type {
                let input_type_changed = state.input_type != input_type;
                state.input_type = input_type;
                if input_type_changed && input_type == InputType::Password {
                    state.masked = true;
                }
                match input_type {
                    InputType::Tel => state.input_mask = InputMask::Phone,
                    InputType::CreditCard => state.input_mask = InputMask::CreditCard,
                    InputType::Date => state.input_mask = InputMask::Date,
                    InputType::Time => state.input_mask = InputMask::Time,
                    _ => {}
                }
            }

            let builder_formatter_active = self.custom_formatter.is_some()
                || self
                    .validation_rules
                    .as_ref()
                    .is_some_and(|rules| rules.custom_formatter.is_some());

            if let Some(rules) = self.validation_rules_for_state() {
                state.validation_rules = rules;
            }
            state.sync_builder_formatter(builder_formatter_active);

            // Keep the raw value when formatter configuration changes, then
            // refresh the display representation before applying controlled
            // value synchronization.
            state.refresh_formatted_content();

            state.aria_label = self.aria_label.clone();
            state.aria_description = self.aria_description.clone();
            state.autocomplete = self.autocomplete.clone();
            state.helper_text = self.helper_text.clone();

            if let Some(value) = self.value.clone() {
                if !state.value_matches(value.as_ref()) {
                    state.set_value(value, window, cx);
                }
            } else if let Some(value) = self.initial_value.clone() {
                state.apply_initial_value(value, window, cx);
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

        let has_value = !self.state.read(cx).content.is_empty();
        let show_clear = self.clearable && has_value && !self.disabled;
        let state_for_clear = self.state.clone();
        let state_for_password = self.state.clone();

        let input_state = self.state.read(cx);
        let validation_error = input_state.validation_error.clone();
        let success_message = input_state.success_message.clone();
        let content_length = input_state.content.chars().count();
        let max_length = input_state.validation_rules.max_length;
        let is_focused = input_state.focus_handle(cx).is_focused(window);
        let is_masked = input_state.masked;
        let aria_role = if input_state.input_type == InputType::Password {
            Role::PasswordInput
        } else {
            Role::TextInput
        };
        let aria_label = input_state.aria_label.clone();
        let aria_description = input_state.aria_description.clone();
        let aria_placeholder = input_state.placeholder.clone();
        let aria_value: SharedString = if is_masked {
            "•".repeat(input_state.content.chars().count()).into()
        } else {
            input_state.content.clone()
        };
        let aria_required = input_state.validation_rules.required;
        let aria_invalid = self.error || validation_error.is_some();
        let shake_triggered = input_state.shake_triggered;

        if shake_triggered {
            self.state.update(cx, |state, _cx| {
                state.shake_triggered = false;
                state.shake_count = state.shake_count.wrapping_add(1);
            });
        }
        let shake_count = self.state.read(cx).shake_count;

        let shadow_xs = BoxShadow {
            offset: theme.tokens.shadow_xs.offset,
            blur_radius: theme.tokens.shadow_xs.blur_radius,
            spread_radius: theme.tokens.shadow_xs.spread_radius,
            inset: false,
            color: theme.tokens.shadow_xs.color,
        };
        let focus_ring = theme.tokens.focus_ring_light();
        let error_ring_focused = theme.tokens.error_ring();
        let error_ring_unfocused = theme.tokens.error_ring();
        let ring_color = theme.tokens.ring;
        let destructive_color = theme.tokens.destructive;

        let user_style = self.style;

        VStack::new()
            .w_full()
            .gap(px(4.0))
            .child({
                let input_container = div()
                    .id(("input", self.state.entity_id()))
                    .role(aria_role)
                    .when_some(aria_label, |this, label| this.aria_label(label))
                    .when_some(aria_description, |this, description| {
                        this.aria_description(description)
                    })
                    .aria_value(aria_value)
                    .aria_placeholder(aria_placeholder)
                    .aria_disabled(self.disabled)
                    .aria_required(aria_required)
                    .when(aria_invalid, |this| {
                        this.aria_invalid(gpui::accesskit::Invalid::True)
                    })
                    .key_context("Input")
                    .track_focus(
                        &self
                            .state
                            .read(cx)
                            .focus_handle(cx)
                            .tab_index(0)
                            .tab_stop(true),
                    )
                    .when(!self.disabled, |this| {
                        this.on_action(window.listener_for(&self.state, InputState::backspace))
                            .on_action(window.listener_for(&self.state, InputState::delete))
                            .on_action(window.listener_for(&self.state, InputState::left))
                            .on_action(window.listener_for(&self.state, InputState::right))
                            .on_action(window.listener_for(&self.state, InputState::select_left))
                            .on_action(window.listener_for(&self.state, InputState::select_right))
                            .on_action(window.listener_for(&self.state, InputState::select_all))
                            .on_action(window.listener_for(&self.state, InputState::home))
                            .on_action(window.listener_for(&self.state, InputState::end))
                            .on_action(window.listener_for(&self.state, InputState::copy))
                            .on_action(window.listener_for(&self.state, InputState::cut))
                            .on_action(window.listener_for(&self.state, InputState::paste))
                            .on_action(window.listener_for(&self.state, InputState::enter))
                            .on_action(window.listener_for(&self.state, InputState::tab))
                            .on_action(window.listener_for(&self.state, InputState::shift_tab))
                            .on_action(window.listener_for(&self.state, InputState::escape))
                            .on_action(window.listener_for(&self.state, InputState::word_left))
                            .on_action(window.listener_for(&self.state, InputState::word_right))
                            .on_action(
                                window.listener_for(&self.state, InputState::select_word_left),
                            )
                            .on_action(
                                window.listener_for(&self.state, InputState::select_word_right),
                            )
                    })
                    .child(
                        HStack::new()
                            .h(height)
                            .w_full()
                            .px(padding_x)
                            .gap(gap)
                            .bg(bg_color)
                            .border_1()
                            .border_color(border_color)
                            .rounded(theme.tokens.radius_md)
                            .items_center()
                            .text_size(font_size)
                            .font_family(theme.tokens.font_mono.clone())
                            .text_color(text_color)
                            .shadow(smallvec::smallvec![shadow_xs])
                            .when(!self.disabled, |h| h.cursor(gpui::CursorStyle::IBeam))
                            .when(!self.disabled, |h| {
                                h.hover(move |style| {
                                    style.border_color(if self.error {
                                        destructive_color
                                    } else {
                                        ring_color
                                    })
                                })
                            })
                            .when(is_focused && !self.disabled, |h| {
                                if self.error {
                                    h.border_color(destructive_color)
                                        .shadow(smallvec::smallvec![error_ring_focused])
                                } else {
                                    h.border_color(ring_color)
                                        .shadow(smallvec::smallvec![focus_ring])
                                }
                            })
                            .when(self.error && !is_focused, |h| {
                                h.shadow(smallvec::smallvec![error_ring_unfocused])
                            })
                            .children(self.prefix)
                            .child(div().flex_1().overflow_hidden().child(self.state.clone()))
                            .when(show_clear, |h| {
                                h.child(
                                    div()
                                        .px(px(4.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.tokens.muted))
                                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                            state_for_clear.update(cx, |state, cx| {
                                                state.set_value("", window, cx);
                                            })
                                        })
                                        .child("×")
                                        .text_color(theme.tokens.muted_foreground),
                                )
                            })
                            .when(self.password, |h| {
                                h.child(
                                    div()
                                        .px(px(4.0))
                                        .py(px(4.0))
                                        .rounded(px(4.0))
                                        .cursor_pointer()
                                        .hover(|style| style.bg(theme.tokens.muted))
                                        .on_mouse_down(MouseButton::Left, {
                                            let state = state_for_password.clone();
                                            move |_, window, cx| {
                                                state.update(cx, |state, cx| {
                                                    state.set_masked(!state.masked, cx);
                                                });
                                                window.refresh();
                                            }
                                        })
                                        .child(
                                            Icon::new(if is_masked { "eye" } else { "eye-off" })
                                                .size(px(16.0))
                                                .color(theme.tokens.muted_foreground),
                                        ),
                                )
                            })
                            .children(self.suffix),
                    );

                if shake_count > 0 {
                    input_container
                        .with_animation(
                            ElementId::Name(format!("input-shake-{}", shake_count).into()),
                            Animation::new(Duration::from_millis(300))
                                .with_easing(easings::ease_out_quad),
                            move |el, delta| el.ml(px(shake_offset(delta, 6.0))),
                        )
                        .into_any_element()
                } else {
                    input_container.into_any_element()
                }
            })
            .when(
                self.helper_text.is_some()
                    || validation_error.is_some()
                    || success_message.is_some()
                    || self.show_character_count,
                |v| {
                    v.child(
                        HStack::new()
                            .w_full()
                            .px(px(2.0))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.0))
                                    .font_family(theme.tokens.font_family.clone())
                                    .child(if let Some(error) = validation_error {
                                        div()
                                            .text_color(theme.tokens.destructive)
                                            .child(error.message)
                                    } else if let Some(success) = success_message {
                                        if has_value {
                                            div().text_color(theme.tokens.primary).child(success)
                                        } else {
                                            div()
                                        }
                                    } else if let Some(helper) = self.helper_text {
                                        div()
                                            .text_color(theme.tokens.muted_foreground)
                                            .child(helper)
                                    } else {
                                        div()
                                    }),
                            )
                            .when(self.show_character_count, |h| {
                                h.child(
                                    div()
                                        .text_size(px(12.0))
                                        .font_family(theme.tokens.font_family.clone())
                                        .text_color(
                                            if max_length.is_some()
                                                && content_length >= max_length.unwrap()
                                            {
                                                theme.tokens.destructive
                                            } else {
                                                theme.tokens.muted_foreground
                                            },
                                        )
                                        .child(if let Some(max) = max_length {
                                            format!("{}/{}", content_length, max)
                                        } else {
                                            format!("{}", content_length)
                                        }),
                                )
                            }),
                    )
                },
            )
            .map(|this| {
                let mut vstack = this;
                vstack.style().refine(&user_style);
                vstack
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::foundation::theme::{install_theme, Theme};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    struct InputRenderTestView {
        state: Entity<InputState>,
        callback_calls: Option<Rc<Cell<usize>>>,
        initial_value: Option<SharedString>,
        value: Option<SharedString>,
    }

    struct FormattedInputRenderTestView {
        state: Entity<InputState>,
        value: SharedString,
        callback_calls: Rc<Cell<usize>>,
        suffix: bool,
    }

    struct FormattedCallbackTestView {
        state: Entity<InputState>,
        values: Rc<RefCell<Vec<SharedString>>>,
    }

    struct ToggleFormatterRenderTestView {
        state: Entity<InputState>,
        formatter_enabled: bool,
    }

    impl Render for FormattedCallbackTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let values = self.values.clone();
            Input::new(&self.state)
                .custom_formatter(|input| format!("${:.2}", input.parse::<f64>().unwrap_or(0.0)))
                .on_change(move |value, _| values.borrow_mut().push(value))
        }
    }

    impl Render for FormattedInputRenderTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let callback_calls = self.callback_calls.clone();
            let suffix = self.suffix;
            Input::new(&self.state)
                .value(self.value.clone())
                .custom_formatter(move |input| {
                    if suffix {
                        format!("{input} USD")
                    } else {
                        format!("${:.2}", input.parse::<f64>().unwrap_or(0.0))
                    }
                })
                .on_change(move |_, _| callback_calls.set(callback_calls.get() + 1))
        }
    }

    impl Render for ToggleFormatterRenderTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let input = Input::new(&self.state).value("12");
            if self.formatter_enabled {
                input.custom_formatter(|value| {
                    format!("${:.2}", value.parse::<f64>().unwrap_or(0.0))
                })
            } else {
                input
            }
        }
    }

    impl Render for InputRenderTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            Input::new(&self.state)
                .when_some(self.initial_value.clone(), Input::initial_value)
                .when_some(self.value.clone(), Input::value)
                .when_some(self.callback_calls.clone(), |input, calls| {
                    input.on_change(move |_, _| calls.set(calls.get() + 1))
                })
        }
    }

    #[gpui::test]
    fn controlled_value_changes_replace_existing_content() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: None,
            initial_value: None,
            value: Some("first".into()),
        });

        window.draw();
        window.update(|view, _, cx| {
            view.value = Some("replacement".into());
            cx.notify();
        });
        window.draw();

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "replacement"
        );
    }

    #[gpui::test]
    fn unchanged_controlled_value_does_not_emit_change_on_rerender() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(|cx| {
                let mut state = InputState::new(cx);
                state.content = "first".into();
                state
            }),
            callback_calls: Some(calls.clone()),
            initial_value: None,
            value: Some("first".into()),
        });

        window.draw();
        window.update(|_, _, cx| cx.notify());
        window.draw();

        assert_eq!(calls.get(), 0);
    }

    #[gpui::test]
    fn formatted_controlled_value_does_not_emit_on_unchanged_rerender() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| FormattedInputRenderTestView {
            state: cx.new(InputState::new),
            value: "12".into(),
            callback_calls: calls.clone(),
            suffix: false,
        });

        window.draw();
        let calls_after_initial_render = calls.get();
        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "$12.00"
        );

        window.update(|_, _, cx| cx.notify());
        window.draw();

        assert_eq!(calls.get(), calls_after_initial_render);
    }

    #[gpui::test]
    fn changing_formatter_reformats_the_controlled_value() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| FormattedInputRenderTestView {
            state: cx.new(InputState::new),
            value: "12".into(),
            callback_calls: calls.clone(),
            suffix: false,
        });

        window.draw();
        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "$12.00"
        );

        window.update(|view, _, cx| {
            view.suffix = true;
            cx.notify();
        });
        window.draw();

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "12 USD"
        );
    }

    #[gpui::test]
    fn removing_builder_formatter_restores_raw_display_value() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| ToggleFormatterRenderTestView {
            state: cx.new(InputState::new),
            formatter_enabled: true,
        });

        window.draw();
        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "$12.00"
        );

        window.update(|view, _, cx| {
            view.formatter_enabled = false;
            cx.notify();
        });
        window.draw();

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "12"
        );
    }

    #[gpui::test]
    fn formatted_input_change_callback_reports_raw_value() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let values = Rc::new(RefCell::new(Vec::new()));
        let values_for_view = values.clone();
        let mut window = app.open_window(|_, _| FormattedCallbackTestView {
            state: state.clone(),
            values: values_for_view,
        });

        window.draw();
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("123", window, cx);
            });
        });

        assert_eq!(
            values.borrow().last().map(SharedString::as_ref),
            Some("123")
        );
    }

    #[gpui::test]
    fn clearable_builder_enables_clear_button() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));

        let input = Input::new(&state).clearable(true);

        assert!(input.clearable);
    }

    #[gpui::test]
    fn repeated_render_replaces_input_event_subscription() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let stale_calls = Rc::new(Cell::new(0));
        let current_calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: Some(stale_calls.clone()),
            initial_value: None,
            value: None,
        });

        window.draw();
        window.update(|view, _, cx| {
            view.callback_calls = Some(current_calls.clone());
            cx.notify();
        });
        window.draw();
        window.update(|view, _, cx| {
            view.state.update(cx, |_, cx| cx.emit(InputEvent::Change));
        });
        assert_eq!(stale_calls.get(), 0);
        assert_eq!(current_calls.get(), 1);

        window.update(|view, _, cx| {
            view.callback_calls = None;
            cx.notify();
        });
        window.draw();
        window.update(|view, _, cx| {
            view.state.update(cx, |_, cx| cx.emit(InputEvent::Change));
        });
        assert_eq!(current_calls.get(), 1);
    }

    #[gpui::test]
    fn initial_value_does_not_overwrite_user_edits_on_rerender() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: None,
            initial_value: Some("initial".into()),
            value: None,
        });

        window.draw();
        window.update(|view, _, cx| {
            view.state.update(cx, |state, cx| {
                state.content = "edited".into();
                cx.notify();
            });
            cx.notify();
        });
        window.draw();

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "edited"
        );
    }

    #[gpui::test]
    fn builder_initial_value_changes_are_ignored_after_first_render() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: None,
            initial_value: Some("initial".into()),
            value: None,
        });

        window.draw();
        window.update(|view, _, cx| {
            view.initial_value = Some("replacement".into());
            cx.notify();
        });
        window.draw();

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "initial"
        );
    }

    #[gpui::test]
    fn set_value_emits_one_change_event() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let calls = Rc::new(Cell::new(0));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: Some(calls.clone()),
            initial_value: None,
            value: None,
        });

        window.draw();
        window.update(|view, window, cx| {
            view.state
                .update(cx, |state, cx| state.set_value("edited", window, cx));
        });

        assert_eq!(calls.get(), 1);
    }

    #[gpui::test]
    fn set_value_replaces_existing_content() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, cx| InputRenderTestView {
            state: cx.new(InputState::new),
            callback_calls: None,
            initial_value: Some("initial".into()),
            value: None,
        });

        window.draw();
        window.update(|view, window, cx| {
            view.state
                .update(cx, |state, cx| state.set_value("edited", window, cx));
        });

        assert_eq!(
            window.read(|view, cx| view.state.read(cx).content().to_string()),
            "edited"
        );
    }
}
