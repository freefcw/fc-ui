use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

pub(crate) fn offset_from_utf16(text: &str, offset: usize) -> usize {
    let mut utf8_offset = 0;
    let mut utf16_count = 0;

    for ch in text.chars() {
        if utf16_count >= offset {
            break;
        }
        utf16_count += ch.len_utf16();
        utf8_offset += ch.len_utf8();
    }

    utf8_offset
}

pub(crate) fn offset_to_utf16(text: &str, offset: usize) -> usize {
    let mut utf16_offset = 0;
    let mut utf8_count = 0;

    for ch in text.chars() {
        if utf8_count >= offset {
            break;
        }
        utf8_count += ch.len_utf8();
        utf16_offset += ch.len_utf16();
    }

    utf16_offset
}

pub(crate) fn range_to_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    offset_to_utf16(text, range.start)..offset_to_utf16(text, range.end)
}

pub(crate) fn range_from_utf16(text: &str, range: &Range<usize>) -> Range<usize> {
    offset_from_utf16(text, range.start)..offset_from_utf16(text, range.end)
}

/// Clamp `offset` into the valid byte range of `text`.
pub(crate) fn clamp_offset(text: &str, offset: usize) -> usize {
    offset.min(text.len())
}

/// Return the active cursor offset for a normalized selection.
pub(crate) fn cursor_offset(selection: &Range<usize>, selection_reversed: bool) -> usize {
    if selection_reversed {
        selection.start
    } else {
        selection.end
    }
}

/// Extend a selection to `offset` and normalize its bounds and direction.
pub(crate) fn extend_selection(
    mut selection: Range<usize>,
    mut selection_reversed: bool,
    offset: usize,
) -> (Range<usize>, bool) {
    if selection_reversed {
        selection.start = offset;
    } else {
        selection.end = offset;
    }
    if selection.end < selection.start {
        selection_reversed = !selection_reversed;
        selection = selection.end..selection.start;
    }
    (selection, selection_reversed)
}

/// Return the start byte index of the grapheme cluster strictly before `offset`.
pub(crate) fn previous_grapheme_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .rev()
        .find_map(|(idx, _)| (idx < offset).then_some(idx))
        .unwrap_or(0)
}

/// Return the start byte index of the grapheme cluster strictly after `offset`.
pub(crate) fn next_grapheme_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .find_map(|(idx, _)| (idx > offset).then_some(idx))
        .unwrap_or(text.len())
}

/// Whether the grapheme cluster spanning `start..end` is entirely whitespace.
fn grapheme_is_whitespace(text: &str, start: usize, end: usize) -> bool {
    start < end && text[start..end].chars().all(|ch| ch.is_whitespace())
}

/// Whether the grapheme cluster spanning `start..end` is a word character
/// (alphanumeric or underscore).
fn grapheme_is_word(text: &str, start: usize, end: usize) -> bool {
    start < end
        && text[start..end]
            .chars()
            .all(|ch| ch.is_alphanumeric() || ch == '_')
}

/// Previous word boundary at or before `offset`: skips trailing whitespace, then
/// consumes the contiguous run of the same character class (word / non-word / space).
pub(crate) fn previous_word_boundary(text: &str, offset: usize) -> usize {
    let mut current = clamp_offset(text, offset);

    while current > 0 {
        let previous = previous_grapheme_boundary(text, current);
        if !grapheme_is_whitespace(text, previous, current) {
            break;
        }
        current = previous;
    }

    if current == 0 {
        return 0;
    }

    let previous = previous_grapheme_boundary(text, current);
    let is_word = grapheme_is_word(text, previous, current);

    while current > 0 {
        let previous = previous_grapheme_boundary(text, current);
        if grapheme_is_whitespace(text, previous, current)
            || grapheme_is_word(text, previous, current) != is_word
        {
            break;
        }
        current = previous;
    }

    current
}

/// Next word boundary at or after `offset`: consumes the current character class run,
/// then skips any trailing whitespace.
pub(crate) fn next_word_boundary(text: &str, offset: usize) -> usize {
    let mut current = clamp_offset(text, offset);
    let len = text.len();

    if current >= len {
        return len;
    }

    let next = next_grapheme_boundary(text, current);

    if grapheme_is_word(text, current, next) {
        while current < len {
            let next = next_grapheme_boundary(text, current);
            if next == current || !grapheme_is_word(text, current, next) {
                break;
            }
            current = next;
        }
    } else if !grapheme_is_whitespace(text, current, next) {
        while current < len {
            let next = next_grapheme_boundary(text, current);
            if next == current
                || grapheme_is_whitespace(text, current, next)
                || grapheme_is_word(text, current, next)
            {
                break;
            }
            current = next;
        }
    }

    while current < len {
        let next = next_grapheme_boundary(text, current);
        if next == current || !grapheme_is_whitespace(text, current, next) {
            break;
        }
        current = next;
    }

    if current == offset {
        next_grapheme_boundary(text, current)
    } else {
        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_unicode_boundaries_between_utf8_and_utf16() {
        let text = "a😀é";
        assert_eq!(offset_to_utf16(text, text.len()), 4);
        assert_eq!(offset_from_utf16(text, 3), "a😀".len());
        assert_eq!(range_to_utf16(text, &(1.."a😀".len())), 1..3);
        assert_eq!(range_from_utf16(text, &(1..3)), 1.."a😀".len());
    }

    #[test]
    fn utf16_boundaries_round_trip_astral_characters() {
        let text = "a😀b";
        let emoji_start = "a".len();
        let emoji_end = "a😀".len();

        assert_eq!(offset_to_utf16(text, 0), 0);
        assert_eq!(offset_to_utf16(text, emoji_start), 1);
        assert_eq!(offset_to_utf16(text, emoji_end), 3);
        assert_eq!(offset_to_utf16(text, text.len()), 4);
        assert_eq!(offset_from_utf16(text, 0), 0);
        assert_eq!(offset_from_utf16(text, 1), emoji_start);
        assert_eq!(offset_from_utf16(text, 3), emoji_end);
        assert_eq!(offset_from_utf16(text, 4), text.len());
        assert_eq!(range_to_utf16(text, &(emoji_start..emoji_end)), 1..3);
        assert_eq!(range_from_utf16(text, &(1..3)), emoji_start..emoji_end);
    }

    #[test]
    fn clamp_offset_stays_within_text_bounds() {
        assert_eq!(clamp_offset("abc", 0), 0);
        assert_eq!(clamp_offset("abc", 2), 2);
        assert_eq!(clamp_offset("abc", 10), 3);
        assert_eq!(clamp_offset("", 5), 0);
    }

    #[test]
    fn cursor_offset_follows_selection_direction() {
        assert_eq!(cursor_offset(&(2..5), false), 5);
        assert_eq!(cursor_offset(&(2..5), true), 2);
    }

    #[test]
    fn extend_selection_normalizes_crossed_ranges() {
        assert_eq!(extend_selection(2..5, false, 4), (2..4, false));
        assert_eq!(extend_selection(2..5, false, 1), (1..2, true));
        assert_eq!(extend_selection(2..5, true, 3), (3..5, true));
        assert_eq!(extend_selection(2..5, true, 6), (5..6, false));
    }

    #[test]
    fn grapheme_boundaries_skip_clusters_not_bytes() {
        // a😀é  -> grapheme starts at 0, 1, 5
        let text = "a😀é";
        assert_eq!(previous_grapheme_boundary(text, 1), 0);
        assert_eq!(previous_grapheme_boundary(text, 5), 1);
        assert_eq!(previous_grapheme_boundary(text, 0), 0);
        assert_eq!(next_grapheme_boundary(text, 0), 1);
        assert_eq!(next_grapheme_boundary(text, 1), 5);
        assert_eq!(next_grapheme_boundary(text, text.len()), text.len());
    }

    #[test]
    fn grapheme_class_helpers_detect_classes() {
        // a(0) space(1) _(2) 中(3..6)
        let text = "a _中";
        assert!(grapheme_is_word(text, 0, 1)); // a
        assert!(!grapheme_is_word(text, 1, 2)); // space
        assert!(grapheme_is_word(text, 2, 3)); // _
        assert!(grapheme_is_word(text, 3, 6)); // 中
        assert!(grapheme_is_whitespace(text, 1, 2));
        assert!(!grapheme_is_whitespace(text, 0, 1));
        assert!(!grapheme_is_word(text, 0, 0));
    }

    #[test]
    fn previous_word_boundary_skips_trailing_whitespace_and_class_run() {
        let text = "foo bar";
        assert_eq!(previous_word_boundary(text, 7), 4); // before "bar"
        assert_eq!(previous_word_boundary(text, 3), 0); // before "foo"
        assert_eq!(previous_word_boundary(text, 0), 0);

        let mixed = "foo, bar";
        // at end, skip whitespace then stop at 'bar' start
        assert_eq!(previous_word_boundary(mixed, 8), 5);
        // inside punctuation run before space -> land on the space
        assert_eq!(previous_word_boundary(mixed, 4), 3);
    }

    #[test]
    fn next_word_boundary_skips_class_run_then_whitespace() {
        let text = "foo bar";
        assert_eq!(next_word_boundary(text, 0), 4); // end of "foo" + space
        assert_eq!(next_word_boundary(text, 4), 7); // end of "bar"
        assert_eq!(next_word_boundary(text, 7), 7); // already at end

        let mixed = "foo, bar";
        // from start of foo -> skip foo, stop at comma boundary
        assert_eq!(next_word_boundary(mixed, 0), 3);
        // at the end -> no movement
        assert_eq!(next_word_boundary(mixed, 8), 8);
    }

    #[test]
    fn word_boundary_handles_underscore_and_cjk_and_emoji() {
        // underscore is part of a word
        let snake = "foo_bar baz";
        assert_eq!(previous_word_boundary(snake, 10), 8);
        assert_eq!(previous_word_boundary(snake, 8), 0);
        assert_eq!(next_word_boundary(snake, 0), 8);

        // CJK: each grapheme is alphanumeric, treated as one word run
        let cjk = "中文 test";
        assert_eq!(next_word_boundary(cjk, 0), 7);
        assert_eq!(previous_word_boundary(cjk, 7), 0);

        // emoji is NOT alphanumeric, treated as its own non-word class run
        let emoji = "a😀 b";
        let emoji_start = "a".len();
        assert_eq!(next_word_boundary(emoji, 0), 1);
        assert_eq!(
            next_word_boundary(emoji, emoji_start),
            emoji_start + "😀".len() + " ".len()
        );
    }

    #[test]
    fn word_boundary_empty_and_single_char() {
        assert_eq!(previous_word_boundary("", 0), 0);
        assert_eq!(next_word_boundary("", 0), 0);
        assert_eq!(previous_word_boundary("a", 1), 0);
        assert_eq!(next_word_boundary("a", 0), 1);
    }
}
