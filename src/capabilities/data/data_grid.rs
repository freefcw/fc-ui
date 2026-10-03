use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum CellEditor {
    /// Free-form text editing.
    Text,
    /// Numeric editing that accepts digits, one leading minus sign, and one decimal point.
    Number,
    /// Boolean editing toggled with the Space key.
    Checkbox,
    /// Reserved for a future custom editor hook; currently behaves like text editing.
    Custom,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellPosition {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GridSortDirection {
    Ascending,
    Descending,
    None,
}

pub struct GridColumnDef<T: 'static> {
    pub id: SharedString,
    pub header: SharedString,
    pub width: Pixels,
    pub min_width: Option<Pixels>,
    pub max_width: Option<Pixels>,
    pub resizable: bool,
    pub sortable: bool,
    pub editable: bool,
    pub editor: CellEditor,
    pub cell_renderer: Rc<dyn Fn(&T, usize) -> AnyElement>,
    pub value_getter: Rc<dyn Fn(&T) -> String>,
    pub value_setter: Option<Rc<dyn Fn(&mut T, &str)>>,
}

impl<T: 'static> GridColumnDef<T> {
    pub fn new<S: Into<SharedString>>(
        id: S,
        header: S,
        renderer: impl Fn(&T, usize) -> AnyElement + 'static,
        getter: impl Fn(&T) -> String + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            header: header.into(),
            width: px(150.0),
            min_width: None,
            max_width: None,
            resizable: true,
            sortable: false,
            editable: false,
            editor: CellEditor::Text,
            cell_renderer: Rc::new(renderer),
            value_getter: Rc::new(getter),
            value_setter: None,
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    pub fn editor(mut self, editor: CellEditor) -> Self {
        self.editor = editor;
        self
    }

    pub fn value_setter(mut self, setter: impl Fn(&mut T, &str) + 'static) -> Self {
        self.value_setter = Some(Rc::new(setter));
        self
    }

    fn clamp_width(&self, width: Pixels) -> Pixels {
        let min_width = self.min_width.unwrap_or(px(50.0)).max(px(0.0));
        let max_width = self.max_width.map(|max| max.max(min_width));
        max_width.map_or(width.max(min_width), |max| width.max(min_width).min(max))
    }
}

pub struct DataGridState<T: 'static> {
    data: Vec<T>,
    columns: Vec<GridColumnDef<T>>,
    editing_cell: Option<CellPosition>,
    edit_value: String,
    column_widths: HashMap<SharedString, Pixels>,
    sort_column: Option<SharedString>,
    sort_direction: GridSortDirection,
    focus_handle: Option<FocusHandle>,
    resizing_column: Option<usize>,
    resize_start_x: f32,
    resize_start_width: Pixels,
}

impl<T: 'static> DataGridState<T> {
    pub fn new(data: Vec<T>, columns: Vec<GridColumnDef<T>>) -> Self {
        let column_widths = columns
            .iter()
            .map(|col| (col.id.clone(), col.clamp_width(col.width)))
            .collect();
        Self {
            data,
            columns,
            editing_cell: None,
            edit_value: String::new(),
            column_widths,
            sort_column: None,
            sort_direction: GridSortDirection::None,
            focus_handle: None,
            resizing_column: None,
            resize_start_x: 0.0,
            resize_start_width: px(0.0),
        }
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut Vec<T> {
        &mut self.data
    }

    pub fn start_editing(&mut self, pos: CellPosition) {
        if pos.col >= self.columns.len() || pos.row >= self.data.len() {
            return;
        }
        if !self.columns[pos.col].editable {
            return;
        }
        self.edit_value = (self.columns[pos.col].value_getter)(&self.data[pos.row]);
        self.editing_cell = Some(pos);
    }

    pub fn commit_edit(&mut self) {
        if let Some(pos) = self.editing_cell.take() {
            if let Some(col) = self.columns.get(pos.col) {
                if let Some(ref setter) = col.value_setter {
                    if let Some(row) = self.data.get_mut(pos.row) {
                        setter(row, &self.edit_value);
                    }
                }
            }
            self.edit_value.clear();
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing_cell = None;
        self.edit_value.clear();
    }

    fn handle_edit_key(&mut self, key: &str, key_char: Option<&str>) {
        let Some(position) = self.editing_cell.as_ref() else {
            return;
        };
        let Some(editor) = self
            .columns
            .get(position.col)
            .map(|column| column.editor.clone())
        else {
            return;
        };

        match editor {
            CellEditor::Checkbox => {
                if key == "space" {
                    self.edit_value = if Self::is_truthy(&self.edit_value) {
                        "false"
                    } else {
                        "true"
                    }
                    .into();
                }
            }
            CellEditor::Number => {
                if key == "backspace" {
                    self.edit_value.pop();
                    return;
                }
                let Some(character) = key_char else {
                    return;
                };
                let is_digit = character.chars().all(|ch| ch.is_ascii_digit());
                let is_minus = character == "-"
                    && self.edit_value.is_empty()
                    && !self.edit_value.contains('-');
                let is_decimal = character == "." && !self.edit_value.contains('.');
                if is_digit || is_minus || is_decimal {
                    self.edit_value.push_str(character);
                }
            }
            CellEditor::Text | CellEditor::Custom => {
                if key == "backspace" {
                    self.edit_value.pop();
                } else if let Some(character) = key_char {
                    self.edit_value.push_str(character);
                }
            }
        }
    }

    fn is_truthy(value: &str) -> bool {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        )
    }

    pub fn move_edit_next(&mut self) {
        let current = match self.editing_cell.take() {
            Some(pos) => pos,
            None => return,
        };
        if let Some(col) = self.columns.get(current.col) {
            if let Some(ref setter) = col.value_setter {
                if let Some(row) = self.data.get_mut(current.row) {
                    setter(row, &self.edit_value);
                }
            }
        }
        self.edit_value.clear();

        let num_cols = self.columns.len();
        let num_rows = self.data.len();
        let mut row = current.row;
        let mut col = current.col + 1;
        while row < num_rows {
            while col < num_cols {
                if self.columns[col].editable {
                    self.start_editing(CellPosition { row, col });
                    return;
                }
                col += 1;
            }
            col = 0;
            row += 1;
        }
    }

    fn resort_data(&mut self) {
        let Some(sort_column) = self.sort_column.clone() else {
            return;
        };
        let Some(col_idx) = self
            .columns
            .iter()
            .position(|column| column.id == sort_column)
        else {
            self.sort_column = None;
            self.sort_direction = GridSortDirection::None;
            return;
        };
        if self.sort_direction == GridSortDirection::None {
            return;
        }

        let getter = self.columns[col_idx].value_getter.clone();
        let ascending = self.sort_direction == GridSortDirection::Ascending;
        self.data.sort_by(|a, b| {
            let va = getter(a);
            let vb = getter(b);
            let ordering = match (va.parse::<f64>(), vb.parse::<f64>()) {
                (Ok(number_a), Ok(number_b)) => number_a
                    .partial_cmp(&number_b)
                    .unwrap_or(std::cmp::Ordering::Equal),
                _ => va.cmp(&vb),
            };
            if ascending {
                ordering
            } else {
                ordering.reverse()
            }
        });
    }

    pub fn sort_by_column(&mut self, col_id: &str) {
        let col_id_shared: SharedString = SharedString::from(col_id.to_string());
        let Some(col_idx) = self.columns.iter().position(|c| c.id == col_id_shared) else {
            return;
        };
        if !self.columns[col_idx].sortable {
            return;
        }
        // Sorting changes the row order, so commit the active edit first. Otherwise
        // the editing position would refer to a different row after the reorder.
        if self.editing_cell.is_some() {
            self.commit_edit();
        }

        if self.sort_column.as_ref() == Some(&col_id_shared) {
            self.sort_direction = match self.sort_direction {
                GridSortDirection::Ascending => GridSortDirection::Descending,
                GridSortDirection::Descending => GridSortDirection::None,
                GridSortDirection::None => GridSortDirection::Ascending,
            };
        } else {
            self.sort_column = Some(col_id_shared.clone());
            self.sort_direction = GridSortDirection::Ascending;
        }

        if self.sort_direction == GridSortDirection::None {
            self.sort_column = None;
            return;
        }

        self.resort_data();
    }

    pub fn resize_column(&mut self, col_id: &str, width: Pixels) {
        let col_id = SharedString::from(col_id.to_string());
        let Some(column) = self.columns.iter().find(|column| column.id == col_id) else {
            return;
        };
        self.column_widths.insert(col_id, column.clamp_width(width));
    }

    pub fn set_data(&mut self, data: Vec<T>) {
        self.data = data;
        self.editing_cell = None;
        self.edit_value.clear();
        self.resort_data();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Row {
        value: i32,
    }

    fn value_column() -> GridColumnDef<Row> {
        GridColumnDef::new(
            "value",
            "Value",
            |row: &Row, _| div().child(row.value.to_string()).into_any_element(),
            |row: &Row| row.value.to_string(),
        )
        .sortable(true)
        .editable(true)
        .value_setter(|row, value| {
            row.value = value.parse().expect("test value should be numeric");
        })
    }

    #[::core::prelude::v1::test]
    fn sorting_commits_edit_before_reordering_rows() {
        let mut state = DataGridState::new(
            vec![Row { value: 2 }, Row { value: 1 }],
            vec![value_column()],
        );
        state.start_editing(CellPosition { row: 0, col: 0 });
        state.edit_value = "0".into();

        state.sort_by_column("value");

        assert!(state.editing_cell.is_none());
        assert_eq!(
            state.data.iter().map(|row| row.value).collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[::core::prelude::v1::test]
    fn replacing_data_reapplies_the_active_sort() {
        let mut state = DataGridState::new(
            vec![Row { value: 2 }, Row { value: 1 }],
            vec![value_column()],
        );
        state.sort_by_column("value");
        state.set_data(vec![Row { value: 10 }, Row { value: 3 }]);

        assert_eq!(
            state.data.iter().map(|row| row.value).collect::<Vec<_>>(),
            [3, 10]
        );
    }

    #[::core::prelude::v1::test]
    fn resize_column_respects_consistent_width_bounds() {
        let column = value_column().min_width(px(120.0)).max_width(px(80.0));
        let mut state = DataGridState::new(vec![Row { value: 1 }], vec![column]);

        state.resize_column("value", px(20.0));
        assert_eq!(state.column_widths[&SharedString::from("value")], px(120.0));

        state.resize_column("value", px(240.0));
        // An invalid max < min is normalized to the minimum rather than
        // allowing a resize operation to violate its own lower bound.
        assert_eq!(state.column_widths[&SharedString::from("value")], px(120.0));
    }

    #[::core::prelude::v1::test]
    fn number_editor_rejects_invalid_characters() {
        let column = value_column().editor(CellEditor::Number);
        let mut state = DataGridState::new(vec![Row { value: 1 }], vec![column]);
        state.start_editing(CellPosition { row: 0, col: 0 });
        state.edit_value.clear();

        state.handle_edit_key("", Some("1"));
        state.handle_edit_key("", Some("."));
        state.handle_edit_key("", Some("2"));
        state.handle_edit_key("", Some("."));
        state.handle_edit_key("", Some("x"));

        assert_eq!(state.edit_value, "1.2");
    }

    #[::core::prelude::v1::test]
    fn checkbox_editor_toggles_boolean_value_with_space() {
        let column = value_column().editor(CellEditor::Checkbox);
        let mut state = DataGridState::new(vec![Row { value: 1 }], vec![column]);
        state.start_editing(CellPosition { row: 0, col: 0 });
        state.edit_value = "false".into();

        state.handle_edit_key("space", None);
        assert_eq!(state.edit_value, "true");

        state.handle_edit_key("space", None);
        assert_eq!(state.edit_value, "false");
    }
}

pub struct DataGrid<T: 'static> {
    state: Entity<DataGridState<T>>,
    striped: bool,
    bordered: bool,
    compact: bool,
    style: StyleRefinement,
}

impl<T: 'static> DataGrid<T> {
    pub fn new(state: Entity<DataGridState<T>>) -> Self {
        Self {
            state,
            striped: false,
            bordered: true,
            compact: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }
}

impl<T: 'static> Styled for DataGrid<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

struct ColSnapshot {
    id: SharedString,
    header: SharedString,
    width: Pixels,
    resizable: bool,
    sortable: bool,
    editable: bool,
}

impl<T: 'static> RenderOnce for DataGrid<T> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let state_entity = self.state.clone();
        let striped = self.striped;
        let bordered = self.bordered;
        let compact = self.compact;
        let user_style = self.style;

        let (cell_px, cell_py) = if compact {
            (px(10.0), px(6.0))
        } else {
            (px(16.0), px(12.0))
        };

        let focus_handle = state_entity.update(cx, |s, scx| {
            if s.focus_handle.is_none() {
                s.focus_handle = Some(scx.focus_handle());
            }
            s.focus_handle.clone().unwrap()
        });

        let state = state_entity.read(cx);
        let num_rows = state.data.len();
        let num_cols = state.columns.len();
        let editing = state.editing_cell.clone();
        let edit_val = state.edit_value.clone();
        let sort_col = state.sort_column.clone();
        let sort_dir = state.sort_direction.clone();

        let col_infos: Vec<ColSnapshot> = state
            .columns
            .iter()
            .map(|c| ColSnapshot {
                id: c.id.clone(),
                header: c.header.clone(),
                width: state.column_widths.get(&c.id).copied().unwrap_or(c.width),
                resizable: c.resizable,
                sortable: c.sortable,
                editable: c.editable,
            })
            .collect();

        let mut all_cells: Vec<Vec<AnyElement>> = Vec::with_capacity(num_rows);
        for row_idx in 0..num_rows {
            let mut row_cells: Vec<AnyElement> = Vec::with_capacity(num_cols);
            for col_idx in 0..num_cols {
                let is_editing = editing
                    .as_ref()
                    .map_or(false, |p| p.row == row_idx && p.col == col_idx);
                if is_editing {
                    let editor = &state.columns[col_idx].editor;
                    let edit_display = match editor {
                        CellEditor::Checkbox => {
                            if DataGridState::<T>::is_truthy(&edit_val) {
                                "☑"
                            } else {
                                "☐"
                            }
                        }
                        CellEditor::Text | CellEditor::Number | CellEditor::Custom => "",
                    };
                    let value = if edit_display.is_empty() {
                        format!("{}|", edit_val)
                    } else {
                        edit_display.to_string()
                    };
                    row_cells.push(
                        div()
                            .flex()
                            .items_center()
                            .size_full()
                            .text_size(px(13.0))
                            .text_color(theme.tokens.foreground)
                            .child(value)
                            .into_any_element(),
                    );
                } else {
                    let content =
                        (state.columns[col_idx].cell_renderer)(&state.data[row_idx], row_idx);
                    row_cells.push(content);
                }
            }
            all_cells.push(row_cells);
        }

        let total_width: f32 = col_infos.iter().map(|c| -> f32 { c.width.into() }).sum();
        let total_width_px = px(total_width);

        let header_cells: Vec<AnyElement> = col_infos
            .iter()
            .enumerate()
            .map(|(col_idx, info)| {
                let is_sorted = sort_col.as_ref() == Some(&info.id);
                let sort_indicator = if is_sorted {
                    match sort_dir {
                        GridSortDirection::Ascending => "\u{25B2}",
                        GridSortDirection::Descending => "\u{25BC}",
                        GridSortDirection::None => "",
                    }
                } else {
                    ""
                };

                let header_content = div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(info.header.clone())
                    .when(!sort_indicator.is_empty(), |el| {
                        el.child(div().text_size(px(10.0)).child(sort_indicator))
                    });

                let base = div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .relative()
                    .w(info.width)
                    .px(cell_px)
                    .py(cell_py)
                    .text_size(px(13.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.tokens.muted_foreground)
                    .bg(theme.tokens.muted.opacity(0.5))
                    .when(bordered, |el| {
                        el.border_b_1()
                            .border_r_1()
                            .border_color(theme.tokens.border)
                    })
                    .child(header_content);

                let base = base.id(ElementId::NamedInteger("grid-hdr".into(), col_idx as u64));

                let with_sort = if info.sortable {
                    let col_id = info.id.clone();
                    let st = state_entity.clone();
                    base.cursor(CursorStyle::PointingHand)
                        .hover(|s| s.bg(theme.tokens.muted.opacity(0.7)))
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            st.update(cx, |s, scx| {
                                s.sort_by_column(&col_id);
                                scx.notify();
                            });
                        })
                } else {
                    base
                };

                let with_resize = if info.resizable {
                    let col_width = info.width;
                    let st = state_entity.clone();
                    with_sort.child(
                        div()
                            .id(ElementId::NamedInteger("grid-rsz".into(), col_idx as u64))
                            .absolute()
                            .right(px(0.0))
                            .top(px(0.0))
                            .w(px(4.0))
                            .h_full()
                            .cursor(CursorStyle::ResizeLeftRight)
                            .hover(|s| s.bg(theme.tokens.primary.opacity(0.5)))
                            .on_mouse_down(
                                MouseButton::Left,
                                move |event: &MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    st.update(cx, |s, scx| {
                                        s.resizing_column = Some(col_idx);
                                        s.resize_start_x = event.position.x.into();
                                        s.resize_start_width = col_width;
                                        scx.notify();
                                    });
                                },
                            ),
                    )
                } else {
                    with_sort
                };

                with_resize.into_any_element()
            })
            .collect();

        let header_row = div()
            .flex()
            .w(total_width_px)
            .min_w(total_width_px)
            .children(header_cells);

        let body_rows: Vec<AnyElement> = all_cells
            .into_iter()
            .enumerate()
            .map(|(row_idx, cell_contents)| {
                let row_bg = if striped && row_idx % 2 == 1 {
                    theme.tokens.muted.opacity(0.3)
                } else {
                    theme.tokens.background
                };

                let cells: Vec<AnyElement> = cell_contents
                    .into_iter()
                    .enumerate()
                    .map(|(col_idx, content)| {
                        let width = col_infos[col_idx].width;
                        let is_editing = editing
                            .as_ref()
                            .map_or(false, |p| p.row == row_idx && p.col == col_idx);
                        let is_editable = col_infos[col_idx].editable;

                        let mut cell = div()
                            .id(ElementId::NamedInteger(
                                "grid-cell".into(),
                                (row_idx * 10000 + col_idx) as u64,
                            ))
                            .flex()
                            .items_center()
                            .w(width)
                            .px(cell_px)
                            .py(cell_py)
                            .text_size(px(13.0))
                            .text_color(theme.tokens.foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .when(bordered, |el| {
                                el.border_b_1()
                                    .border_r_1()
                                    .border_color(theme.tokens.border.opacity(0.5))
                            });

                        if is_editing {
                            cell = cell
                                .bg(theme.tokens.background)
                                .border_2()
                                .border_color(theme.tokens.ring);
                        }

                        if is_editable && !is_editing {
                            let st = state_entity.clone();
                            cell = cell.cursor(CursorStyle::IBeam).on_mouse_down(
                                MouseButton::Left,
                                move |event: &MouseDownEvent, window, cx| {
                                    if event.click_count < 2 {
                                        return;
                                    }
                                    let fh = st.update(cx, |s, scx| {
                                        if s.editing_cell.is_some() {
                                            s.commit_edit();
                                        }
                                        s.start_editing(CellPosition {
                                            row: row_idx,
                                            col: col_idx,
                                        });
                                        scx.notify();
                                        s.focus_handle.clone()
                                    });
                                    if let Some(handle) = fh {
                                        window.focus(&handle);
                                    }
                                },
                            );
                        }

                        cell.child(content).into_any_element()
                    })
                    .collect();

                div()
                    .flex()
                    .w(total_width_px)
                    .min_w(total_width_px)
                    .bg(row_bg)
                    .hover(|s| s.bg(theme.tokens.accent.opacity(0.1)))
                    .children(cells)
                    .into_any_element()
            })
            .collect();

        let body = div()
            .id("data-grid-body")
            .flex_1()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .children(body_rows);

        let state_for_keys = state_entity.clone();
        let state_for_move = state_entity.clone();
        let state_for_up = state_entity.clone();

        div()
            .id("data-grid-container")
            .track_focus(&focus_handle)
            .flex()
            .flex_col()
            .w_full()
            .border_1()
            .border_color(theme.tokens.border)
            .rounded(theme.tokens.radius_lg)
            .overflow_hidden()
            .bg(theme.tokens.card)
            .shadow_sm()
            .on_key_down(move |event: &KeyDownEvent, _, cx| {
                state_for_keys.update(cx, |s, scx| {
                    if s.editing_cell.is_none() {
                        return;
                    }
                    let key = event.keystroke.key.as_str();
                    if key == "enter" {
                        s.commit_edit();
                    } else if key == "escape" {
                        s.cancel_edit();
                    } else if key == "tab" {
                        s.move_edit_next();
                    } else {
                        s.handle_edit_key(
                            key,
                            event
                                .keystroke
                                .key_char
                                .as_ref()
                                .map(|character| character.as_ref()),
                        );
                    }
                    scx.notify();
                });
            })
            .on_mouse_move(move |event: &MouseMoveEvent, _, cx| {
                state_for_move.update(cx, |s, scx| {
                    if let Some(col_idx) = s.resizing_column {
                        let current_x: f32 = event.position.x.into();
                        let delta = current_x - s.resize_start_x;
                        let start_w: f32 = s.resize_start_width.into();
                        let new_width = px((start_w + delta).max(0.0));
                        let clamped = s.columns[col_idx].clamp_width(new_width);
                        let col_id = s.columns[col_idx].id.clone();
                        s.column_widths.insert(col_id, clamped);
                        scx.notify();
                    }
                });
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                state_for_up.update(cx, |s, scx| {
                    if s.resizing_column.is_some() {
                        s.resizing_column = None;
                        scx.notify();
                    }
                });
            })
            .child(header_row)
            .child(body)
            .map(|mut el| {
                el.style().refine(&user_style);
                el
            })
    }
}
