//! Shared editing core for the Input / Textarea / Mention state machines.
//!
//! [`EditCore`] is deliberately stateless: it owns no fields and no gpui
//! context. Every method is a pure function over the caller's content and
//! selection state, returning the next selection (plus direction flag) so
//! each state keeps ownership of `notify()`, `emit()` and its own event
//! type. Leaf text primitives (grapheme boundaries, UTF-16 mapping,
//! selection extension) live in [`super::text_util`]; this module owns the
//! composite editing operations the three states previously duplicated
//! line for line, plus the [`CaretBlink`] runtime they share.

use std::ops::Range;

use gpui::{Context, SharedString, Task, UTF16Selection};

use crate::capabilities::controls::text_util;
use crate::capabilities::controls::validation::{
    TextConstraints, ValidationError, ValidationRules,
};
use crate::capabilities::foundation::edit;

/// A builder temporarily owns only fields it explicitly overrides. Removing
/// the override restores the value captured on entry. External writes remain
/// live whenever no override owns the field; writes while owned are discarded.
pub(crate) struct BuilderOverride<T> {
    baseline: Option<T>,
}

impl<T> Default for BuilderOverride<T> {
    fn default() -> Self {
        Self { baseline: None }
    }
}

impl<T: Clone> BuilderOverride<T> {
    pub(crate) fn restore(&mut self, value: &mut T) {
        if let Some(baseline) = self.baseline.take() {
            *value = baseline;
        }
    }

    pub(crate) fn apply(&mut self, value: &mut T, next: Option<T>) {
        if let Some(next) = next {
            self.baseline = Some(value.clone());
            *value = next;
        }
    }
}

/// Caret movement shared by the text-editing states, applied with
/// [`EditCore::apply_motion`].
pub(crate) enum Motion {
    /// Collapse to the selection start, or step one grapheme left when the
    /// selection is already empty.
    Left,
    /// Collapse to the selection end, or step one grapheme right when the
    /// selection is already empty.
    Right,
    /// Collapse left to the previous word boundary, or to the selection
    /// start — whichever is further left.
    WordLeft,
    /// Collapse right to the next word boundary, or to the selection end —
    /// whichever is further right.
    WordRight,
    /// Extend the selection one grapheme left.
    SelectLeft,
    /// Extend the selection one grapheme right.
    SelectRight,
    /// Extend the selection to the previous word boundary.
    SelectWordLeft,
    /// Extend the selection to the next word boundary.
    SelectWordRight,
    /// Select the entire text.
    SelectAll,
    /// Collapse to the start of the text (single-line semantics; multi-line
    /// states resolve the enclosing line's start first).
    Home,
    /// Collapse to the end of the text (single-line semantics).
    End,
}

/// What a backspace/delete keypress should do once resolved against the
/// current selection, from [`EditCore::deletion_plan`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum DeletionPlan {
    /// The selection is empty and the caret sits on the text edge: there is
    /// nothing to delete.
    Nothing,
    /// The selection is empty but a neighboring grapheme exists: extend
    /// over it first — the returned selection (plus direction flag) is the
    /// range the caller replaces.
    Extend(Range<usize>, bool),
    /// The selection is non-empty: replace it directly.
    Selection,
}

/// Pure editing operations shared by the Input, Textarea and Mention states.
///
/// The core owns no state and no gpui context: every method is a pure
/// function over the caller's content and selection, returning the next
/// selection (plus direction flag) so each state keeps ownership of
/// `notify()`, `emit()` and its own event type.
pub(crate) struct EditCore;

impl EditCore {
    // ── Selection queries ───────────────────────────────────────────

    /// Active caret offset for a normalized selection.
    pub(crate) fn cursor_offset(selected_range: &Range<usize>, selection_reversed: bool) -> usize {
        text_util::cursor_offset(selected_range, selection_reversed)
    }

    /// Clamp `offset` into the valid byte range of `content`.
    pub(crate) fn clamp_offset(content: &str, offset: usize) -> usize {
        text_util::clamp_offset(content, offset)
    }

    // ── Grapheme / word boundaries ─────────────────────────────────

    /// Start byte index of the grapheme cluster strictly before `offset`.
    pub(crate) fn previous_boundary(content: &str, offset: usize) -> usize {
        text_util::previous_grapheme_boundary(content, offset)
    }

    /// Start byte index of the grapheme cluster strictly after `offset`.
    pub(crate) fn next_boundary(content: &str, offset: usize) -> usize {
        text_util::next_grapheme_boundary(content, offset)
    }

    /// Previous word boundary at or before `offset`.
    pub(crate) fn previous_word_boundary(content: &str, offset: usize) -> usize {
        text_util::previous_word_boundary(content, offset)
    }

    /// Next word boundary at or after `offset`.
    pub(crate) fn next_word_boundary(content: &str, offset: usize) -> usize {
        text_util::next_word_boundary(content, offset)
    }

    // ── Selection mutation (pure) ──────────────────────────────────

    /// Clamp `offset` and collapse the selection onto it.
    pub(crate) fn move_to(content: &str, offset: usize) -> (Range<usize>, bool) {
        let offset = Self::clamp_offset(content, offset);
        (offset..offset, false)
    }

    /// Clamp `offset` and extend the selection to it, normalizing bounds
    /// and direction.
    pub(crate) fn select_to(
        content: &str,
        selected_range: Range<usize>,
        selection_reversed: bool,
        offset: usize,
    ) -> (Range<usize>, bool) {
        let offset = Self::clamp_offset(content, offset);
        text_util::extend_selection(selected_range, selection_reversed, offset)
    }

    /// Select the whole content with a forward direction.
    pub(crate) fn select_all(content: &str) -> (Range<usize>, bool) {
        (0..content.len(), false)
    }

    /// Resolve `motion` against the current selection, returning the next
    /// selection and direction.
    pub(crate) fn apply_motion(
        content: &str,
        selected_range: &Range<usize>,
        selection_reversed: bool,
        motion: Motion,
    ) -> (Range<usize>, bool) {
        let cursor = Self::cursor_offset(selected_range, selection_reversed);
        match motion {
            Motion::Left => {
                if selected_range.is_empty() {
                    Self::move_to(content, Self::previous_boundary(content, cursor))
                } else {
                    Self::move_to(content, selected_range.start)
                }
            }
            Motion::Right => {
                if selected_range.is_empty() {
                    Self::move_to(content, Self::next_boundary(content, selected_range.end))
                } else {
                    Self::move_to(content, selected_range.end)
                }
            }
            Motion::WordLeft => {
                let offset = Self::previous_word_boundary(content, cursor);
                if selected_range.is_empty() {
                    Self::move_to(content, offset)
                } else {
                    Self::move_to(content, selected_range.start.min(offset))
                }
            }
            Motion::WordRight => {
                let offset = Self::next_word_boundary(content, cursor);
                if selected_range.is_empty() {
                    Self::move_to(content, offset)
                } else {
                    Self::move_to(content, selected_range.end.max(offset))
                }
            }
            Motion::SelectLeft => Self::select_to(
                content,
                selected_range.clone(),
                selection_reversed,
                Self::previous_boundary(content, cursor),
            ),
            Motion::SelectRight => Self::select_to(
                content,
                selected_range.clone(),
                selection_reversed,
                Self::next_boundary(content, cursor),
            ),
            Motion::SelectWordLeft => Self::select_to(
                content,
                selected_range.clone(),
                selection_reversed,
                Self::previous_word_boundary(content, cursor),
            ),
            Motion::SelectWordRight => Self::select_to(
                content,
                selected_range.clone(),
                selection_reversed,
                Self::next_word_boundary(content, cursor),
            ),
            Motion::SelectAll => Self::select_all(content),
            Motion::Home => Self::move_to(content, 0),
            Motion::End => Self::move_to(content, content.len()),
        }
    }

    // ── Deletion ───────────────────────────────────────────────────

    /// Resolve a backspace (`forward = false`) or delete (`forward = true`)
    /// keypress against the current selection.
    pub(crate) fn deletion_plan(
        content: &str,
        selected_range: &Range<usize>,
        selection_reversed: bool,
        forward: bool,
    ) -> DeletionPlan {
        if !selected_range.is_empty() {
            return DeletionPlan::Selection;
        }
        let cursor = Self::cursor_offset(selected_range, selection_reversed);
        let target = if forward {
            Self::next_boundary(content, cursor)
        } else {
            Self::previous_boundary(content, cursor)
        };
        if target == cursor {
            return DeletionPlan::Nothing;
        }
        let (range, reversed) =
            text_util::extend_selection(selected_range.clone(), selection_reversed, target);
        DeletionPlan::Extend(range, reversed)
    }

    // ── UTF-16 conversion (EntityInputHandler coordinates) ─────────

    /// Map a UTF-8 byte offset into UTF-16 code units.
    pub(crate) fn offset_to_utf16(content: &str, offset: usize) -> usize {
        text_util::offset_to_utf16(content, offset)
    }

    /// Map a UTF-8 byte range into UTF-16 code units.
    pub(crate) fn range_to_utf16(content: &str, range: &Range<usize>) -> Range<usize> {
        text_util::range_to_utf16(content, range)
    }

    /// Map a UTF-16 range back into UTF-8 byte offsets.
    pub(crate) fn range_from_utf16(content: &str, range_utf16: &Range<usize>) -> Range<usize> {
        text_util::range_from_utf16(content, range_utf16)
    }

    // ── Text access ────────────────────────────────────────────────

    /// The text covered by `selected_range`.
    pub(crate) fn selected_text(content: &str, selected_range: &Range<usize>) -> String {
        content[selected_range.clone()].to_string()
    }

    /// Body of [`gpui::EntityInputHandler::text_for_range`].
    pub(crate) fn text_for_range(
        content: &str,
        range_utf16: &Range<usize>,
        actual_range: &mut Option<Range<usize>>,
    ) -> Option<String> {
        let range = Self::range_from_utf16(content, range_utf16);
        actual_range.replace(Self::range_to_utf16(content, &range));
        content.get(range).map(str::to_string)
    }

    /// Body of [`gpui::EntityInputHandler::selected_text_range`].
    pub(crate) fn selected_text_range(
        content: &str,
        selected_range: &Range<usize>,
        selection_reversed: bool,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: Self::range_to_utf16(content, selected_range),
            reversed: selection_reversed,
        })
    }

    /// Body of [`gpui::EntityInputHandler::marked_text_range`].
    pub(crate) fn marked_text_range(
        content: &str,
        marked_range: Option<&Range<usize>>,
    ) -> Option<Range<usize>> {
        marked_range.map(|range| Self::range_to_utf16(content, range))
    }

    // ── Replacement ────────────────────────────────────────────────

    /// Splice `new_text` over `range`, returning the next content and the
    /// caret offset directly after the inserted text.
    pub(crate) fn replace_range(
        content: &str,
        range: &Range<usize>,
        new_text: &str,
    ) -> (String, usize) {
        let mut next = String::with_capacity(content.len() + new_text.len());
        next.push_str(&content[..range.start]);
        next.push_str(new_text);
        next.push_str(&content[range.end..]);
        (next, range.start + new_text.len())
    }

    // ── Validation bookkeeping ─────────────────────────────────────

    /// Whether the rules carry any constraint that can fail (used to gate
    /// automatic blur/change validation).
    pub(crate) fn has_validation_rules(rules: &ValidationRules) -> bool {
        TextConstraints::from(rules).has_rules()
    }

    /// Update the public error/success slots from a validation outcome —
    /// the half of `finish_validation` every text state shares. `None` is an
    /// optional empty value: stale messages are cleared without reporting
    /// success. Emitting the state-specific `Validate` event and notifying
    /// stay with the caller.
    pub(crate) fn apply_validation_outcome(
        validation_error: &mut Option<ValidationError>,
        success_message: &mut Option<SharedString>,
        outcome: Option<&Result<(), ValidationError>>,
    ) {
        match outcome {
            None => {
                *validation_error = None;
                *success_message = None;
            }
            Some(Ok(())) => {
                *validation_error = None;
                *success_message = Some("Valid input".into());
            }
            Some(Err(error)) => {
                *validation_error = Some(error.clone());
            }
        }
    }
}

/// States that own a [`CaretBlink`], letting the blink task toggle the
/// caret without the core holding a gpui context.
pub(crate) trait HasCaretBlink {
    fn caret_blink(&mut self) -> &mut CaretBlink;
}

/// Caret blink runtime shared by the text-editing states: the visibility
/// flag plus the pending blink task. The state layer decides when to start,
/// stop and reset it; this type only drives the periodic toggle.
pub(crate) struct CaretBlink {
    /// Whether the caret is currently in its visible phase.
    pub(crate) visible: bool,
    task: Option<Task<()>>,
}

impl Default for CaretBlink {
    fn default() -> Self {
        Self {
            visible: true,
            task: None,
        }
    }
}

impl CaretBlink {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Start blinking from the visible phase, replacing any previous task.
    pub(crate) fn start<T: HasCaretBlink + 'static>(&mut self, cx: &mut Context<T>) {
        self.task = None;
        self.visible = true;
        self.task = Some(edit::spawn_caret_blink(
            cx,
            |state: &mut T, cx: &mut Context<T>| {
                let blink = state.caret_blink();
                blink.visible = !blink.visible;
                cx.notify();
            },
        ));
    }

    /// Stop blinking and make the caret visible again.
    pub(crate) fn stop(&mut self) {
        self.task = None;
        self.visible = true;
    }

    /// Restart the blink so the caret stays visible right after a move.
    pub(crate) fn reset<T: HasCaretBlink + 'static>(&mut self, cx: &mut Context<T>) {
        self.stop();
        self.start(cx);
    }
}

/// Generate the builder methods `InputState` and `TextareaState` share
/// verbatim (placeholder, disabled and the validation-rule setters), so the
/// two states cannot drift apart.
macro_rules! impl_shared_edit_builders {
    ($state:ty) => {
        impl $state {
            /// Set the placeholder text.
            pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
                self.placeholder = placeholder.into();
                self
            }

            /// Set whether user interaction is disabled.
            pub fn disabled(mut self, disabled: bool) -> Self {
                self.disabled = disabled;
                self
            }

            /// Set validation rules.
            pub fn validation_rules(mut self, rules: ValidationRules) -> Self {
                self.validation_rules = rules;
                self
            }

            /// Set the minimum length requirement.
            pub fn min_length(mut self, min: usize) -> Self {
                self.validation_rules.min_length = Some(min);
                self
            }

            /// Set the maximum length requirement.
            pub fn max_length(mut self, max: usize) -> Self {
                self.validation_rules.max_length = Some(max);
                self
            }

            /// Mark the field as required.
            pub fn required(mut self, required: bool) -> Self {
                self.validation_rules.required = required;
                self
            }
        }
    };
}

pub(crate) use impl_shared_edit_builders;

/// Attach `window.listener_for` handlers for a list of action methods to a
/// container, replacing the repeated `.on_action(...)` chains the Input and
/// Textarea containers previously duplicated.
macro_rules! attach_action_handlers {
    ($container:expr, $window:expr, $state:expr $(, $handler:path)* $(,)?) => {{
        let mut container = $container;
        $(
            container = container.on_action($window.listener_for($state, $handler));
        )*
        container
    }};
}

pub(crate) use attach_action_handlers;

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "foo bar baz";

    #[test]
    fn move_to_clamps_and_collapses() {
        assert_eq!(EditCore::move_to(TEXT, 4), (4..4, false));
        assert_eq!(
            EditCore::move_to(TEXT, 999),
            (TEXT.len()..TEXT.len(), false)
        );
        assert_eq!(EditCore::move_to("", 3), (0..0, false));
    }

    #[test]
    fn select_to_extends_and_normalizes() {
        assert_eq!(EditCore::select_to(TEXT, 4..4, false, 8), (4..8, false));
        assert_eq!(EditCore::select_to(TEXT, 4..4, false, 2), (2..4, true));
        assert_eq!(
            EditCore::select_to(TEXT, 4..4, false, 999),
            (4..TEXT.len(), false)
        );
        assert_eq!(EditCore::select_to(TEXT, 4..8, true, 6), (6..8, true));
    }

    #[test]
    fn motions_match_single_line_editing_semantics() {
        let end = TEXT.len()..TEXT.len();

        // Empty selection: left steps one grapheme, word-left skips a word.
        assert_eq!(
            EditCore::apply_motion(TEXT, &end, false, Motion::Left),
            (10..10, false)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &end, false, Motion::WordLeft),
            (8..8, false)
        );
        // Non-empty selection: left/right collapse to the range edges.
        assert_eq!(
            EditCore::apply_motion(TEXT, &(4..8), false, Motion::Left),
            (4..4, false)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &(4..8), true, Motion::Right),
            (8..8, false)
        );
        // Word motions never move past the far edge of the selection.
        assert_eq!(
            EditCore::apply_motion(TEXT, &(4..8), false, Motion::WordLeft),
            (4..4, false)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &(4..8), false, Motion::WordRight),
            (11..11, false)
        );
        // Select variants extend the selection and flip direction.
        assert_eq!(
            EditCore::apply_motion(TEXT, &end, false, Motion::SelectLeft),
            (10..11, true)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &(0..0), false, Motion::SelectRight),
            (0..1, false)
        );
        // Home/end/select-all.
        assert_eq!(
            EditCore::apply_motion(TEXT, &end, false, Motion::Home),
            (0..0, false)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &(0..0), false, Motion::End),
            (11..11, false)
        );
        assert_eq!(
            EditCore::apply_motion(TEXT, &(2..5), true, Motion::SelectAll),
            (0..11, false)
        );
    }

    #[test]
    fn motions_step_over_whole_graphemes() {
        let text = "a😀b";
        let end = text.len()..text.len();
        assert_eq!(
            EditCore::apply_motion(text, &end, false, Motion::Left),
            (5..5, false),
            "left stops before the astral emoji, not inside it"
        );
        assert_eq!(
            EditCore::apply_motion(text, &(0..0), false, Motion::SelectRight),
            (0..1, false)
        );
    }

    #[test]
    fn deletion_plan_covers_edges_and_selections() {
        assert!(matches!(
            EditCore::deletion_plan(TEXT, &(0..0), false, false),
            DeletionPlan::Nothing
        ));
        assert!(matches!(
            EditCore::deletion_plan(TEXT, &(TEXT.len()..TEXT.len()), false, true),
            DeletionPlan::Nothing
        ));
        assert!(matches!(
            EditCore::deletion_plan(TEXT, &(1..3), false, false),
            DeletionPlan::Selection
        ));
        assert_eq!(
            EditCore::deletion_plan(TEXT, &(3..3), false, false),
            DeletionPlan::Extend(2..3, true),
            "extending left past the anchor flips the direction flag, as select_to always did"
        );
        assert_eq!(
            EditCore::deletion_plan("a😀b", &(5..5), false, false),
            DeletionPlan::Extend(1..5, true),
            "backspace consumes the whole emoji cluster"
        );
    }

    #[test]
    fn replace_range_splices_and_places_the_caret() {
        assert_eq!(
            EditCore::replace_range("hello", &(1..3), "EL"),
            ("hELlo".to_string(), 3)
        );
        assert_eq!(
            EditCore::replace_range("hello", &(0..0), "x"),
            ("xhello".to_string(), 1)
        );
        assert_eq!(
            EditCore::replace_range("hello", &(0..5), ""),
            (String::new(), 0)
        );
    }

    #[test]
    fn entity_input_handler_bodies_map_utf16_ranges() {
        let text = "a😀b";
        let mut actual = None;
        assert_eq!(
            EditCore::text_for_range(text, &(1..3), &mut actual),
            Some("😀".to_string())
        );
        assert_eq!(actual, Some(1..3));

        let selected = EditCore::selected_text_range(text, &(1..5), false).unwrap();
        assert_eq!(selected.range, 1..3);
        assert!(!selected.reversed);

        assert_eq!(EditCore::marked_text_range(text, Some(&(1..5))), Some(1..3));
        assert_eq!(EditCore::marked_text_range(text, None), None);
    }
}
