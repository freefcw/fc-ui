//! Table - Simple table component for structured data display.

use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::FluentBuilder as _, *};

#[derive(Clone)]
pub struct TableColumn {
    pub header: SharedString,
    pub width: Option<Pixels>,
}

impl TableColumn {
    pub fn new<T: Into<SharedString>>(header: T) -> Self {
        Self {
            header: header.into(),
            width: None,
        }
    }

    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }
}

#[derive(Clone)]
pub struct TableRow {
    pub cells: Vec<SharedString>,
    pub selected: bool,
}

impl TableRow {
    pub fn new(cells: Vec<SharedString>) -> Self {
        Self {
            cells,
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

#[derive(IntoElement)]
pub struct Table {
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    style: StyleRefinement,
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    pub fn columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns = columns;
        self
    }

    pub fn rows(mut self, rows: Vec<TableRow>) -> Self {
        self.rows = rows;
        self
    }
}

impl Styled for Table {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;

        let header_cells = self.columns.iter().map(|column| {
            let width = column.width.unwrap_or(px(120.0));
            div()
                .flex()
                .items_center()
                .px(px(12.0))
                .py(px(8.0))
                .w(width)
                .text_size(px(12.0))
                .font_family(theme.tokens.font_family.clone())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.tokens.muted_foreground)
                .border_b_1()
                .border_color(theme.tokens.border)
                .child(column.header.clone())
        });

        let header = div().flex().children(header_cells);

        let row_elements = self.rows.into_iter().map(|row| {
            let cell_elements = row.cells.iter().enumerate().map(|(col_index, cell)| {
                let width = self
                    .columns
                    .get(col_index)
                    .and_then(|column| column.width)
                    .unwrap_or(px(120.0));

                div()
                    .flex()
                    .items_center()
                    .px(px(12.0))
                    .py(px(8.0))
                    .w(width)
                    .text_size(px(13.0))
                    .font_family(theme.tokens.font_family.clone())
                    .text_color(theme.tokens.foreground)
                    .border_b_1()
                    .border_color(theme.tokens.border.opacity(0.5))
                    .bg(if row.selected {
                        theme.tokens.accent.opacity(0.3)
                    } else {
                        gpui::transparent_black()
                    })
                    .child(cell.clone())
            });

            div()
                .flex()
                .hover(|style| style.bg(theme.tokens.accent.opacity(0.1)))
                .children(cell_elements)
        });

        div()
            .flex()
            .flex_col()
            .border_1()
            .border_color(theme.tokens.border)
            .rounded(theme.tokens.radius_md)
            .overflow_hidden()
            .child(header)
            .children(row_elements)
            .map(|this| {
                let mut div = this;
                div.style().refine(&user_style);
                div
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::foundation::theme::{install_theme, Theme};

    struct TableTestView;

    impl Render for TableTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            Table::new()
                .columns(vec![
                    TableColumn::new("a").width(px(100.0)),
                    TableColumn::new("b"),
                ])
                .rows(vec![
                    // 行内 cells 多于 columns：修复前取宽度按索引直接数组下标 panic，
                    // 修复后兜底 120px 宽度
                    TableRow::new(vec![
                        "1".into(),
                        "2".into(),
                        "3".into(),
                        "4".into(),
                        "5".into(),
                    ]),
                    TableRow::new(vec!["only".into(), "two".into(), "extra".into()]).selected(true),
                ])
        }
    }

    #[gpui::test]
    fn rows_with_more_cells_than_columns_render_without_panic() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let mut window = app.open_window(|_, _| TableTestView);
        window.draw();
    }
}
