use fc_ui::overlays::command_palette::{Command, CommandPalette};
use gpui::{
    div, AppContext as _, Context, Entity, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window,
};
use std::{cell::Cell, rc::Rc};

struct PaletteHost {
    palette: Entity<CommandPalette>,
    other_focus: FocusHandle,
}

impl Render for PaletteHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().track_focus(&self.other_focus))
            .child(self.palette.clone())
    }
}

fn setup(
    cx: &mut TestAppContext,
    commands: Vec<Command>,
    close_count: Rc<Cell<u32>>,
) -> (Entity<PaletteHost>, &mut VisualTestContext) {
    cx.update(fc_ui::init);
    let (host, cx) = cx.add_window_view(|window, cx| {
        let palette = cx.new(|cx| {
            CommandPalette::new(window, cx, commands)
                .on_close(move |_, _| close_count.set(close_count.get() + 1))
        });
        PaletteHost {
            palette,
            other_focus: cx.focus_handle(),
        }
    });
    draw(cx);
    (host, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

#[gpui::test]
fn escape_closes_once_with_the_default_search_focus(cx: &mut TestAppContext) {
    let closes = Rc::new(Cell::new(0));
    let (_, cx) = setup(cx, vec![Command::new("a", "Alpha")], closes.clone());

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    draw(cx);

    assert_eq!(closes.get(), 1);
}

#[gpui::test]
fn search_focus_supports_navigation_and_enter(cx: &mut TestAppContext) {
    let selected = Rc::new(Cell::new(0));
    let first = selected.clone();
    let second = selected.clone();
    let third = selected.clone();
    let closes = Rc::new(Cell::new(0));
    let (_, cx) = setup(
        cx,
        vec![
            Command::new("a", "Alpha").on_select(move |_, _| first.set(1)),
            Command::new("b", "Beta").on_select(move |_, _| second.set(2)),
            Command::new("c", "Gamma").on_select(move |_, _| third.set(3)),
        ],
        closes.clone(),
    );

    cx.simulate_keystrokes("down down up enter");
    cx.run_until_parked();

    assert_eq!(selected.get(), 2);
    assert_eq!(closes.get(), 1);
}

#[gpui::test]
fn typing_filters_before_enter_executes(cx: &mut TestAppContext) {
    let selected = Rc::new(Cell::new(0));
    let first = selected.clone();
    let second = selected.clone();
    let closes = Rc::new(Cell::new(0));
    let (_, cx) = setup(
        cx,
        vec![
            Command::new("a", "Alpha").on_select(move |_, _| first.set(1)),
            Command::new("b", "Beta").on_select(move |_, _| second.set(2)),
        ],
        closes.clone(),
    );

    cx.simulate_input("Beta");
    cx.run_until_parked();
    draw(cx);
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();

    assert_eq!(selected.get(), 2);
    assert_eq!(closes.get(), 1);
}

#[gpui::test]
fn ordinary_search_blur_does_not_close(cx: &mut TestAppContext) {
    let closes = Rc::new(Cell::new(0));
    let (host, cx) = setup(cx, vec![Command::new("a", "Alpha")], closes.clone());
    cx.update(|window, cx| {
        window.focus(&host.read(cx).other_focus);
    });
    cx.run_until_parked();
    draw(cx);

    assert_eq!(closes.get(), 0);

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert_eq!(
        closes.get(),
        0,
        "palette bindings must stay scoped to its focus"
    );
}
