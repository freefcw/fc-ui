//! Shared helpers for the tabular family (`DataTable`, `DataGrid`):
//! numeric-aware value comparison, sortable header content, and the column
//! resize grab handle.
//!
//! Public table APIs keep their own sort enums (`SortDirection`,
//! `GridSortDirection`); the helpers here use an internal
//! [`HeaderSortState`] and callers convert.

use gpui::{prelude::FluentBuilder as _, *};
use std::cmp::Ordering;
use std::rc::Rc;

pub(crate) type RowComparator<T> = Rc<dyn Fn(&T, &T) -> Ordering>;

/// Sort state used by the shared header helpers. `DataTable`'s
/// `SortDirection` and `DataGrid`'s `GridSortDirection` stay public;
/// callers convert to this type at the call site.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum HeaderSortState {
    Ascending,
    Descending,
    None,
}

/// Numeric values sort before text; numbers compare numerically and text
/// compares lexicographically. NaN sorts after other numbers (before text).
/// Equal numeric spellings retain their input order under a stable sort.
/// Classifying each value independently keeps mixed columns transitive.
pub(crate) fn compare_cell_values(a: &str, b: &str) -> Ordering {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(number_a), Ok(number_b)) => match (number_a.is_nan(), number_b.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => number_a.partial_cmp(&number_b).unwrap(),
        },
        (Ok(_), Err(_)) => Ordering::Less,
        (Err(_), Ok(_)) => Ordering::Greater,
        (Err(_), Err(_)) => a.cmp(b),
    }
}

/// Header label plus a ▲/▼ glyph when the column is actively sorted.
/// `gap` reproduces each table's historical label-to-glyph spacing
/// (`DataTable`: 8px, `DataGrid`: 6px).
pub(crate) fn sortable_header_content(
    header: gpui::SharedString,
    sort: HeaderSortState,
    gap: gpui::Pixels,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(gap)
        .child(header)
        .when(sort != HeaderSortState::None, |el| {
            el.child(div().text_size(px(10.0)).child(match sort {
                HeaderSortState::Ascending => "▲",
                HeaderSortState::Descending => "▼",
                HeaderSortState::None => "",
            }))
        })
}

/// 4px grab handle pinned to a header cell's right edge for column resizing.
/// The transparent background guarantees the strip registers pointer events
/// across its full width.
pub(crate) fn column_resize_handle(
    hover_highlight: gpui::Hsla,
    on_resize_start: impl Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Div {
    div()
        .w(px(4.0))
        .h_full()
        .absolute()
        .right(px(0.0))
        .top(px(0.0))
        .cursor(gpui::CursorStyle::ResizeLeftRight)
        .bg(transparent_black())
        .hover(move |style| style.bg(hover_highlight))
        .on_mouse_down(gpui::MouseButton::Left, on_resize_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[::core::prelude::v1::test]
    fn numeric_pairs_compare_numerically() {
        assert_eq!(compare_cell_values("10", "9"), std::cmp::Ordering::Greater);
        assert_eq!(compare_cell_values("2.5", "10.0"), std::cmp::Ordering::Less);
        assert_eq!(compare_cell_values("-1", "2"), std::cmp::Ordering::Less);
        assert_eq!(compare_cell_values("3", "3"), std::cmp::Ordering::Equal);
    }

    #[::core::prelude::v1::test]
    fn mixed_values_obey_transitivity() {
        assert!(compare_cell_values("2", "10").is_lt());
        assert!(compare_cell_values("10", "1x").is_lt());
        assert!(compare_cell_values("2", "1x").is_lt());
    }

    #[::core::prelude::v1::test]
    fn non_numeric_pairs_fall_back_to_lexicographic_order() {
        assert_eq!(compare_cell_values("a", "b"), std::cmp::Ordering::Less);
        assert_eq!(compare_cell_values("b", "a"), std::cmp::Ordering::Greater);
        // Digits sort before letters, matching the previous duplicated logic.
        assert_eq!(compare_cell_values("5", "b"), std::cmp::Ordering::Less);
        assert_eq!(
            compare_cell_values("abc", "10"),
            std::cmp::Ordering::Greater
        );
    }

    #[::core::prelude::v1::test]
    fn nan_sorts_after_numbers_and_before_text() {
        assert_eq!(compare_cell_values("NaN", "NaN"), std::cmp::Ordering::Equal);
        assert!(compare_cell_values("NaN", "inf").is_gt());
        assert!(compare_cell_values("NaN", "text").is_lt());
        assert_eq!(compare_cell_values("10", "1x"), std::cmp::Ordering::Less);
    }

    #[::core::prelude::v1::test]
    fn mixed_values_form_a_total_order() {
        let values = [
            "2", "10", "1x", "NaN", "-NaN", "inf", "-inf", "0", "-0", "2.0", "", "a",
        ];
        for a in values {
            assert_eq!(compare_cell_values(a, a), Ordering::Equal);
            for b in values {
                assert_eq!(
                    compare_cell_values(a, b),
                    compare_cell_values(b, a).reverse()
                );
                for c in values {
                    if compare_cell_values(a, b).is_le() && compare_cell_values(b, c).is_le() {
                        assert!(compare_cell_values(a, c).is_le(), "{a:?} <= {b:?} <= {c:?}");
                    }
                }
            }
        }
    }
}
