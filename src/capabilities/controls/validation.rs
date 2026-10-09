//! Shared validation machinery for text-entry controls (`Input`, `Textarea`).
//!
//! Both `InputState` and `TextareaState` build on these primitives so that
//! validation rules, error reporting, and the footer row (error/success/
//! helper text plus character count) exist exactly once.

use gpui::{prelude::FluentBuilder as _, *};
use std::sync::Arc;

use crate::capabilities::foundation::theme::ThemeTokens;
use crate::capabilities::layout::HStack;

pub type ValidationCallback = Arc<dyn Fn(&str) -> Result<(), ValidationError>>;
pub type TextTransformer = Arc<dyn Fn(&str) -> String>;

/// Validation result with error message
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: SharedString,
    pub field_name: Option<SharedString>,
}

/// Input validation rules
#[derive(Clone, Default)]
pub struct ValidationRules {
    /// Minimum length requirement
    pub min_length: Option<usize>,
    /// Maximum length requirement
    pub max_length: Option<usize>,
    /// For number inputs: minimum value
    pub min_value: Option<f64>,
    /// For number inputs: maximum value
    pub max_value: Option<f64>,
    /// Regex pattern to match
    pub pattern: Option<String>,
    /// Custom validation function
    pub custom_validator: Option<ValidationCallback>,
    /// Custom filter function for input
    pub custom_filter: Option<TextTransformer>,
    /// Custom formatter function for display
    pub custom_formatter: Option<TextTransformer>,
    /// Whether the field is required
    pub required: bool,
}

impl std::fmt::Debug for ValidationRules {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationRules")
            .field("min_length", &self.min_length)
            .field("max_length", &self.max_length)
            .field("min_value", &self.min_value)
            .field("max_value", &self.max_value)
            .field("pattern", &self.pattern)
            .field("custom_validator", &self.custom_validator.is_some())
            .field("custom_filter", &self.custom_filter.is_some())
            .field("custom_formatter", &self.custom_formatter.is_some())
            .field("required", &self.required)
            .finish()
    }
}

/// Empty-value gate shared by text-entry states.
///
/// Returns `Some(result)` when validation is already decided by the value
/// being empty: `Err` when the field is required, `Ok` when it is not (the
/// remaining constraints are skipped for optional empty values). Returns
/// `None` when the value is non-empty and constraint checks should continue.
pub(crate) fn check_required_empty(
    required: bool,
    value: &str,
    field_name: Option<SharedString>,
) -> Option<Result<(), ValidationError>> {
    if required && value.trim().is_empty() {
        return Some(Err(ValidationError {
            message: "This field is required".into(),
            field_name,
        }));
    }
    if !required && value.trim().is_empty() {
        return Some(Ok(()));
    }
    None
}

/// Length, pattern, and custom-validator checks shared by text-entry states.
///
/// Assumes the empty-value gate has already run (see [`check_required_empty`]).
pub(crate) fn check_constraints(
    rules: &ValidationRules,
    value: &str,
    field_name: Option<SharedString>,
) -> Result<(), ValidationError> {
    if let Some(min_length) = rules.min_length {
        if value.chars().count() < min_length {
            return Err(ValidationError {
                message: format!("Must be at least {} characters", min_length).into(),
                field_name,
            });
        }
    }

    if let Some(max_length) = rules.max_length {
        if value.chars().count() > max_length {
            return Err(ValidationError {
                message: format!("Must be no more than {} characters", max_length).into(),
                field_name,
            });
        }
    }

    if let Some(pattern) = rules.pattern.as_deref() {
        match regex::Regex::new(pattern) {
            Ok(regex) if !regex.is_match(value) => {
                return Err(ValidationError {
                    message: "Value does not match the required pattern".into(),
                    field_name,
                });
            }
            Err(_) => {
                return Err(ValidationError {
                    message: "Invalid validation pattern".into(),
                    field_name,
                });
            }
            Ok(_) => {}
        }
    }

    if let Some(ref validator) = rules.custom_validator {
        validator(value)?;
    }

    Ok(())
}

/// Character-count display state for [`validation_footer`].
pub(crate) struct CharacterCount {
    /// Committed characters (excludes in-flight IME composition).
    pub committed: usize,
    /// In-flight IME composition characters (0 when not composing).
    pub composing: usize,
    /// Configured maximum, if any.
    pub max: Option<usize>,
}

/// Format the character counter for text-entry footers.
///
/// Returns the counter text and whether it should render in the destructive
/// color. While an IME composition is in flight, its characters are shown
/// separately (`12+3/140`) because they are not committed yet; an impending
/// overflow already turns the counter red so the user sees the composition
/// will not fit before committing it.
pub(crate) fn format_character_count(
    committed: usize,
    composing: usize,
    max: Option<usize>,
) -> (String, bool) {
    let mut text = committed.to_string();
    if composing > 0 {
        text.push('+');
        text.push_str(&composing.to_string());
    }
    if let Some(max) = max {
        text.push('/');
        text.push_str(&max.to_string());
    }
    let over = max.is_some_and(|max| committed >= max || committed + composing > max);
    (text, over)
}

/// Message slot of [`validation_footer`]: at most one of error, success, or
/// helper text is shown, in this priority order.
pub(crate) enum FooterMessage<'a> {
    Error(&'a ValidationError),
    Success(&'a SharedString),
    Helper(&'a SharedString),
    /// Render an empty message slot (keeps layout height stable).
    Blank,
}

/// Footer row shared by `Input` and `Textarea`: error/success/helper text on
/// the left, optional character count on the right.
///
/// The counter turns destructive when the committed text reaches `max`, and
/// while composing also when the in-flight composition would overflow. Pass
/// `None` to hide the counter.
pub(crate) fn validation_footer(
    tokens: &ThemeTokens,
    message: FooterMessage,
    character_count: Option<CharacterCount>,
) -> impl IntoElement {
    let message_element = match message {
        FooterMessage::Error(error) => div()
            .text_color(tokens.destructive)
            .child(error.message.clone()),
        FooterMessage::Success(success) => div().text_color(tokens.primary).child(success.clone()),
        FooterMessage::Helper(helper) => div()
            .text_color(tokens.muted_foreground)
            .child(helper.clone()),
        FooterMessage::Blank => div(),
    };

    HStack::new()
        .w_full()
        .px(px(2.0))
        .child(
            div()
                .flex_1()
                .text_size(tokens.text_xs)
                .font_family(tokens.font_family.clone())
                .child(message_element),
        )
        .when_some(character_count, |h, count| {
            let (text, over) = format_character_count(count.committed, count.composing, count.max);
            h.child(
                div()
                    .text_size(tokens.text_xs)
                    .font_family(tokens.font_family.clone())
                    .text_color(if over {
                        tokens.destructive
                    } else {
                        tokens.muted_foreground
                    })
                    .child(text),
            )
        })
}

#[cfg(test)]
mod tests {
    // Precise imports instead of `super::*`: the parent module glob-imports
    // gpui, whose `test` attribute would clash with the built-in `#[test]`.
    use super::{
        check_constraints, check_required_empty, format_character_count, CharacterCount,
        ValidationError, ValidationRules,
    };
    use std::sync::Arc;

    #[test]
    fn required_empty_fails() {
        let result = check_required_empty(true, "  ", None);
        assert!(matches!(result, Some(Err(_))));
    }

    #[test]
    fn optional_empty_passes_without_constraint_checks() {
        let rules = ValidationRules {
            min_length: Some(3),
            ..Default::default()
        };
        assert!(matches!(
            check_required_empty(false, "", None),
            Some(Ok(()))
        ));
        // The gate decides empty values; constraints never run for them.
        assert!(check_required_empty(rules.required, "", None).is_some());
    }

    #[test]
    fn constraints_enforce_lengths() {
        let rules = ValidationRules {
            min_length: Some(2),
            max_length: Some(4),
            ..Default::default()
        };
        assert!(check_constraints(&rules, "a", None).is_err());
        assert!(check_constraints(&rules, "abcde", None).is_err());
        assert!(check_constraints(&rules, "abc", None).is_ok());
    }

    #[test]
    fn constraints_count_chars_not_bytes() {
        let rules = ValidationRules {
            max_length: Some(2),
            ..Default::default()
        };
        assert!(check_constraints(&rules, "你好", None).is_ok());
        assert!(check_constraints(&rules, "你好呀", None).is_err());
    }

    #[test]
    fn constraints_apply_pattern_and_custom_validator() {
        let rules = ValidationRules {
            pattern: Some(r"^\d+$".to_string()),
            ..Default::default()
        };
        assert!(check_constraints(&rules, "123", None).is_ok());
        assert!(check_constraints(&rules, "12a", None).is_err());

        let rules = ValidationRules {
            custom_validator: Some(Arc::new(|value| {
                if value.contains('x') {
                    Ok(())
                } else {
                    Err(ValidationError {
                        message: "needs x".into(),
                        field_name: None,
                    })
                }
            })),
            ..Default::default()
        };
        assert!(check_constraints(&rules, "oxo", None).is_ok());
        assert!(check_constraints(&rules, "ooo", None).is_err());
    }

    #[test]
    fn character_count_formats_plain_and_max() {
        assert_eq!(
            format_character_count(12, 0, None),
            ("12".to_string(), false)
        );
        assert_eq!(
            format_character_count(12, 0, Some(140)),
            ("12/140".to_string(), false)
        );
    }

    #[test]
    fn character_count_shows_composition_separately() {
        assert_eq!(
            format_character_count(12, 3, Some(140)),
            ("12+3/140".to_string(), false)
        );
    }

    #[test]
    fn character_count_flags_full_and_impending_overflow() {
        // Committed text reached the limit.
        assert_eq!(
            format_character_count(140, 0, Some(140)),
            ("140/140".to_string(), true)
        );
        // Composition has not committed yet but would not fit.
        assert_eq!(
            format_character_count(138, 5, Some(140)),
            ("138+5/140".to_string(), true)
        );
        // Composition fits exactly: no warning.
        assert_eq!(
            format_character_count(138, 2, Some(140)),
            ("138+2/140".to_string(), false)
        );
        // Without a limit there is nothing to overflow.
        assert_eq!(
            format_character_count(10, 50, None),
            ("10+50".to_string(), false)
        );
    }

    #[test]
    fn character_count_struct_is_what_footer_consumes() {
        let count = CharacterCount {
            committed: 3,
            composing: 2,
            max: Some(4),
        };
        let (_, over) = format_character_count(count.committed, count.composing, count.max);
        assert!(over);
    }
}
