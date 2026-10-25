use fc_ui::{
    display::data_table::{ColumnDef, DataTable},
    layout::{Align, Grid, HStack, Justify, ScrollContainer, ScrollDirection, VStack},
    overlays::dialog::{Dialog, DialogSize},
};
use gpui::{
    div, point, px, size, AppContext as _, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, ScrollHandle, Styled as _, TestAppContext, VisualTestContext, Window,
};

fn draw_view<V: Render + 'static>(
    cx: &mut TestAppContext,
    view: V,
    viewport: gpui::Size<gpui::Pixels>,
) -> &mut VisualTestContext {
    let (_, cx) = cx.add_window_view(|_, _| view);
    cx.simulate_resize(viewport);
    cx.update(|window, app| window.draw(app).clear());
    cx
}

fn bounds(cx: &mut VisualTestContext, selector: &'static str) -> gpui::Bounds<gpui::Pixels> {
    cx.debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing debug bounds for {selector}"))
}

#[gpui::test]
fn justify_evenly_distributes_free_space_between_children(cx: &mut TestAppContext) {
    struct EvenlyView;

    impl Render for EvenlyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .fill_width()
                .h(px(80.0))
                .justify(Justify::Evenly)
                .child(
                    div()
                        .debug_selector(|| "evenly-first".into())
                        .size(px(20.0)),
                )
                .child(
                    div()
                        .debug_selector(|| "evenly-second".into())
                        .size(px(20.0)),
                )
                .child(
                    div()
                        .debug_selector(|| "evenly-third".into())
                        .size(px(20.0)),
                )
        }
    }

    let cx = draw_view(cx, EvenlyView, size(px(300.0), px(80.0)));

    let first = bounds(cx, "evenly-first");
    let second = bounds(cx, "evenly-second");
    let third = bounds(cx, "evenly-third");

    assert_eq!(first.origin.x, px(60.0));
    assert_eq!(second.origin.x, px(140.0));
    assert_eq!(third.origin.x, px(220.0));
    assert_eq!(second.origin.x - first.origin.x, px(80.0));
    assert_eq!(third.origin.x - second.origin.x, px(80.0));
}

#[gpui::test]
fn grid_keeps_full_and_partial_rows_on_the_same_columns(cx: &mut TestAppContext) {
    struct GridView;

    impl Render for GridView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            Grid::new()
                .columns(3)
                .gap(px(10.0))
                .w_full()
                .child(div().debug_selector(|| "grid-a".into()).h(px(20.0)))
                .child(div().debug_selector(|| "grid-b".into()).h(px(20.0)))
                .child(div().debug_selector(|| "grid-c".into()).h(px(20.0)))
                .child(div().debug_selector(|| "grid-d".into()).h(px(20.0)))
        }
    }

    let cx = draw_view(cx, GridView, size(px(300.0), px(100.0)));

    let first = bounds(cx, "grid-a");
    let second = bounds(cx, "grid-b");
    let third = bounds(cx, "grid-c");
    let fourth = bounds(cx, "grid-d");

    assert!((f32::from(first.size.width) - f32::from(second.size.width)).abs() <= 1.0);
    assert!((f32::from(second.size.width) - f32::from(third.size.width)).abs() <= 1.0);
    assert!((f32::from(third.size.width) - f32::from(fourth.size.width)).abs() <= 1.0);
    assert_eq!(first.origin.x, fourth.origin.x);
    assert!(fourth.origin.y > first.origin.y);
}

#[gpui::test]
fn nested_stacks_preserve_padding_and_measured_bounds(cx: &mut TestAppContext) {
    struct NestedView;

    impl Render for NestedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .debug_selector(|| "nested-root".into())
                .fill()
                .padding(px(10.0))
                .child(
                    HStack::new()
                        .debug_selector(|| "nested-row".into())
                        .fill_width()
                        .h(px(40.0))
                        .align(Align::Center)
                        .child(
                            div()
                                .debug_selector(|| "nested-child".into())
                                .size(px(20.0)),
                        ),
                )
        }
    }

    let cx = draw_view(cx, NestedView, size(px(300.0), px(200.0)));

    let root = bounds(cx, "nested-root");
    let row = bounds(cx, "nested-row");
    let child = bounds(cx, "nested-child");

    assert_eq!(root.size, size(px(300.0), px(200.0)));
    assert_eq!(row.origin, point(px(10.0), px(10.0)));
    assert_eq!(row.size.width, px(280.0));
    assert_eq!(child.origin.y, px(20.0));
}

#[gpui::test]
fn scroll_container_reports_content_extent_without_phantom_range(cx: &mut TestAppContext) {
    let handle = ScrollHandle::new();
    struct ScrollView {
        handle: ScrollHandle,
    }

    impl Render for ScrollView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            ScrollContainer::new(ScrollDirection::Vertical)
                .id("layout-scroll-regression")
                .track_scroll(&self.handle)
                .no_smooth()
                .w(px(200.0))
                .h(px(100.0))
                .child(
                    div()
                        .debug_selector(|| "scroll-content-regression".into())
                        .w_full()
                        .h(px(300.0)),
                )
        }
    }

    let cx = draw_view(
        cx,
        ScrollView {
            handle: handle.clone(),
        },
        size(px(200.0), px(100.0)),
    );

    let content = bounds(cx, "scroll-content-regression");
    assert_eq!(content.size.height, px(300.0));
    assert_eq!(handle.offset().y, px(0.0));
    assert_eq!(handle.max_offset().height, px(200.0));
}

struct DialogLayoutView {
    dialog: Entity<Dialog>,
}

impl Render for DialogLayoutView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::rgb(0x20242b))
            .child(self.dialog.clone())
    }
}

#[gpui::test]
fn dialog_content_stays_inside_the_viewport_after_layout(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| DialogLayoutView {
        dialog: cx.new(|cx| {
            Dialog::new(cx)
                .size(DialogSize::Md)
                .title("Layout regression")
                .description("The dialog should remain centered and measurable.")
                .child(|_, _| {
                    div()
                        .debug_selector(|| "dialog-layout-body".into())
                        .h(px(60.0))
                })
        }),
    });

    cx.update(|window, cx| window.draw(cx).clear());

    let viewport = cx.update(|window, _| window.viewport_size());
    let content = bounds(cx, "dialog-content");
    let body = bounds(cx, "dialog-layout-body");

    assert_eq!(content.size.width, px(500.0));
    assert!(content.origin.x >= px(0.0));
    assert!(content.origin.y >= px(0.0));
    assert!(content.origin.x + content.size.width <= viewport.width);
    assert!(content.origin.y + content.size.height <= viewport.height);
    assert!(body.size.height >= px(60.0));
}

struct DataTableLayoutView {
    table: Entity<DataTable<String>>,
}

impl Render for DataTableLayoutView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.table.clone())
    }
}

#[gpui::test]
fn data_table_keeps_header_body_and_viewport_geometry_aligned(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    let (_, cx) = cx.add_window_view(|_, cx| {
        let columns = vec![
            ColumnDef::new("name", "Name", |row: &String| row.clone().into()).width(px(180.0)),
        ];
        let table = cx.new(|cx| {
            DataTable::new(
                vec!["Alpha".to_string(), "Beta".to_string(), "Gamma".to_string()],
                columns,
                cx,
            )
        });
        DataTableLayoutView { table }
    });

    cx.update(|window, cx| window.draw(cx).clear());

    let root = bounds(cx, "data-table-root");
    let content = bounds(cx, "data-table-content");
    let body = bounds(cx, "data-table-body-container");

    assert!((f32::from(root.size.width) - f32::from(content.size.width)).abs() <= 2.0);
    assert!(content.size.width >= body.size.width);
    assert!(body.size.width > px(0.0));
    assert_eq!(body.size.height, px(600.0));
    assert!(root.size.height >= body.size.height);
}
