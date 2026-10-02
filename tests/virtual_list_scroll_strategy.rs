use std::ops::Range;

use adabraka_ui::{
    layout::PhysicsScrollState,
    virtual_list::{hlist_uniform, vlist_uniform, vlist_variable, ItemExtentProvider},
};
use gpui::{
    div, point, px, size, AppContext, Context, Entity, IntoElement, Pixels, Render, ScrollHandle,
    ScrollStrategy, Styled, TestAppContext, Window,
};

struct FixedExtents;

impl ItemExtentProvider for FixedExtents {
    fn extent(&self, index: usize) -> Pixels {
        px(if index == 10 { 40. } else { 20. })
    }
}

struct UniformTestView {
    handle: ScrollHandle,
    item_count: usize,
    request: Option<(usize, ScrollStrategy)>,
}

impl Render for UniformTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = vlist_uniform(
            "uniform-strategy-test",
            self.item_count,
            px(20.),
            |range: Range<usize>, _, _| {
                range.map(|_| div().h(px(20.)).w_full()).collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.handle);

        if let Some((index, strategy)) = self.request.take() {
            list.scroll_to(index, strategy);
        }
        list
    }
}

struct VariableTestView {
    handle: ScrollHandle,
    item_count: usize,
    request: Option<(usize, ScrollStrategy)>,
}

impl Render for VariableTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut list = vlist_variable(
            "variable-strategy-test",
            self.item_count,
            FixedExtents,
            |range: Range<usize>, _, _| range.map(|_| div().w_full()).collect::<Vec<_>>(),
        )
        .track_scroll(&self.handle);

        if let Some((index, strategy)) = self.request.take() {
            list.scroll_to(index, strategy);
        }
        list
    }
}

struct HorizontalUniformTestView {
    handle: ScrollHandle,
    request: Option<(usize, ScrollStrategy)>,
}

impl Render for HorizontalUniformTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = hlist_uniform(
            "horizontal-uniform-strategy-test",
            20,
            px(20.),
            |range: Range<usize>, _, _| {
                range.map(|_| div().w(px(20.)).h_full()).collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.handle);

        if let Some((index, strategy)) = self.request.take() {
            list.scroll_to(index, strategy);
        }
        list
    }
}

struct PaddedUniformTestView {
    handle: ScrollHandle,
    request: Option<(usize, ScrollStrategy)>,
}

impl Render for PaddedUniformTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = vlist_uniform(
            "padded-uniform-strategy-test",
            20,
            px(20.),
            |range: Range<usize>, _, _| {
                range.map(|_| div().h(px(20.)).w_full()).collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.handle)
        .p(px(10.));
        if let Some((index, strategy)) = self.request.take() {
            list.scroll_to(index, strategy);
        }
        list
    }
}

struct AnimationOrderTestView {
    handle: ScrollHandle,
    physics: PhysicsScrollState,
    issue_commands: bool,
}

impl Render for AnimationOrderTestView {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let list = vlist_uniform("uniform-animation-order-test", 20, px(20.), |_, _, _| {
            Vec::<gpui::Div>::new()
        })
        .track_scroll(&self.handle)
        .with_physics(&self.physics);

        if self.issue_commands {
            self.issue_commands = false;
            list.scroll_to(5, ScrollStrategy::Center);
            list.scroll_to_animated(10, window);
        }
        list
    }
}

fn draw_uniform(cx: &mut TestAppContext, strategy: ScrollStrategy) -> Pixels {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<UniformTestView> = cx.update(|_, cx| {
        cx.new(|_| UniformTestView {
            handle: handle.clone(),
            item_count: 20,
            request: Some((10, strategy)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    handle.offset().y
}

fn draw_variable(cx: &mut TestAppContext, strategy: ScrollStrategy) -> Pixels {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<VariableTestView> = cx.update(|_, cx| {
        cx.new(|_| VariableTestView {
            handle: handle.clone(),
            item_count: 20,
            request: Some((10, strategy)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    handle.offset().y
}

fn draw_uniform_without_request(cx: &mut TestAppContext) -> ScrollHandle {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<UniformTestView> = cx.update(|_, cx| {
        cx.new(|_| UniformTestView {
            handle: handle.clone(),
            item_count: 20,
            request: None,
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    handle
}

fn draw_variable_without_request(cx: &mut TestAppContext) -> ScrollHandle {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<VariableTestView> = cx.update(|_, cx| {
        cx.new(|_| VariableTestView {
            handle: handle.clone(),
            item_count: 20,
            request: None,
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    handle
}

#[gpui::test]
fn uniform_strategies_align_on_first_layout(cx: &mut TestAppContext) {
    assert_eq!(draw_uniform(cx, ScrollStrategy::Top), px(-200.));
    assert_eq!(draw_uniform(cx, ScrollStrategy::Center), px(-160.));
    assert_eq!(draw_uniform(cx, ScrollStrategy::Bottom), px(-120.));
}

#[gpui::test]
fn variable_strategies_use_the_target_item_extent(cx: &mut TestAppContext) {
    assert_eq!(draw_variable(cx, ScrollStrategy::Top), px(-200.));
    assert_eq!(draw_variable(cx, ScrollStrategy::Center), px(-170.));
    assert_eq!(draw_variable(cx, ScrollStrategy::Bottom), px(-140.));
}

#[gpui::test]
fn uniform_scroll_to_updates_handle_immediately_after_layout(cx: &mut TestAppContext) {
    let handle = draw_uniform_without_request(cx);
    let list = vlist_uniform("uniform-immediate-test", 20, px(20.), |_, _, _| {
        Vec::<gpui::Div>::new()
    })
    .track_scroll(&handle);

    list.scroll_to(10, ScrollStrategy::Center);

    assert_eq!(handle.offset().y, px(-160.));
}

#[gpui::test]
fn variable_scroll_to_updates_handle_immediately_after_layout(cx: &mut TestAppContext) {
    let handle = draw_variable_without_request(cx);
    let mut list = vlist_variable("variable-immediate-test", 20, FixedExtents, |_, _, _| {
        Vec::<gpui::Div>::new()
    })
    .track_scroll(&handle);

    list.scroll_to(10, ScrollStrategy::Bottom);

    assert_eq!(handle.offset().y, px(-140.));
}

#[gpui::test]
fn animated_scroll_starts_immediately(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let physics = PhysicsScrollState::new();

    cx.update(|window, _| {
        let list = vlist_uniform("uniform-animated-test", 20, px(20.), |_, _, _| {
            Vec::<gpui::Div>::new()
        })
        .track_scroll(&handle)
        .with_physics(&physics);
        list.scroll_to_animated(10, window);
    });

    assert!(physics.is_animating());
}

#[gpui::test]
fn animated_scroll_replaces_an_earlier_pending_request(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let physics = PhysicsScrollState::new();
    let view: Entity<AnimationOrderTestView> = cx.update(|_, cx| {
        cx.new(|_| AnimationOrderTestView {
            handle: handle.clone(),
            physics,
            issue_commands: true,
        })
    });

    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });

    assert_eq!(handle.offset().y, px(0.));
}

#[gpui::test]
fn horizontal_center_updates_only_the_horizontal_offset(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<HorizontalUniformTestView> = cx.update(|_, cx| {
        cx.new(|_| HorizontalUniformTestView {
            handle: handle.clone(),
            request: Some((10, ScrollStrategy::Center)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });

    assert_eq!(handle.offset(), point(px(-160.), px(0.)));
}

#[gpui::test]
fn padding_is_included_in_center_alignment(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<PaddedUniformTestView> = cx.update(|_, cx| {
        cx.new(|_| PaddedUniformTestView {
            handle: handle.clone(),
            request: Some((10, ScrollStrategy::Center)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });

    assert_eq!(handle.offset().y, px(-170.));
}

#[gpui::test]
fn scroll_to_cancels_an_in_flight_animation(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let physics = PhysicsScrollState::new();

    cx.update(|window, _| {
        let list = vlist_uniform("uniform-cancel-animation-test", 20, px(20.), |_, _, _| {
            Vec::<gpui::Div>::new()
        })
        .track_scroll(&handle)
        .with_physics(&physics);
        list.scroll_to_animated(10, window);
        list.scroll_to(5, ScrollStrategy::Center);
    });

    assert!(!physics.is_animating());
}

#[gpui::test]
fn uniform_scroll_to_on_empty_list_is_a_noop(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<UniformTestView> = cx.update(|_, cx| {
        cx.new(|_| UniformTestView {
            handle: handle.clone(),
            item_count: 0,
            request: Some((5, ScrollStrategy::Center)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    assert_eq!(handle.offset().y, px(0.));

    // Immediate path after a layout: still a no-op.
    let list = vlist_uniform("uniform-empty-immediate-test", 0, px(20.), |_, _, _| {
        Vec::<gpui::Div>::new()
    })
    .track_scroll(&handle);
    list.scroll_to(5, ScrollStrategy::Center);
    assert_eq!(handle.offset().y, px(0.));
}

#[gpui::test]
fn variable_scroll_to_on_empty_list_is_a_noop(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<VariableTestView> = cx.update(|_, cx| {
        cx.new(|_| VariableTestView {
            handle: handle.clone(),
            item_count: 0,
            request: Some((5, ScrollStrategy::Center)),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });
    assert_eq!(handle.offset().y, px(0.));

    // Immediate path after a layout: still a no-op.
    let mut list = vlist_variable(
        "variable-empty-immediate-test",
        0,
        FixedExtents,
        |_, _, _| Vec::<gpui::Div>::new(),
    )
    .track_scroll(&handle);
    list.scroll_to(5, ScrollStrategy::Center);
    assert_eq!(handle.offset().y, px(0.));
}

#[gpui::test]
fn padded_immediate_center_uses_the_padded_viewport(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<PaddedUniformTestView> = cx.update(|_, cx| {
        cx.new(|_| PaddedUniformTestView {
            handle: handle.clone(),
            request: None,
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });

    // The immediate path must agree with the prepaint path: the viewport
    // derived from content - max_offset already excludes the padding.
    let list = vlist_uniform("padded-uniform-immediate-test", 20, px(20.), |_, _, _| {
        Vec::<gpui::Div>::new()
    })
    .track_scroll(&handle)
    .p(px(10.));
    list.scroll_to(10, ScrollStrategy::Center);

    assert_eq!(handle.offset().y, px(-170.));
}

struct FractionalPaddingTestView {
    handle: ScrollHandle,
}

impl Render for FractionalPaddingTestView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // 4 items of 23px plus py(4.25) snapped to 4px per side = 100px,
        // exactly the viewport. Unsnapped padding (8.5px) would push the
        // content 0.5px past the bounds and grow a phantom scrollbar.
        vlist_uniform(
            "fractional-padding-test",
            4,
            px(23.),
            |range: Range<usize>, _, _| {
                range.map(|_| div().h(px(23.)).w_full()).collect::<Vec<_>>()
            },
        )
        .track_scroll(&self.handle)
        .py(px(4.25))
    }
}

#[gpui::test]
fn fractional_padding_does_not_create_phantom_scroll(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let handle = ScrollHandle::new();
    let view: Entity<FractionalPaddingTestView> = cx.update(|_, cx| {
        cx.new(|_| FractionalPaddingTestView {
            handle: handle.clone(),
        })
    });
    cx.draw(point(px(0.), px(0.)), size(px(100.), px(100.)), |_, _| {
        view.into_any_element()
    });

    assert_eq!(handle.max_offset().height, px(0.));
    assert_eq!(handle.offset().y, px(0.));
}
