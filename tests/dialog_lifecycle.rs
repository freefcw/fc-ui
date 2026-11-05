//! Dismissal timers must follow the lifetime of the dialog being dismissed.

use fc_ui::overlays::dialog::Dialog;
use gpui::{
    prelude::FluentBuilder as _, AppContext as _, Context, Entity, FocusHandle,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Styled as _, TestAppContext,
    VisualTestContext, Window,
};
use std::{cell::Cell, rc::Rc, time::Duration};

struct DialogHost {
    anchor: FocusHandle,
    next_focus: FocusHandle,
    dialog: Option<Entity<Dialog>>,
}

impl Render for DialogHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .size_full()
            .child(gpui::div().id("anchor").track_focus(&self.anchor))
            .child(gpui::div().id("next-focus").track_focus(&self.next_focus))
            .when_some(self.dialog.clone(), |host, dialog| host.child(dialog))
    }
}

fn setup(cx: &mut TestAppContext) -> (Entity<DialogHost>, &mut VisualTestContext) {
    cx.update(fc_ui::init);
    cx.add_window_view(|_, cx| DialogHost {
        anchor: cx.focus_handle(),
        next_focus: cx.focus_handle(),
        dialog: None,
    })
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

fn finish_dismissal(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    draw(cx);
}

#[gpui::test]
fn escape_completes_dismissal_once_and_restores_focus(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    let close_count = Rc::new(Cell::new(0));
    let count = close_count.clone();
    let anchor = cx.update(|_, cx| host.read(cx).anchor.clone());
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);
    host.update(cx, |host, cx| {
        host.dialog = Some(cx.new(|cx| {
            Dialog::new(cx)
                .title("Original")
                .on_close(move |_, _| count.set(count.get() + 1))
        }));
        cx.notify();
    });
    draw(cx);

    cx.simulate_keystrokes("escape");
    draw(cx);
    assert_eq!(close_count.get(), 0);
    finish_dismissal(cx);

    assert_eq!(close_count.get(), 1);
    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(anchor));
    finish_dismissal(cx);
    assert_eq!(close_count.get(), 1);
}

#[gpui::test]
fn unmounted_dismissing_dialog_cancels_its_callback_and_preserves_new_focus(
    cx: &mut TestAppContext,
) {
    let (host, cx) = setup(cx);
    let close_count = Rc::new(Cell::new(0));
    let count = close_count.clone();
    let (anchor, next_focus) = cx.update(|_, cx| {
        let host = host.read(cx);
        (host.anchor.clone(), host.next_focus.clone())
    });
    cx.update(|window, _| window.focus(&anchor));
    draw(cx);
    host.update(cx, |host, cx| {
        host.dialog = Some(cx.new(|cx| {
            Dialog::new(cx)
                .title("Original")
                .on_close(move |_, _| count.set(count.get() + 1))
        }));
        cx.notify();
    });
    draw(cx);
    cx.simulate_keystrokes("escape");
    draw(cx);

    host.update(cx, |host, cx| {
        host.dialog = None;
        cx.notify();
    });
    cx.update(|window, _| window.focus(&next_focus));
    draw(cx);
    finish_dismissal(cx);

    assert_eq!(
        close_count.get(),
        0,
        "an unmounted dialog must cancel its close callback"
    );
    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(next_focus));
}

#[gpui::test]
fn replaced_dismissing_dialog_does_not_close_its_replacement(cx: &mut TestAppContext) {
    let (host, cx) = setup(cx);
    draw(cx);
    host.update(cx, |host, cx| {
        let host_weak = cx.weak_entity();
        host.dialog = Some(cx.new(|cx| {
            Dialog::new(cx).title("Original").on_close(move |_, cx| {
                let _ = host_weak.update(cx, |host, cx| {
                    host.dialog = None;
                    cx.notify();
                });
            })
        }));
        cx.notify();
    });
    draw(cx);
    cx.simulate_keystrokes("escape");
    draw(cx);

    host.update(cx, |host, cx| {
        host.dialog = Some(cx.new(|cx| Dialog::new(cx).title("Replacement")));
        cx.notify();
    });
    draw(cx);
    let replacement_focus = cx.update(|window, cx| window.focused(cx));
    finish_dismissal(cx);

    assert!(
        cx.update(|_, cx| host.read(cx).dialog.is_some()),
        "the old close callback must not remove its replacement"
    );
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        replacement_focus
    );
}
