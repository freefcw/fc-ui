//! Behavior tests for the `BottomSheet` overlay.
//!
//! Locks the accessibility alignment introduced with the shared overlay
//! foundations: Escape closes through the `BottomSheet` key context, focus is
//! captured while the sheet is mounted and restored to the previously focused
//! element on close, and backdrop dismissal respects `close_on_backdrop_click`
//! while clicks inside the panel never dismiss.

use fc_ui::prelude::BottomSheet;
use gpui::{
    prelude::FluentBuilder as _, px, Context, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, Window,
};
use std::{cell::Cell, rc::Rc, time::Duration};

/// Host that focuses an anchor element and optionally mounts a sheet.
struct SheetHost {
    anchor: FocusHandle,
    next_focus: FocusHandle,
    sheet_child_focus: FocusHandle,
    sheet_open: bool,
    disable_backdrop_close: bool,
    close_count: Rc<Cell<u32>>,
}

impl SheetHost {
    fn new(cx: &mut Context<Self>, close_count: Rc<Cell<u32>>) -> Self {
        Self {
            anchor: cx.focus_handle(),
            next_focus: cx.focus_handle(),
            sheet_child_focus: cx.focus_handle(),
            sheet_open: false,
            disable_backdrop_close: false,
            close_count,
        }
    }

    fn open(&mut self, cx: &mut Context<Self>) {
        self.sheet_open = true;
        cx.notify();
    }
}

impl Render for SheetHost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let close_count = self.close_count.clone();
        let close_on_backdrop_click = !self.disable_backdrop_close;
        let host_entity = cx.weak_entity();

        gpui::div()
            .size_full()
            .child(
                gpui::div()
                    .id("anchor")
                    .track_focus(&self.anchor)
                    .child("Anchor"),
            )
            .child(
                gpui::div()
                    .id("next-focus")
                    .track_focus(&self.next_focus)
                    .child("Next"),
            )
            .when(self.sheet_open, |host| {
                host.child(
                    BottomSheet::new()
                        .title("Sheet title")
                        .description("Sheet description")
                        .content(
                            gpui::div()
                                .id("sheet-content")
                                .track_focus(&self.sheet_child_focus)
                                .child("Sheet body"),
                        )
                        .close_on_backdrop_click(close_on_backdrop_click)
                        .on_close(move |_, cx| {
                            close_count.set(close_count.get() + 1);
                            let _ = host_entity.update(cx, |host, cx| {
                                host.sheet_open = false;
                                cx.notify();
                            });
                        }),
                )
            })
    }
}

fn setup(cx: &mut TestAppContext) -> (gpui::Entity<SheetHost>, &mut gpui::VisualTestContext) {
    cx.update(fc_ui::init);
    let close_count = Rc::new(Cell::new(0));
    let (host, cx) = cx.add_window_view(|_, cx| SheetHost::new(cx, close_count));
    (host, cx)
}

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

/// Advance the clock past the sheet's enter animation (300 ms) and redraw, so
/// hit-testing sees the panel at its settled position.
fn settle(cx: &mut gpui::VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    draw(cx);
}

fn close_count(host: &gpui::Entity<SheetHost>, cx: &mut gpui::VisualTestContext) -> u32 {
    cx.update(|_, cx| host.read(cx).close_count.get())
}

#[gpui::test]
fn escape_closes_the_sheet_and_runs_on_close_once(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    draw(cx);

    host.update(cx, |host, cx| host.open(cx));
    draw(cx);
    assert_eq!(close_count(&host, cx), 0);

    cx.simulate_keystrokes("escape");
    draw(cx);
    draw(cx);

    assert_eq!(
        close_count(&host, cx),
        1,
        "escape must close the sheet exactly once"
    );
}

#[gpui::test]
fn sheet_captures_focus_and_restores_the_previous_element_on_close(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    let anchor = cx.update(|_, cx| host.read(cx).anchor.clone());

    // Focus the anchor before the sheet exists.
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);
    assert!(cx.update(|window, _| anchor.is_focused(window)));

    // Mounting the sheet must move focus off the anchor (into the sheet).
    host.update(cx, |host, cx| host.open(cx));
    draw(cx);
    assert!(
        !cx.update(|window, _| anchor.is_focused(window)),
        "the sheet must capture focus while mounted"
    );
    let focused_while_open = cx.update(|window, cx| window.focused(cx));
    assert_ne!(focused_while_open, Some(anchor.clone()));

    // Closing via Escape must restore focus to the anchor.
    cx.simulate_keystrokes("escape");
    draw(cx);
    let focused_after_close = cx.update(|window, cx| window.focused(cx));
    assert_eq!(
        focused_after_close,
        Some(anchor),
        "closing the sheet must restore the previously focused element"
    );
}

#[gpui::test]
fn programmatic_unmount_restores_the_previous_focus_without_on_close(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    let anchor = cx.update(|_, cx| host.read(cx).anchor.clone());
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);

    host.update(cx, |host, cx| host.open(cx));
    draw(cx);
    assert!(!cx.update(|window, _| anchor.is_focused(window)));

    host.update(cx, |host, cx| {
        host.sheet_open = false;
        cx.notify();
    });
    draw(cx);
    cx.run_until_parked();
    draw(cx);

    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(anchor));
    assert_eq!(close_count(&host, cx), 0);
}

#[gpui::test]
fn programmatic_unmount_preserves_the_hosts_new_focus(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    let (anchor, next_focus, sheet_child_focus) = cx.update(|_, cx| {
        let host = host.read(cx);
        (
            host.anchor.clone(),
            host.next_focus.clone(),
            host.sheet_child_focus.clone(),
        )
    });
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);
    host.update(cx, |host, cx| host.open(cx));
    draw(cx);
    cx.update(|window, _| window.focus(&sheet_child_focus));
    draw(cx);

    host.update(cx, |host, cx| {
        host.sheet_open = false;
        cx.notify();
    });
    cx.update(|window, _| window.focus(&next_focus));
    draw(cx);
    cx.run_until_parked();
    draw(cx);

    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(next_focus));
    assert_eq!(close_count(&host, cx), 0);
}

#[gpui::test]
fn programmatic_unmount_restores_focus_from_a_sheet_child_in_an_inactive_window(
    cx: &mut TestAppContext,
) {
    let (host, cx) = setup(cx);
    cx.deactivate_window();
    assert!(!cx.update(|window, _| window.is_window_active()));
    let (anchor, sheet_child_focus) = cx.update(|_, cx| {
        let host = host.read(cx);
        (host.anchor.clone(), host.sheet_child_focus.clone())
    });
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);
    host.update(cx, |host, cx| host.open(cx));
    draw(cx);
    cx.update(|window, _| window.focus(&sheet_child_focus));
    draw(cx);

    host.update(cx, |host, cx| {
        host.sheet_open = false;
        cx.notify();
    });
    draw(cx);
    cx.run_until_parked();
    draw(cx);

    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(anchor));
    assert_eq!(close_count(&host, cx), 0);
}

#[gpui::test]
fn reopening_after_escape_captures_and_restores_focus_again(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    let anchor = cx.update(|_, cx| host.read(cx).anchor.clone());
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);

    for expected_close_count in 1..=2 {
        host.update(cx, |host, cx| host.open(cx));
        draw(cx);
        assert!(!cx.update(|window, _| anchor.is_focused(window)));

        cx.simulate_keystrokes("escape");
        draw(cx);
        cx.run_until_parked();
        draw(cx);

        assert_eq!(
            cx.update(|window, cx| window.focused(cx)),
            Some(anchor.clone())
        );
        assert_eq!(close_count(&host, cx), expected_close_count);
    }
}

#[gpui::test]
fn backdrop_click_closes_the_sheet_only_when_enabled(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    draw(cx);

    // Default: close_on_backdrop_click = true. Click near the top-left corner,
    // far above the panel (which hugs the bottom edge).
    host.update(cx, |host, cx| host.open(cx));
    settle(cx);
    cx.simulate_click(gpui::point(px(20.0), px(20.0)), gpui::Modifiers::default());
    draw(cx);
    assert_eq!(
        close_count(&host, cx),
        1,
        "a backdrop click must close the sheet by default"
    );

    // Opt-out: the same click must not close.
    host.update(cx, |host, cx| {
        host.disable_backdrop_close = true;
        host.open(cx);
    });
    settle(cx);
    cx.simulate_click(gpui::point(px(20.0), px(20.0)), gpui::Modifiers::default());
    draw(cx);
    assert_eq!(
        close_count(&host, cx),
        1,
        "close_on_backdrop_click(false) must keep the sheet open"
    );

    // Clicks inside the panel never close the sheet.
    host.update(cx, |host, cx| {
        host.disable_backdrop_close = false;
        host.open(cx);
    });
    settle(cx);
    let viewport = cx.update(|window, _| window.viewport_size());
    let panel_point = gpui::point(viewport.width / 2.0, viewport.height - px(40.0));
    cx.simulate_click(panel_point, gpui::Modifiers::default());
    draw(cx);
    assert_eq!(
        close_count(&host, cx),
        1,
        "clicking inside the panel must not dismiss the sheet"
    );
}
