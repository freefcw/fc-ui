//! Shared helpers for the tabular family (`DataTable`, `DataGrid`):
//! numeric-aware value comparison, sortable header content, and the column
//! resize grab handle.
//!
//! Public table APIs keep their own sort enums (`SortDirection`,
//! `GridSortDirection`); the helpers here use an internal
//! [`HeaderSortState`] and callers convert.

use gpui::{prelude::FluentBuilder as _, *};

/// Sort state used by the shared header helpers. `DataTable`'s
/// `SortDirection` and `DataGrid`'s `GridSortDirection` stay public;
/// callers convert to this type at the call site.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum HeaderSortState {
    Ascending,
    Descending,
    None,
}

/// Numeric-aware ordering for cell values: two parseable numbers compare
/// numerically, any other pair falls back to lexicographic order.
///
/// Lifted from the `parse::<f64>` comparators previously duplicated in
/// `DataTableState::sort_rows` and `DataGridState::resort_data`.
pub(crate) fn compare_cell_values(a: &str, b: &str) -> std::cmp::Ordering {
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(number_a), Ok(number_b)) => number_a
            .partial_cmp(&number_b)
            .unwrap_or(std::cmp::Ordering::Equal),
        _ => a.cmp(b),
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
    fn non_parseable_numbers_use_partial_cmp_semantics() {
        // "NaN" parses as a float; NaN comparisons fall back to Equal.
        assert_eq!(compare_cell_values("NaN", "NaN"), std::cmp::Ordering::Equal);
        // Mixed pairs cannot compare numerically and use string order.
        assert_eq!(compare_cell_values("10", "1x"), std::cmp::Ordering::Less);
    }
}
