//! Shared helpers for control components (select/combobox navigation, test
//! selectors).

/// Cycle a list highlight one position `up` (or down), wrapping around.
///
/// `None` selects the first entry when moving down and the last entry when
/// moving up, matching the select/combobox keyboard navigation behavior.
/// Callers must guard the empty-list case; `len == 0` returns `0`.
pub(crate) fn cycle_highlight(len: usize, current: Option<usize>, up: bool) -> usize {
    if len == 0 {
        return 0;
    }
    match current {
        None if up => len - 1,
        None => 0,
        Some(current) => {
            let current = current.min(len - 1);
            if up {
                if current == 0 {
                    len - 1
                } else {
                    current - 1
                }
            } else if current >= len - 1 {
                0
            } else {
                current + 1
            }
        }
    }
}

/// Attach a stable debug selector to an element (test builds only).
#[cfg(test)]
pub(crate) fn with_test_selector<E: gpui::InteractiveElement>(
    element: E,
    selector: &'static str,
) -> E {
    element.debug_selector(move || selector.into())
}

/// No-op in non-test builds.
#[cfg(not(test))]
pub(crate) fn with_test_selector<E>(element: E, _selector: &'static str) -> E {
    element
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_down_wrapping_to_first() {
        assert_eq!(cycle_highlight(3, None, false), 0);
        assert_eq!(cycle_highlight(3, Some(0), false), 1);
        assert_eq!(cycle_highlight(3, Some(1), false), 2);
        assert_eq!(cycle_highlight(3, Some(2), false), 0);
    }

    #[test]
    fn cycles_up_wrapping_to_last() {
        assert_eq!(cycle_highlight(3, None, true), 2);
        assert_eq!(cycle_highlight(3, Some(0), true), 2);
        assert_eq!(cycle_highlight(3, Some(2), true), 1);
        assert_eq!(cycle_highlight(3, Some(1), true), 0);
    }

    #[test]
    fn clamps_out_of_range_and_empty() {
        assert_eq!(cycle_highlight(0, Some(0), false), 0);
        assert_eq!(cycle_highlight(3, Some(9), false), 0);
        assert_eq!(cycle_highlight(1, Some(0), true), 0);
    }
}
