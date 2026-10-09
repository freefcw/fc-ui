use crate::capabilities::foundation::theme::use_theme;
/// Interactive text input state management with validation and accessibility
///
/// Industry-standard input component with:
/// - Multiple input types with built-in validation
/// - Character filtering and input masking
/// - Proper cursor management and accessibility
/// - Real-time validation with error messages
/// - Min/max length enforcement
/// - Pattern matching support
/// - Auto-formatting for phone, date, credit card
/// - ARIA support for screen readers
use gpui::{prelude::*, *};
use std::ops::Range;
use std::sync::LazyLock;

use crate::capabilities::controls::input_format::DisplayMapping;
use crate::capabilities::controls::text_util;
use crate::capabilities::controls::validation;

// The canonical definitions live in `controls::validation` so that
// `TextareaState` shares the exact same machinery; they are re-exported here
// to preserve the long-standing `input_state::ValidationRules` public path.
pub use crate::capabilities::controls::validation::{ValidationError, ValidationRules};

static EMAIL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex pattern")
});

actions!(
    input_state,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Copy,
        Cut,
        Paste,
        Enter,
        Tab,
        ShiftTab,
        Escape,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
    ]
);

/// Input types with built-in validation and filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Plain text input (default)
    Text,
    /// Email input with @ and domain validation
    Email,
    /// Numeric input (integers and decimals)
    Number,
    /// Phone number with formatting
    Tel,
    /// URL input with protocol validation
    Url,
    /// Password input with strength indicators
    Password,
    /// Search input with debounced events
    Search,
    /// Date input (YYYY-MM-DD format)
    Date,
    /// Time input (HH:MM format)
    Time,
    /// Credit card number with masking
    CreditCard,
}

/// Input masking for formatting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMask {
    /// No masking
    None,
    /// Phone: (999) 999-9999
    Phone,
    /// Credit card: 9999 9999 9999 9999
    CreditCard,
    /// Date: MM/DD/YYYY
    Date,
    /// Time: HH:MM
    Time,
    /// Custom mask pattern
    Custom(&'static str),
}

/// Events emitted by the InputState
#[derive(Clone, Debug)]
pub enum InputEvent {
    Change,
    Enter,
    Focus,
    Blur,
    Validate(Result<(), ValidationError>),
    Tab,
    ShiftTab,
}

#[derive(Clone, Copy)]
enum DeletionDirection {
    Backward,
    Forward,
}

/// Core input state entity that handles text editing with validation
pub struct InputState {
    focus_handle: FocusHandle,
    /// Canonical value before custom display formatting.
    raw_content: String,
    /// Last display string written by `InputState` itself. Since `content` is
    /// public for compatibility, this lets us distinguish an external write
    /// from a formatter change without reverse-running the formatter.
    last_synced_display: String,
    pub content: SharedString,
    pub placeholder: SharedString,
    pub disabled: bool,
    pub masked: bool,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<gpui::ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    was_focused: bool,
    last_click_time: Option<std::time::Instant>,
    last_click_position: Option<Point<Pixels>>,

    // Cursor blink state
    cursor_visible: bool,
    _blink_task: Option<Task<()>>,

    // Enhanced features
    pub input_type: InputType,
    pub validation_rules: ValidationRules,
    pub validation_error: Option<ValidationError>,
    pub input_mask: InputMask,
    pub aria_label: Option<SharedString>,
    pub aria_description: Option<SharedString>,
    pub autocomplete: Option<SharedString>,
    pub helper_text: Option<SharedString>,
    pub success_message: Option<SharedString>,
    pub tab_index: Option<i32>,
    pub select_on_focus: bool,
    pub validate_on_blur: bool,
    pub validate_on_change: bool,
    pub trim_on_blur: bool,
    pub shake_triggered: bool,
    pub(crate) shake_count: u32,
    cursor_position_override: Option<usize>,

    // Tracks whether the Input builder's `initial_value` has already been
    // applied to this state. Without this, `Input::render` would re-apply
    // the initial value every frame, overwriting user edits and emitting
    // a spurious `Change` event on each render.
    initial_value_applied: bool,
    // Tracks formatter ownership so removing a builder formatter does not
    // leave the previous display transformation installed on the state.
    builder_formatter_active: bool,
    event_subscription: Option<Subscription>,
}

impl EventEmitter<InputEvent> for InputState {}

impl InputState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            raw_content: String::new(),
            last_synced_display: String::new(),
            content: "".into(),
            placeholder: "Type here...".into(),
            disabled: false,
            masked: false,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            was_focused: false,
            last_click_time: None,
            last_click_position: None,
            cursor_visible: true,
            _blink_task: None,

            input_type: InputType::Text,
            validation_rules: ValidationRules::default(),
            validation_error: None,
            input_mask: InputMask::None,
            aria_label: None,
            aria_description: None,
            autocomplete: None,
            helper_text: None,
            success_message: None,
            tab_index: None,
            select_on_focus: false,
            validate_on_blur: true,
            validate_on_change: false,
            trim_on_blur: true,
            shake_triggered: false,
            shake_count: 0,
            cursor_position_override: None,
            initial_value_applied: false,
            builder_formatter_active: false,
            event_subscription: None,
        }
    }

    pub(super) fn replace_event_subscription(&mut self, subscription: Option<Subscription>) {
        self.event_subscription = subscription;
    }

    /// Set the input type
    pub fn input_type(mut self, input_type: InputType) -> Self {
        self.input_type = input_type;
        match input_type {
            InputType::Email => {
                self.placeholder = "email@example.com".into();
                self.autocomplete = Some("email".into());
            }
            InputType::Tel => {
                self.placeholder = "(555) 555-5555".into();
                self.input_mask = InputMask::Phone;
                self.autocomplete = Some("tel".into());
            }
            InputType::Url => {
                self.placeholder = "https://example.com".into();
                self.autocomplete = Some("url".into());
            }
            InputType::Password => {
                self.masked = true;
                self.autocomplete = Some("current-password".into());
            }
            InputType::CreditCard => {
                self.input_mask = InputMask::CreditCard;
                self.placeholder = "1234 5678 9012 3456".into();
                self.autocomplete = Some("cc-number".into());
            }
            InputType::Date => {
                self.input_mask = InputMask::Date;
                self.placeholder = "MM/DD/YYYY".into();
            }
            InputType::Time => {
                self.input_mask = InputMask::Time;
                self.placeholder = "HH:MM".into();
            }
            _ => {}
        }
        self
    }

    /// Set validation rules
    pub fn validation_rules(mut self, rules: ValidationRules) -> Self {
        self.validation_rules = rules;
        self
    }

    /// Set minimum length
    pub fn min_length(mut self, min: usize) -> Self {
        self.validation_rules.min_length = Some(min);
        self
    }

    /// Set maximum length
    pub fn max_length(mut self, max: usize) -> Self {
        self.validation_rules.max_length = Some(max);
        self
    }

    /// Mark field as required
    pub fn required(mut self, required: bool) -> Self {
        self.validation_rules.required = required;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Set the initial value.
    ///
    /// Equivalent to assigning `content` directly, but leaves the caret at
    /// the end of the text. This is raw initialization: it bypasses
    /// validation rules, filtering, and `max_length` truncation. To replace
    /// the value later with filtering, truncation, validation, and a
    /// `Change` event, use [`InputState::set_value`].
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        self.content = value.into();
        let len = self.content.len();
        self.selected_range = len..len;
        self
    }

    /// Set whether user interaction is disabled.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set whether the whole value is selected when the input gains focus.
    pub fn select_on_focus(mut self, select_on_focus: bool) -> Self {
        self.select_on_focus = select_on_focus;
        self
    }

    /// Set whether surrounding whitespace is trimmed on blur (default `true`).
    pub fn trim_on_blur(mut self, trim_on_blur: bool) -> Self {
        self.trim_on_blur = trim_on_blur;
        self
    }

    /// Set the placeholder text
    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.placeholder = placeholder.into();
        self.invalidate_layout_cache();
        cx.notify();
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// Whether an IME composition is currently in progress.
    ///
    /// While composing, the in-flight text is displayed but not committed:
    /// validation and `max_length` truncation only apply on commit.
    pub fn is_composing(&self) -> bool {
        self.marked_range.is_some()
    }

    /// Number of characters in the in-flight IME composition (0 when not
    /// composing). `content()` includes these characters, so the committed
    /// length is `content().chars().count() - composition_char_count()`.
    pub fn composition_char_count(&self) -> usize {
        // `content` is a public field: a direct external write can leave
        // `marked_range` stale, so slice defensively instead of panicking.
        self.marked_range
            .as_ref()
            .and_then(|range| self.content.get(range.clone()))
            .map(|marked| marked.chars().count())
            .unwrap_or(0)
    }

    /// Return the canonical value before display formatting.
    pub(crate) fn raw_value(&self) -> &str {
        &self.raw_content
    }

    /// Set the text content with validation
    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        self.sync_raw_content();

        // A formatter is allowed to omit all of the raw value from its
        // display (for example a visibility formatter that renders an empty
        // string).  In that case the display range is empty even though the
        // raw value is not.  Replacing through the display range would append
        // to the old raw value instead of replacing it.  Clear the raw value
        // first so the normal insertion path still applies filtering,
        // masking, length limits, validation, and event emission exactly
        // once.
        if self.content.is_empty() && !self.raw_content.is_empty() && self.marked_range.is_none() {
            self.raw_content.clear();
            self.selected_range = 0..0;
            self.set_display_content(String::new());
        }
        let full_range_utf16 = 0..self.offset_to_utf16(self.content.len());
        self.replace_text_in_range(Some(full_range_utf16), &value, window, cx);
    }

    pub(crate) fn value_matches(&self, value: &str) -> bool {
        // Do not replace text while an IME composition is active. The parent
        // value intentionally lags the composing value until `unmark_text`.
        if self.marked_range.is_some() {
            return true;
        }
        self.raw_content == value && self.content.as_ref() == self.format_content(value)
    }

    fn sync_raw_content(&mut self) {
        if self.content.as_ref() != self.last_synced_display {
            // `content` remains public for compatibility. An external write
            // becomes the next raw value; a formatter update leaves content
            // untouched and therefore preserves the existing raw value.
            self.raw_content.clear();
            self.raw_content.push_str(self.content.as_ref());
            self.last_synced_display = self.content.to_string();
        }
    }

    fn set_display_content(&mut self, display: String) {
        self.content = display.into();
        self.last_synced_display = self.content.to_string();
    }

    /// Re-apply a changed formatter while preserving the raw value and caret.
    /// Composition text is deliberately left unformatted until `unmark_text`.
    pub(crate) fn refresh_formatted_content(&mut self) {
        self.sync_raw_content();
        if self.marked_range.is_some() {
            return;
        }
        let old_display = self.content.to_string();
        if self.validation_rules.custom_formatter.is_none() {
            if old_display == self.raw_content {
                return;
            }
            let raw_range = Self::display_to_raw_index(
                &self.raw_content,
                &old_display,
                self.selected_range.start,
            )
                ..Self::display_to_raw_index(
                    &self.raw_content,
                    &old_display,
                    self.selected_range.end,
                );
            let raw = self.raw_content.clone();
            self.set_display_content(raw);
            self.selected_range = raw_range.start..raw_range.end;
            self.invalidate_layout_cache();
            return;
        }
        let new_display = self.format_content(&self.raw_content);
        if old_display == new_display {
            return;
        }
        let raw_range =
            Self::display_to_raw_index(&self.raw_content, &old_display, self.selected_range.start)
                ..Self::display_to_raw_index(
                    &self.raw_content,
                    &old_display,
                    self.selected_range.end,
                );
        self.set_display_content(new_display.clone());
        self.selected_range =
            Self::raw_to_display_index(&self.raw_content, &new_display, raw_range.start)
                ..Self::raw_to_display_index(&self.raw_content, &new_display, raw_range.end);
        self.invalidate_layout_cache();
    }

    pub(crate) fn sync_builder_formatter(&mut self, active: bool) {
        if !active && self.builder_formatter_active {
            self.validation_rules.custom_formatter = None;
        }
        self.builder_formatter_active = active;
    }

    /// Apply the builder-supplied initial value exactly once.
    ///
    /// `Input::render` runs every frame; calling `set_value` there would
    /// overwrite user edits and emit a `Change` event on every render.
    /// This helper applies the value only on the first call, so subsequent
    /// renders leave the state untouched. To forcefully replace the value
    /// later (e.g., resetting a form), call `set_value` directly.
    pub fn apply_initial_value(
        &mut self,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.initial_value_applied {
            return;
        }
        self.initial_value_applied = true;
        self.set_value(value, window, cx);
    }

    /// Validate the current input value
    pub fn validate(&mut self, cx: &mut Context<Self>) -> Result<(), ValidationError> {
        self.sync_raw_content();
        let value = self.committed_raw_value();
        let field_name = self.aria_label.clone();

        if let Some(result) = validation::check_required_empty(
            self.validation_rules.required,
            &value,
            field_name.clone(),
        ) {
            if result.is_ok() {
                // Optional empty value: clear any stale messages without
                // reporting success.
                self.validation_error = None;
                self.success_message = None;
                cx.emit(InputEvent::Validate(Ok(())));
                cx.notify();
                return Ok(());
            }
            return self.finish_validation(result, cx);
        }

        if let Err(error) = self.validate_input_type(&value) {
            return self.finish_validation(Err(error), cx);
        }

        let result = validation::check_constraints(&self.validation_rules, &value, field_name);
        self.finish_validation(result, cx)
    }

    fn validate_input_type(&self, value: &str) -> Result<(), ValidationError> {
        match self.input_type {
            InputType::Email => self.validate_email(value),
            InputType::Number => self.validate_number(value),
            InputType::Url => self.validate_url(value),
            InputType::Tel => self.validate_phone(value),
            InputType::Date => self.validate_date(value),
            InputType::Time => self.validate_time(value),
            InputType::CreditCard => self.validate_credit_card(value),
            _ => Ok(()),
        }
    }

    fn committed_raw_value(&self) -> String {
        let Some(range) = self.marked_range.as_ref() else {
            return self.raw_content.clone();
        };
        let Some(_) = self.raw_content.get(range.clone()) else {
            return self.raw_content.clone();
        };
        let mut committed = self.raw_content.clone();
        committed.replace_range(range.clone(), "");
        committed
    }

    fn finish_validation(
        &mut self,
        result: Result<(), ValidationError>,
        cx: &mut Context<Self>,
    ) -> Result<(), ValidationError> {
        match &result {
            Ok(()) => {
                self.validation_error = None;
                self.success_message = Some("Valid input".into());
            }
            Err(error) => {
                self.validation_error = Some(error.clone());
            }
        }
        cx.emit(InputEvent::Validate(result.clone()));
        cx.notify();
        result
    }

    pub fn trigger_shake(&mut self) {
        self.shake_triggered = true;
    }

    fn filter_input(&self, input: &str) -> String {
        if let Some(ref custom_filter) = self.validation_rules.custom_filter {
            return custom_filter(input);
        }

        match self.input_type {
            InputType::Number => input
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect(),
            InputType::Tel => input
                .chars()
                .filter(|c| {
                    c.is_ascii_digit()
                        || *c == '+'
                        || *c == '-'
                        || *c == '('
                        || *c == ')'
                        || *c == ' '
                })
                .collect(),
            InputType::Date => input
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '/' || *c == '-')
                .collect(),
            InputType::Time => input
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == ':')
                .collect(),
            InputType::CreditCard => input
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == ' ')
                .collect(),
            _ => input.to_string(),
        }
    }

    fn format_content(&self, content: &str) -> String {
        if content.is_empty() {
            return String::new();
        }

        self.validation_rules
            .custom_formatter
            .as_ref()
            .map(|formatter| formatter(content))
            .unwrap_or_else(|| content.to_owned())
    }

    fn raw_to_display_index(raw: &str, display: &str, raw_index: usize) -> usize {
        DisplayMapping::raw_to_display_index(raw, display, raw_index)
    }

    fn display_to_raw_index(raw: &str, display: &str, display_index: usize) -> usize {
        DisplayMapping::display_to_raw_index(raw, display, display_index)
    }

    fn apply_mask(&mut self, input: &str) -> String {
        match self.input_mask {
            InputMask::Phone => {
                let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() <= 3 {
                    digits
                } else if digits.len() <= 6 {
                    format!("({}) {}", &digits[0..3], &digits[3..])
                } else if digits.len() <= 10 {
                    format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..])
                } else {
                    format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..10])
                }
            }
            InputMask::CreditCard => {
                let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
                digits
                    .chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i > 0 && i % 4 == 0 {
                            format!(" {}", c)
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>()
            }
            InputMask::Date => {
                let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() <= 2 {
                    digits
                } else if digits.len() <= 4 {
                    format!("{}/{}", &digits[0..2], &digits[2..])
                } else if digits.len() <= 8 {
                    format!("{}/{}/{}", &digits[0..2], &digits[2..4], &digits[4..])
                } else {
                    format!("{}/{}/{}", &digits[0..2], &digits[2..4], &digits[4..8])
                }
            }
            InputMask::Time => {
                let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
                if digits.len() <= 2 {
                    digits
                } else if digits.len() <= 4 {
                    format!("{}:{}", &digits[0..2], &digits[2..])
                } else {
                    format!("{}:{}", &digits[0..2], &digits[2..4])
                }
            }
            _ => input.to_string(),
        }
    }

    fn validate_email(&self, email: &str) -> Result<(), ValidationError> {
        if !EMAIL_REGEX.is_match(email) {
            return Err(ValidationError {
                message: "Please enter a valid email address".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    fn validate_url(&self, url: &str) -> Result<(), ValidationError> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ValidationError {
                message: "URL must start with http:// or https://".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    fn validate_number(&self, number: &str) -> Result<(), ValidationError> {
        let parsed = number.parse::<f64>();
        if parsed.is_err() {
            return Err(ValidationError {
                message: "Please enter a valid number".into(),
                field_name: self.aria_label.clone(),
            });
        }

        if let Ok(value) = parsed {
            if let Some(min) = self.validation_rules.min_value {
                if value < min {
                    return Err(ValidationError {
                        message: format!("Must be at least {}", min).into(),
                        field_name: self.aria_label.clone(),
                    });
                }
            }
            if let Some(max) = self.validation_rules.max_value {
                if value > max {
                    return Err(ValidationError {
                        message: format!("Must be no more than {}", max).into(),
                        field_name: self.aria_label.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn validate_phone(&self, phone: &str) -> Result<(), ValidationError> {
        let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 10 {
            return Err(ValidationError {
                message: "Phone number must be at least 10 digits".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    fn validate_date(&self, date: &str) -> Result<(), ValidationError> {
        let parts: Vec<&str> = date.split('/').collect();
        if parts.len() != 3 {
            return Err(ValidationError {
                message: "Date must be in MM/DD/YYYY format".into(),
                field_name: self.aria_label.clone(),
            });
        }

        let month = parts[0].parse::<u32>().unwrap_or(0);
        let day = parts[1].parse::<u32>().unwrap_or(0);
        let year = parts[2].parse::<u32>().unwrap_or(0);

        if !(1..=12).contains(&month) {
            return Err(ValidationError {
                message: "Invalid month".into(),
                field_name: self.aria_label.clone(),
            });
        }
        if !(1..=31).contains(&day) {
            return Err(ValidationError {
                message: "Invalid day".into(),
                field_name: self.aria_label.clone(),
            });
        }
        if !(1900..=2100).contains(&year) {
            return Err(ValidationError {
                message: "Invalid year".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    fn validate_time(&self, time: &str) -> Result<(), ValidationError> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(ValidationError {
                message: "Time must be in HH:MM format".into(),
                field_name: self.aria_label.clone(),
            });
        }

        let hours = parts[0].parse::<u32>().unwrap_or(25);
        let minutes = parts[1].parse::<u32>().unwrap_or(61);

        if hours > 23 {
            return Err(ValidationError {
                message: "Invalid hours (0-23)".into(),
                field_name: self.aria_label.clone(),
            });
        }
        if minutes > 59 {
            return Err(ValidationError {
                message: "Invalid minutes (0-59)".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    fn validate_credit_card(&self, card: &str) -> Result<(), ValidationError> {
        let digits: String = card.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() < 13 || digits.len() > 19 {
            return Err(ValidationError {
                message: "Invalid credit card number".into(),
                field_name: self.aria_label.clone(),
            });
        }

        let mut sum = 0;
        let mut alternate = false;
        for c in digits.chars().rev() {
            let Some(mut digit) = c.to_digit(10) else {
                continue;
            };
            if alternate {
                digit *= 2;
                if digit > 9 {
                    digit -= 9;
                }
            }
            sum += digit;
            alternate = !alternate;
        }

        if sum % 10 != 0 {
            return Err(ValidationError {
                message: "Invalid credit card number".into(),
                field_name: self.aria_label.clone(),
            });
        }
        Ok(())
    }

    pub fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    pub fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    pub fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.previous_word_boundary(self.cursor_offset());
        if self.selected_range.is_empty() {
            self.move_to(offset, cx);
        } else {
            self.move_to(self.selected_range.start.min(offset), cx);
        }
    }

    pub fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.next_word_boundary(self.cursor_offset());
        if self.selected_range.is_empty() {
            self.move_to(offset, cx);
        } else {
            self.move_to(self.selected_range.end.max(offset), cx);
        }
    }

    pub fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    pub fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify()
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let cursor = self.cursor_offset();
            let previous = self.previous_boundary(cursor);
            if previous == cursor {
                return;
            }
            self.select_to(previous, cx)
        }
        self.replace_text_in_range_with_direction(
            Some(DeletionDirection::Backward),
            None,
            "",
            window,
            cx,
        )
    }

    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let cursor = self.cursor_offset();
            let next = self.next_boundary(cursor);
            if next == cursor {
                return;
            }
            self.select_to(next, cx)
        }
        self.replace_text_in_range_with_direction(
            Some(DeletionDirection::Forward),
            None,
            "",
            window,
            cx,
        )
    }

    pub fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next();
        cx.emit(InputEvent::Tab);
    }

    pub fn shift_tab(&mut self, _: &ShiftTab, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev();
        cx.emit(InputEvent::ShiftTab);
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        let now = std::time::Instant::now();
        let is_double_click = if let (Some(last_time), Some(last_pos)) =
            (self.last_click_time, self.last_click_position)
        {
            let time_diff = now.duration_since(last_time);
            let dx = event.position.x - last_pos.x;
            let dy = event.position.y - last_pos.y;
            let close_enough = dx < px(5.0) && dx > px(-5.0) && dy < px(5.0) && dy > px(-5.0);
            time_diff.as_millis() < 500 && close_enough
        } else {
            false
        };

        self.last_click_time = Some(now);
        self.last_click_position = Some(event.position);

        if is_double_click && !self.content.is_empty() {
            self.selected_range = 0..self.content.len();
            self.selection_reversed = false;
            self.reset_blink(cx);
            cx.notify();
            return;
        }

        let click_index = self.index_for_mouse_position(event.position);
        self.cursor_position_override = Some(click_index);

        if event.modifiers.shift {
            self.select_to(click_index, cx);
        } else {
            self.move_to(click_index, cx)
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let position = self.index_for_mouse_position(event.position);
            self.select_to(position, cx);
        }
    }

    pub fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let filtered_text = self.filter_input(&text.replace("\n", " "));
            self.replace_text_in_range(None, &filtered_text, window, cx);
        }
    }

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    pub fn enter(&mut self, _: &Enter, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(InputEvent::Enter);
        // Enter is a semantic action for a single-line input. Stop the action
        // here so GPUI does not fall through to character input and insert a
        // newline into a field that cannot represent one. Forms subscribe to
        // InputEvent::Enter instead of relying on action bubbling.
        cx.stop_propagation();
    }

    pub fn escape(&mut self, _: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = self.content.len()..self.content.len();
        cx.emit(InputEvent::Blur);
        cx.notify();
    }

    /// Called when the input gains focus
    pub fn on_focus(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.start_blink(cx);
        if self.select_on_focus && !self.content.is_empty() {
            self.selected_range = 0..self.content.len();
        } else if let Some(position) = self.cursor_position_override.take() {
            let position = self.clamp_offset(position);
            self.selected_range = position..position;
        } else if !self.content.is_empty() {
            let len = self.content.len();
            self.selected_range = len..len;
        }

        cx.emit(InputEvent::Focus);
        cx.notify();
    }

    /// Called when the input loses focus
    pub fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.stop_blink();
        self.cursor_position_override.take();
        if self.trim_on_blur {
            self.sync_raw_content();
            let trimmed = self.raw_content.trim().to_string();
            if trimmed != self.raw_content {
                self.set_value(trimmed, window, cx);
            }
        }

        if self.validate_on_blur {
            self.validate(cx).ok();
        }

        cx.emit(InputEvent::Blur);
        cx.notify();
    }

    // Syncs focus transitions observed during paint with focus/blur side
    // effects, mirroring TextareaState::sync_focus_state.
    fn sync_focus_state(&mut self, is_focused: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.was_focused == is_focused {
            return;
        }

        self.was_focused = is_focused;
        if is_focused {
            self.on_focus(window, cx);
        } else {
            self.is_selecting = false;
            self.on_blur(window, cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        text_util::cursor_offset(&self.selected_range, self.selection_reversed)
    }

    fn index_for_mouse_position(&mut self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if !self.layout_matches_current_display(line) {
            self.invalidate_layout_cache();
            return self.cursor_offset();
        }
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }

        let display_index = line.closest_index_for_x(position.x - bounds.left());
        self.display_to_source_index(display_index)
            .unwrap_or(self.content.len())
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        (self.selected_range, self.selection_reversed) = text_util::extend_selection(
            self.selected_range.clone(),
            self.selection_reversed,
            offset,
        );
        self.reset_blink(cx);
        cx.notify()
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        crate::capabilities::controls::text_util::offset_to_utf16(&self.content, offset)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        crate::capabilities::controls::text_util::range_to_utf16(&self.content, range)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        crate::capabilities::controls::text_util::range_from_utf16(&self.content, range_utf16)
    }

    pub(crate) fn invalidate_layout_cache(&mut self) {
        self.last_layout = None;
        self.last_bounds = None;
    }

    pub fn set_masked(&mut self, masked: bool, cx: &mut Context<Self>) {
        if self.masked == masked {
            return;
        }

        self.masked = masked;
        self.invalidate_layout_cache();
        cx.notify();
    }

    fn current_display_text(&self) -> SharedString {
        if self.content.is_empty() {
            self.placeholder.clone()
        } else if self.masked {
            "•".repeat(self.content.chars().count()).into()
        } else {
            self.content.clone()
        }
    }

    fn layout_matches_current_display(&self, layout: &gpui::ShapedLine) -> bool {
        layout.text == self.current_display_text()
    }

    fn source_to_display_index(&self, source_index: usize) -> Option<usize> {
        if self.content.is_empty() {
            return None;
        }

        if self.masked {
            let char_count = self.content.get(..source_index)?.chars().count();
            Some(char_count * "•".len())
        } else {
            Some(source_index.min(self.content.len()))
        }
    }

    fn display_to_source_index(&self, display_index: usize) -> Option<usize> {
        if self.content.is_empty() {
            return None;
        }

        if self.masked {
            let bullet_len = "•".len();
            let char_index = display_index / bullet_len;
            Some(
                self.content
                    .char_indices()
                    .nth(char_index)
                    .map(|(byte_idx, _)| byte_idx)
                    .unwrap_or(self.content.len()),
            )
        } else {
            Some(display_index.min(self.content.len()))
        }
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        text_util::clamp_offset(&self.content, offset)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        text_util::previous_grapheme_boundary(&self.content, offset)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        text_util::next_grapheme_boundary(&self.content, offset)
    }

    fn previous_word_boundary(&self, offset: usize) -> usize {
        text_util::previous_word_boundary(&self.content, offset)
    }

    fn next_word_boundary(&self, offset: usize) -> usize {
        text_util::next_word_boundary(&self.content, offset)
    }

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self._blink_task = None;
        self.cursor_visible = true;
        self._blink_task = Some(cx.spawn(async |this, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let ok = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                this.cursor_visible = !this.cursor_visible;
                cx.notify();
            });
            if ok.is_err() {
                break;
            }
        }));
    }

    fn stop_blink(&mut self) {
        self._blink_task = None;
        self.cursor_visible = true;
    }

    fn reset_blink(&mut self, cx: &mut Context<Self>) {
        self.stop_blink();
        self.start_blink(cx);
    }

    fn replace_text_in_range_with_direction(
        &mut self,
        deletion_direction: Option<DeletionDirection>,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_raw_content();
        let had_marked_text = self.marked_range.is_some();
        let display_range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let mut range =
            Self::display_to_raw_index(&self.raw_content, &self.content, display_range.start)
                ..Self::display_to_raw_index(&self.raw_content, &self.content, display_range.end);
        if new_text.is_empty() && !display_range.is_empty() && range.is_empty() {
            if let Some(direction) = deletion_direction {
                let fallback = match direction {
                    DeletionDirection::Backward if range.start > 0 => Some(
                        text_util::previous_grapheme_boundary(&self.raw_content, range.start)
                            ..range.start,
                    ),
                    DeletionDirection::Forward if range.end < self.raw_content.len() => Some(
                        range.end..text_util::next_grapheme_boundary(&self.raw_content, range.end),
                    ),
                    _ => None,
                };
                let Some(fallback) = fallback else {
                    // The selected display text contains formatting
                    // characters only (for example a currency prefix). There
                    // is no raw value to remove in this direction.
                    return;
                };
                range = fallback;
            }
        }

        let filtered_text = self.filter_input(new_text);
        let formatted_text = if self.input_mask != InputMask::None {
            self.apply_mask(&filtered_text)
        } else {
            filtered_text
        };

        let formatted_text = if let Some(max_length) = self.validation_rules.max_length {
            let retained_chars =
                self.raw_content.chars().count() - self.raw_content[range.clone()].chars().count();
            let allowed_chars = max_length.saturating_sub(retained_chars);
            formatted_text
                .chars()
                .take(allowed_chars)
                .collect::<String>()
        } else {
            formatted_text
        };

        let next_raw = self.raw_content[0..range.start].to_owned()
            + &formatted_text
            + &self.raw_content[range.end..];
        let content_changed = next_raw != self.raw_content;
        let next_display = self.format_content(&next_raw);
        let raw_cursor = range.start + formatted_text.len();
        let cursor = Self::raw_to_display_index(&next_raw, &next_display, raw_cursor);

        self.raw_content = next_raw;
        self.set_display_content(next_display);
        self.selected_range = cursor..cursor;

        self.marked_range.take();
        self.selection_reversed = false;
        if self.focus_handle.is_focused(window) {
            self.reset_blink(cx);
        }
        self.invalidate_layout_cache();

        if (content_changed || had_marked_text) && self.validate_on_change {
            self.validate(cx).ok();
        }

        if content_changed || had_marked_text {
            cx.emit(InputEvent::Change);
        }
        cx.notify();
    }

    pub fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let had_marked_text = self.marked_range.is_some();
        self.sync_raw_content();
        if self.validation_rules.custom_formatter.is_some() {
            let raw_selected_range = self.selected_range.clone();
            let formatted = self.format_content(&self.raw_content);
            self.set_display_content(formatted);
            self.selected_range = Self::raw_to_display_index(
                &self.raw_content,
                &self.content,
                raw_selected_range.start,
            )
                ..Self::raw_to_display_index(
                    &self.raw_content,
                    &self.content,
                    raw_selected_range.end,
                );
        }
        self.marked_range = None;
        self.invalidate_layout_cache();
        if had_marked_text {
            if self.validate_on_change {
                self.validate(cx).ok();
            }
            cx.emit(InputEvent::Change);
        }
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range_with_direction(None, range_utf16, new_text, window, cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.sync_raw_content();
        let display_range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());
        let range =
            Self::display_to_raw_index(&self.raw_content, &self.content, display_range.start)
                ..Self::display_to_raw_index(&self.raw_content, &self.content, display_range.end);

        let filtered_text = self.filter_input(new_text);
        let next_raw = self.raw_content[0..range.start].to_owned()
            + &filtered_text
            + &self.raw_content[range.end..];
        self.raw_content = next_raw;

        // Keep the composition text editable until the IME commits it. The
        // formatter is applied once by `unmark_text`.
        self.set_display_content(self.raw_content.clone());
        if !filtered_text.is_empty() {
            self.marked_range = Some(range.start..range.start + filtered_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| text_util::range_from_utf16(&filtered_text, range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| {
                range.start + filtered_text.len()..range.start + filtered_text.len()
            });

        self.selection_reversed = false;
        if self.focus_handle.is_focused(window) {
            self.reset_blink(cx);
        }
        self.invalidate_layout_cache();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout_matches = {
            let last_layout = self.last_layout.as_ref()?;
            self.layout_matches_current_display(last_layout)
        };
        if !layout_matches {
            self.invalidate_layout_cache();
            return None;
        }

        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let display_start = self.source_to_display_index(range.start)?;
        let display_end = self.source_to_display_index(range.end)?;
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(display_start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(display_end),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let layout_matches = {
            let last_layout = self.last_layout.as_ref()?;
            self.layout_matches_current_display(last_layout)
        };
        if !layout_matches {
            self.invalidate_layout_cache();
            return None;
        }

        let last_layout = self.last_layout.as_ref()?;
        let display_index = last_layout.index_for_x(point.x - line_point.x)?;
        let utf8_index = self.display_to_source_index(display_index)?;
        Some(self.offset_to_utf16(utf8_index))
    }
}

/// Custom element for rendering the input text with cursor and selection.
/// This is THE KEY to text input - it calls window.handle_input() in the paint method
struct InputTextElement {
    input: Entity<InputState>,
}

struct PrepaintState {
    line: Option<gpui::ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for InputTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for InputTextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let (content, selected_range, cursor) = if input.masked {
            let char_count = input.content.chars().count();
            let masked_text = "•".repeat(char_count).into();

            let start_chars = input.content[..input.selected_range.start].chars().count();
            let end_chars = input.content[..input.selected_range.end].chars().count();
            let masked_selected_range = (start_chars * "•".len())..(end_chars * "•".len());

            let cursor_chars = if input.selection_reversed {
                start_chars
            } else {
                end_chars
            };
            let masked_cursor = cursor_chars * "•".len();

            (masked_text, masked_selected_range, masked_cursor)
        } else {
            (
                input.content.clone(),
                input.selected_range.clone(),
                input.cursor_offset(),
            )
        };
        let style = window.text_style();
        let theme = use_theme(cx);

        let (display_text, text_color) = if content.is_empty() {
            (input.placeholder.clone(), theme.tokens.muted_foreground)
        } else {
            (content, style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            let (marked_start, marked_end) = if input.masked {
                let start_chars = input.content[..marked_range.start].chars().count();
                let end_chars = input.content[..marked_range.end].chars().count();
                (start_chars * "•".len(), end_chars * "•".len())
            } else {
                (marked_range.start, marked_range.end)
            };

            vec![
                TextRun {
                    len: marked_start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_end - marked_start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top()),
                        size(px(2.), bounds.bottom() - bounds.top()),
                    ),
                    theme.tokens.primary,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    theme.tokens.primary.opacity(0.25),
                )),
                None,
            )
        };
        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        let disabled = self.input.read(cx).disabled;

        if !disabled {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.input.clone()),
                cx,
            );
        }

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }
        let Some(line) = prepaint.line.take() else {
            return;
        };
        if line
            .paint(bounds.origin, window.line_height(), window, cx)
            .is_err()
        {
            return;
        }

        let is_focused = focus_handle.is_focused(window);
        if is_focused {
            let cursor_visible = self.input.read(cx).cursor_visible;
            if cursor_visible {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        }

        self.input.update(cx, |input, cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
            input.sync_focus_state(is_focused, window, cx);
        });
    }
}

impl Render for InputState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = cx.entity();

        div()
            .w_full()
            .h_full()
            .on_mouse_down(MouseButton::Left, {
                let input = input.clone();
                move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                    input.update(cx, |input, cx| {
                        input.on_mouse_down(event, window, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let input = input.clone();
                move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    input.update(cx, |input, cx| {
                        input.on_mouse_up(event, window, cx);
                    });
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let input = input.clone();
                move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    input.update(cx, |input, cx| {
                        input.on_mouse_up(event, window, cx);
                    });
                }
            })
            .on_mouse_move({
                let input = input.clone();
                move |event: &MouseMoveEvent, window: &mut Window, cx: &mut App| {
                    input.update(cx, |input, cx| {
                        input.on_mouse_move(event, window, cx);
                    });
                }
            })
            .child(InputTextElement { input })
    }
}

impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

use regex;

/// Wire a bare [`InputState`] into a custom container with full keyboard
/// editing, using the same wiring the `Input` component applies to its shell.
///
/// `InputState` renders only the text body (text, caret, selection, IME
/// underline). The editing keys are key bindings scoped to the `"Input"`
/// context and are attached by the `Input` component's container — a
/// container an application renders itself must reproduce that wiring, or
/// every editing key except plain text insertion silently does nothing.
/// This helper attaches:
///
/// - the `"Input"` key context the key bindings are filtered on,
/// - focus tracking for the state's focus handle (click-to-focus, tab order),
/// - all 20 keyboard actions, including word navigation, unless the state
///   is [`InputState::disabled`].
///
/// The returned div is an unstyled container; style it and add the state as
/// its child:
///
/// ```rust,ignore
/// use fc_ui::components::input_state::wire_actions;
///
/// fn render(state: &Entity<InputState>, window: &mut Window, cx: &App) -> impl IntoElement {
///     wire_actions(state, window, cx)
///         .h(px(36.0))
///         .px(px(12.0))
///         .rounded(px(8.0))
///         .child(state.clone())
/// }
/// ```
///
/// The key bindings are registered by `components::input::init`, which
/// `fc_ui::init` calls at startup; applications that skip crate init
/// must call it themselves or no keystroke will match an action.
pub fn wire_actions(state: &Entity<InputState>, window: &mut Window, cx: &App) -> Stateful<Div> {
    let disabled = state.read(cx).disabled;
    let focus_handle = state.read(cx).focus_handle(cx);

    div()
        .id(("input", state.entity_id()))
        .key_context("Input")
        .track_focus(&focus_handle.tab_index(0).tab_stop(true))
        .when(!disabled, |this| {
            this.on_action(window.listener_for(state, InputState::backspace))
                .on_action(window.listener_for(state, InputState::delete))
                .on_action(window.listener_for(state, InputState::left))
                .on_action(window.listener_for(state, InputState::right))
                .on_action(window.listener_for(state, InputState::select_left))
                .on_action(window.listener_for(state, InputState::select_right))
                .on_action(window.listener_for(state, InputState::select_all))
                .on_action(window.listener_for(state, InputState::home))
                .on_action(window.listener_for(state, InputState::end))
                .on_action(window.listener_for(state, InputState::copy))
                .on_action(window.listener_for(state, InputState::cut))
                .on_action(window.listener_for(state, InputState::paste))
                .on_action(window.listener_for(state, InputState::enter))
                .on_action(window.listener_for(state, InputState::tab))
                .on_action(window.listener_for(state, InputState::shift_tab))
                .on_action(window.listener_for(state, InputState::escape))
                .on_action(window.listener_for(state, InputState::word_left))
                .on_action(window.listener_for(state, InputState::word_right))
                .on_action(window.listener_for(state, InputState::select_word_left))
                .on_action(window.listener_for(state, InputState::select_word_right))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::foundation::theme::{install_theme, Theme};
    use std::sync::Arc;

    struct TestView {
        state: Entity<InputState>,
    }

    impl Render for TestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn builders_configure_common_state_options() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| {
            cx.new(|cx| {
                InputState::new(cx)
                    .value("  hello  ")
                    .placeholder("p")
                    .disabled(true)
                    .select_on_focus(true)
                    .trim_on_blur(false)
            })
        });

        app.update(|cx| {
            let state = state.read(cx);
            assert_eq!(state.content(), "  hello  ");
            assert_eq!(state.placeholder, "p");
            assert!(state.disabled);
            assert!(state.select_on_focus);
            assert!(!state.trim_on_blur);
        });
    }

    #[gpui::test]
    fn word_left_moves_cursor_to_previous_word_boundary() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        let text = "foo bar baz";
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value(text, window, cx);
                state.selected_range = text.len()..text.len();
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 8..8, "should stop before baz");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 4..4, "should stop before bar");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 0..0, "should reach start");
            });
        });
    }

    #[gpui::test]
    fn word_right_moves_cursor_to_next_word_boundary() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("foo bar baz", window, cx);
                state.selected_range = 0..0;
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 4..4, "should stop at bar start");
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 8..8, "should stop at baz start");
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 11..11, "should reach end");
            });
        });
    }

    #[gpui::test]
    fn word_movement_handles_unicode() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        let text = "中文 test";
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value(text, window, cx);
                state.selected_range = text.len()..text.len();
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 7..7, "should stop before test");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 0..0, "should reach start");
            });
        });
    }

    #[gpui::test]
    fn ime_marked_replacement_maps_selection_into_inserted_text() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("ab中文cd", window, cx);
                state.selected_range = 2..8; // "中文"

                // IME 组字提交：替换非空选区；new_selected_range 是相对新插入文本的
                // UTF-16 区间，修复后的语义是 new_range + range.start
                state.replace_and_mark_text_in_range(None, "xy测试", Some(2..4), window, cx);

                assert_eq!(state.content(), "abxy测试cd");
                // "xy测试" 的 UTF-16 2..4 == 其字节 2..8（"测试"），再整体偏移到插入点 2
                let range = state.selected_range.clone();
                assert_eq!(range, 4..10);
                assert_eq!(&state.content()[range.clone()], "测试");
                assert!(state.content().is_char_boundary(range.start));
                assert!(state.content().is_char_boundary(range.end));
                assert_eq!(state.marked_range, Some(2..10));
                assert_eq!(
                    &state.content()[state.marked_range.clone().unwrap()],
                    "xy测试"
                );
            });
        });
    }

    #[gpui::test]
    fn ime_commit_replacing_nonempty_selection_places_cursor_after_insert() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("ab中文cd", window, cx);
                state.selected_range = 2..8; // "中文"

                state.replace_text_in_range(None, "新字", window, cx);

                assert_eq!(state.content(), "ab新字cd");
                assert_eq!(state.selected_range, 8..8);
                assert!(state.content().is_char_boundary(state.selected_range.start));
            });
        });
    }

    #[gpui::test]
    fn validation_pattern_is_enforced() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.pattern = Some(r"^[A-Z]{3}-\d{4}$".into());
                state.set_value("ABC-1234", window, cx);
                assert!(state.validate(cx).is_ok());

                state.set_value("abc-1234", window, cx);
                assert!(state.validate(cx).is_err());
            });
        });
    }

    #[gpui::test]
    fn custom_formatter_supports_editing_and_clear() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter = Some(Arc::new(|value| {
                    format!("${:.2}", value.parse::<f64>().unwrap_or(0.0))
                }));
                state.set_value("12", window, cx);
                assert_eq!(state.content(), "$12.00");

                state.replace_text_in_range(None, "3", window, cx);
                assert_eq!(state.content(), "$123.00");

                state.set_value("", window, cx);
                assert!(state.content().is_empty());
            });
        });
    }

    #[gpui::test]
    fn formatter_preserves_middle_cursor_position() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter = Some(Arc::new(|value| value.to_owned()));
                state.set_value("abcd", window, cx);
                state.selected_range = 1..1;

                state.replace_text_in_range(None, "X", window, cx);

                assert_eq!(state.content(), "aXbcd");
                assert_eq!(state.selected_range, 2..2);
            });
        });
    }

    #[gpui::test]
    fn formatter_cursor_mapping_handles_inserted_separators_and_affixes() {
        assert_eq!(
            InputState::raw_to_display_index("1234", "1,234", 2),
            3,
            "the cursor after raw `12` should move past the grouping separator"
        );
        assert_eq!(InputState::display_to_raw_index("1234", "1,234", 3), 2);
        assert_eq!(InputState::raw_to_display_index("12", "$12.00", 0), 1);
        assert_eq!(InputState::raw_to_display_index("12", "$12.00", 2), 6);
        assert_eq!(InputState::display_to_raw_index("12", "$12.00", 6), 2);
    }

    #[gpui::test]
    fn formatter_cursor_mapping_uses_character_boundaries_for_unicode_expansion() {
        // `İ`.to_lowercase() expands to two display characters (`i` plus a
        // combining dot), whose UTF-8 length is three bytes.  Both ends of
        // the raw value must map to display character boundaries.
        let raw = "İ";
        let display = "i\u{307}";
        assert_eq!(InputState::raw_to_display_index(raw, display, 0), 0);
        assert_eq!(
            InputState::raw_to_display_index(raw, display, raw.len()),
            display.len()
        );
        assert_eq!(InputState::display_to_raw_index(raw, display, 0), 0);
        assert_eq!(
            InputState::display_to_raw_index(raw, display, display.len()),
            raw.len()
        );
        assert!(display.is_char_boundary(InputState::raw_to_display_index(
            raw,
            display,
            raw.len()
        )));
    }

    #[gpui::test]
    fn formatter_cursor_mapping_falls_back_for_reordered_text() {
        // A reverse formatter has no monotonic raw/display inverse.  The
        // mapping remains monotonic and usable for editing instead of using
        // the first accidental character match as an anchor.
        let raw = "abcd";
        let display = "dcba";
        assert_eq!(InputState::raw_to_display_index(raw, display, 0), 0);
        assert_eq!(
            InputState::raw_to_display_index(raw, display, raw.len()),
            display.len()
        );
        assert_eq!(InputState::display_to_raw_index(raw, display, 0), 0);
        assert_eq!(
            InputState::display_to_raw_index(raw, display, display.len()),
            raw.len()
        );
    }

    #[gpui::test]
    fn set_value_replaces_raw_text_when_formatter_hides_display() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter = Some(Arc::new(|_| String::new()));
                state.set_value("first", window, cx);
                assert_eq!(state.raw_value(), "first");
                assert!(state.content().is_empty());

                state.set_value("second", window, cx);
                assert_eq!(state.raw_value(), "second");
                assert!(state.content().is_empty());
            });
        });
    }

    #[gpui::test]
    fn removing_a_formatter_restores_the_raw_display_value() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter =
                    Some(Arc::new(|value| format!("${value}")));
                state.set_value("12", window, cx);
                state.validation_rules.custom_formatter = None;
                state.refresh_formatted_content();

                assert_eq!(state.content(), "12");
            });
        });
    }

    #[gpui::test]
    fn formatter_is_applied_when_ime_composition_ends() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter =
                    Some(Arc::new(|value| value.to_uppercase()));
                state.replace_and_mark_text_in_range(None, "ab", None, window, cx);
                assert_eq!(state.content(), "ab");

                state.unmark_text(window, cx);

                assert_eq!(state.content(), "AB");
                assert!(state.marked_range.is_none());
            });
        });
    }

    #[gpui::test]
    fn ime_commit_emits_change_when_composition_ends() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let changes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let changes_for_subscription = changes.clone();
        app.update(|cx| {
            cx.subscribe(&state, move |_emitter, event: &InputEvent, _cx| {
                if matches!(event, InputEvent::Change) {
                    changes_for_subscription.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
            })
            .detach();
        });
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "ab", None, window, cx);
                assert_eq!(
                    changes.load(std::sync::atomic::Ordering::SeqCst),
                    0,
                    "composition updates should not emit a committed change"
                );
                state.unmark_text(window, cx);
            });
        });

        assert_eq!(
            changes.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "ending composition should emit one committed change"
        );
    }

    #[gpui::test]
    fn externally_changed_public_content_is_used_as_the_next_raw_value() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.custom_formatter =
                    Some(Arc::new(|value| format!("<{value}>")));
                state.set_value("12", window, cx);
                state.content = "34".into();
                state.selected_range = 2..2;

                state.replace_text_in_range(None, "5", window, cx);

                assert_eq!(state.content(), "<345>");
            });
        });
    }

    #[gpui::test]
    fn invalid_validation_pattern_is_rejected() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.validation_rules.pattern = Some("[".into());
                state.set_value("abc", window, cx);

                let error = state.validate(cx).expect_err("invalid regex should fail");
                assert_eq!(error.message.as_ref(), "Invalid validation pattern");
            });
        });
    }

    #[gpui::test]
    fn grapheme_deletion_removes_whole_unicode_clusters() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("a👩‍💻e\u{301}b", window, cx);
                state.backspace(&Backspace, window, cx);
                assert_eq!(state.content(), "a👩‍💻e\u{301}");
                assert_eq!(
                    state.selected_range,
                    state.content().len()..state.content().len()
                );

                state.backspace(&Backspace, window, cx);
                assert_eq!(state.content(), "a👩‍💻");
                assert_eq!(
                    state.selected_range,
                    state.content().len()..state.content().len()
                );

                state.selected_range = "a".len().."a".len();
                state.selection_reversed = false;
                state.delete(&Delete, window, cx);
                assert_eq!(state.content(), "a");
                assert_eq!(state.selected_range, "a".len().."a".len());
            });
        });
    }

    #[gpui::test]
    fn selection_direction_flips_when_crossing_the_anchor() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                let text = "a😀b";
                let emoji_start = "a".len();
                let emoji_end = "a😀".len();

                state.set_value(text, window, cx);
                state.selected_range = emoji_start..emoji_start;
                state.selection_reversed = false;
                state.select_right(&SelectRight, window, cx);
                assert_eq!(state.selected_range, emoji_start..emoji_end);
                assert!(!state.selection_reversed);

                state.select_left(&SelectLeft, window, cx);
                assert_eq!(state.selected_range, emoji_start..emoji_start);
                assert!(!state.selection_reversed);
                state.select_left(&SelectLeft, window, cx);
                assert_eq!(state.selected_range, 0..emoji_start);
                assert!(state.selection_reversed);

                state.selected_range = emoji_end..emoji_end;
                state.selection_reversed = false;
                state.select_left(&SelectLeft, window, cx);
                assert_eq!(state.selected_range, emoji_start..emoji_end);
                assert!(state.selection_reversed);
                state.select_right(&SelectRight, window, cx);
                assert_eq!(state.selected_range, emoji_end..emoji_end);
                assert!(state.selection_reversed);
                state.select_right(&SelectRight, window, cx);
                assert_eq!(state.selected_range, emoji_end..text.len());
                assert!(!state.selection_reversed);
            });
        });
    }

    #[gpui::test]
    fn utf16_ranges_preserve_astral_character_boundaries() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("a😀b", window, cx);
                let mut actual_range = None;
                assert_eq!(
                    state.text_for_range(1..3, &mut actual_range, window, cx),
                    Some("😀".to_string())
                );
                assert_eq!(actual_range, Some(1..3));

                state.selected_range = "a".len().."a😀".len();
                let selected = state
                    .selected_text_range(false, window, cx)
                    .expect("selection should be available");
                assert_eq!(selected.range, 1..3);
                assert!(!selected.reversed);

                state.replace_text_in_range(Some(1..3), "", window, cx);
                assert_eq!(state.content(), "ab");
                assert_eq!(state.selected_range, 1..1);
            });
        });
    }
    #[gpui::test]
    fn ime_composition_state_is_queryable() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(InputState::new));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("hello", window, cx);
                assert!(!state.is_composing());
                assert_eq!(state.composition_char_count(), 0);

                // IME starts composing "nihao" at the caret.
                state.replace_and_mark_text_in_range(None, "nihao", None, window, cx);
                assert!(state.is_composing());
                assert_eq!(state.composition_char_count(), 5);
                // Committed length excludes the in-flight composition.
                assert_eq!(
                    state.content().chars().count() - state.composition_char_count(),
                    5
                );
            });
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                // Committing the composed text ends the composition.
                state.replace_text_in_range(None, "你好", window, cx);
                assert!(!state.is_composing());
                assert_eq!(state.composition_char_count(), 0);
                assert_eq!(state.content(), "hello你好");
            });
        });
    }
}
