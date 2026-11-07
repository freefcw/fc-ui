use std::{cell::Cell, rc::Rc};

use fc_ui::{
    layout::{PhysicsScrollState, ScrollContainer},
    virtual_list::{
        hlist_uniform, hlist_variable, vlist_uniform, vlist_variable, ItemExtentProvider,
    },
};
use gpui::{
    div, point, px, Context, IntoElement, ParentElement as _, Pixels, Render, ScrollDelta,
    ScrollEvent, ScrollHandle, Styled as _, TestAppContext, VisualTestContext, Window,
};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

fn wheel(cx: &mut VisualTestContext, horizontal: bool, delta: f32) {
    cx.simulate_event(ScrollEvent {
        position: point(px(50.), px(50.)),
        delta: ScrollDelta::Pixels(if horizontal {
            point(px(delta), px(0.))
        } else {
            point(px(0.), px(delta))
        }),
        ..Default::default()
    });
}

struct FixedExtents;
impl ItemExtentProvider for FixedExtents {
    fn extent(&self, _: usize) -> Pixels {
        px(20.)
    }
}

struct ListHost {
    horizontal: bool,
    variable: bool,
    first: Rc<Cell<usize>>,
    physics: Option<PhysicsScrollState>,
    handle: Option<ScrollHandle>,
    reversed: bool,
}

impl Render for ListHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let first = self.first.clone();
        let renderer = move |range: std::ops::Range<usize>, _: &mut Window, _: &mut gpui::App| {
            first.set(range.start);
            range.map(|_| div().size(px(20.))).collect::<Vec<_>>()
        };
        let list = if self.variable {
            let mut list = if self.horizontal {
                hlist_variable("default-variable", 100, FixedExtents, renderer)
            } else {
                vlist_variable("default-variable", 100, FixedExtents, renderer)
            }
            .overscan(0);
            if let Some(physics) = &self.physics {
                list = list.with_physics(physics);
            }
            if let Some(handle) = &self.handle {
                list = list.track_scroll(handle);
            }
            list.into_any_element()
        } else {
            let mut list = if self.horizontal {
                hlist_uniform("default-uniform", 100, px(20.), renderer)
            } else {
                vlist_uniform("default-uniform", 100, px(20.), renderer)
            }
            .overscan(0);
            if self.reversed {
                if let Some(physics) = &self.physics {
                    list = list.with_physics(physics).with_physics(physics);
                }
            }
            if let Some(handle) = &self.handle {
                list = list.track_scroll(handle);
            }
            if !self.reversed {
                if let Some(physics) = &self.physics {
                    list = list.with_physics(physics);
                }
            }
            list.into_any_element()
        };
        div().size(px(100.)).child(list)
    }
}

fn assert_default_scroll(cx: &mut TestAppContext, horizontal: bool, variable: bool) {
    let first = Rc::new(Cell::new(usize::MAX));
    let (host, cx) = cx.add_window_view(|_, _| ListHost {
        horizontal,
        variable,
        first: first.clone(),
        physics: None,
        handle: None,
        reversed: false,
    });
    draw(cx);
    wheel(cx, horizontal, -200.);
    draw(cx);
    assert_eq!(
        first.get(),
        10,
        "wheel input must update the rendered range"
    );
    host.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert_eq!(
        first.get(),
        10,
        "rebuilding the element must preserve its offset"
    );
    wheel(cx, horizontal, -40.);
    draw(cx);
    assert_eq!(first.get(), 12);
}

#[gpui::test]
fn default_uniform_vertical_preserves_scroll(cx: &mut TestAppContext) {
    assert_default_scroll(cx, false, false);
}
#[gpui::test]
fn default_uniform_horizontal_preserves_scroll(cx: &mut TestAppContext) {
    assert_default_scroll(cx, true, false);
}
#[gpui::test]
fn default_variable_vertical_preserves_scroll(cx: &mut TestAppContext) {
    assert_default_scroll(cx, false, true);
}
#[gpui::test]
fn default_variable_horizontal_preserves_scroll(cx: &mut TestAppContext) {
    assert_default_scroll(cx, true, true);
}

#[gpui::test]
fn physics_preserves_the_default_list_offset_on_redraw(cx: &mut TestAppContext) {
    let physics = PhysicsScrollState::new().overscroll(false);
    let first = Rc::new(Cell::new(0));
    let (host, cx) = cx.add_window_view(|_, _| ListHost {
        horizontal: false,
        variable: false,
        first: first.clone(),
        physics: Some(physics),
        handle: None,
        reversed: false,
    });
    draw(cx);
    wheel(cx, false, -200.);
    draw(cx);
    assert_eq!(first.get(), 10);
    for _ in 0..4 {
        host.update(cx, |_, cx| cx.notify());
        draw(cx);
    }
    assert_eq!(first.get(), 10);
}

struct ContainerHost {
    top: Rc<Cell<Pixels>>,
    physics: PhysicsScrollState,
}
impl Render for ContainerHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let top = self.top.clone();
        ScrollContainer::vertical()
            .with_physics(&self.physics)
            .size(px(100.))
            .child(
                gpui::canvas(
                    move |bounds, _, _| {
                        top.set(bounds.origin.y);
                    },
                    |_, _, _, _| {},
                )
                .w(px(100.))
                .h(px(2000.))
                .flex_shrink_0(),
            )
    }
}

#[gpui::test]
fn default_container_preserves_its_offset_across_rebuilds(cx: &mut TestAppContext) {
    let top = Rc::new(Cell::new(px(0.)));
    let (host, cx) = cx.add_window_view(|_, _| ContainerHost {
        top: top.clone(),
        physics: PhysicsScrollState::new().overscroll(false),
    });
    draw(cx);
    wheel(cx, false, -40.);
    draw(cx);
    let after_wheel = top.get();
    assert_eq!(after_wheel, px(-40.));
    for _ in 0..4 {
        host.update(cx, |_, cx| cx.notify());
        draw(cx);
    }
    assert_eq!(top.get(), after_wheel);
}
