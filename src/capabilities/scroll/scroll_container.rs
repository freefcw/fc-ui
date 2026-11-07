//! Scroll containers, lists, and physics-backed scrolling.

use crate::capabilities::layout::{Align, VStack};
use crate::capabilities::motion::animations::{easings, lerp_f32};
use crate::capabilities::scroll::physics::ScrollPhysics;
use crate::capabilities::scroll::scrollbar::{Scrollbar, ScrollbarAxis, ScrollbarState};
use gpui::*;
use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;

/// Attach the physics-driven wheel handler to a scrollable element.
///
/// With smooth scrolling disabled, GPUI's built-in wheel handling applies
/// the delta directly, skipping the physics simulation; with no physics state
/// configured there is nothing to drive.
fn apply_scroll_behavior(
    mut scrollable: Stateful<Div>,
    smooth_scroll: bool,
    physics_state: Option<&PhysicsScrollState>,
    handle: &ScrollHandle,
    direction: ScrollDirection,
) -> Stateful<Div> {
    if smooth_scroll {
        if let Some(physics) = physics_state {
            let physics_c = physics.clone();
            let handle_c = handle.clone();
            let dir = direction;
            scrollable = scrollable.on_scroll_wheel(move |event, window, _cx| {
                physics_c.handle_scroll_event(&handle_c, dir, event, window);
            });
        }
    }
    scrollable
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

const PIXELS_PER_LINE: f32 = 20.0;

/// Frame delta the physics simulation advances per `on_next_frame` tick.
/// GPUI's frame callback does not report real elapsed time, so `tick` is
/// driven at this nominal 60 Hz dt; `ScrollPhysics::tick` normalizes its
/// decay by `dt * 60.0` to stay frame-rate independent.
const PHYSICS_FRAME_DT: f32 = 1.0 / 60.0;

/// Scroll-to-target animation speed, in progress fraction per second:
/// `1 / 3.33 ≈ 0.3 s` per animation (≈18 frames at 60 Hz), matching the
/// original hardcoded `progress + dt * 3.33`.
const SCROLL_ANIM_SPEED: f32 = 3.33;

#[derive(Clone)]
pub struct PhysicsScrollState {
    inner: Rc<RefCell<PhysicsScrollInner>>,
}

struct PhysicsScrollInner {
    physics_y: ScrollPhysics,
    physics_x: ScrollPhysics,
    scroll_target: Option<ScrollAnimTarget>,
    animating: bool,
    generation: u64,
}

struct ScrollAnimTarget {
    start_y: f32,
    end_y: f32,
    start_x: f32,
    end_x: f32,
    progress: f32,
    animate_y: bool,
    animate_x: bool,
}

impl Default for PhysicsScrollState {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsScrollState {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(PhysicsScrollInner {
                physics_y: ScrollPhysics::new(),
                physics_x: ScrollPhysics::new(),
                scroll_target: None,
                animating: false,
                generation: 0,
            })),
        }
    }

    pub fn with_deceleration(self, deceleration: f32) -> Self {
        let mut inner = self.inner.borrow_mut();
        inner.physics_y = inner.physics_y.clone().with_deceleration(deceleration);
        inner.physics_x = inner.physics_x.clone().with_deceleration(deceleration);
        drop(inner);
        self
    }

    pub fn momentum(self, enabled: bool) -> Self {
        let mut inner = self.inner.borrow_mut();
        inner.physics_y = inner.physics_y.clone().momentum(enabled);
        inner.physics_x = inner.physics_x.clone().momentum(enabled);
        drop(inner);
        self
    }

    pub fn overscroll(self, enabled: bool) -> Self {
        let mut inner = self.inner.borrow_mut();
        inner.physics_y = inner.physics_y.clone().overscroll(enabled);
        inner.physics_x = inner.physics_x.clone().overscroll(enabled);
        drop(inner);
        self
    }

    pub fn scroll_to_y_animated(&self, target: f32, handle: &ScrollHandle, window: &Window) {
        let mut inner = self.inner.borrow_mut();
        let current = -f32::from(handle.offset().y);
        inner.physics_y.set_position(current);
        inner.physics_x.set_position(-f32::from(handle.offset().x));
        inner.physics_y.stop();
        inner.scroll_target = Some(ScrollAnimTarget {
            start_y: current,
            end_y: target,
            start_x: 0.0,
            end_x: 0.0,
            progress: 0.0,
            animate_y: true,
            animate_x: false,
        });
        inner.animating = true;
        inner.generation += 1;
        let gen = inner.generation;
        drop(inner);
        drive_physics_frame(self.clone(), handle.clone(), gen, window);
    }

    pub fn scroll_to_x_animated(&self, target: f32, handle: &ScrollHandle, window: &Window) {
        let mut inner = self.inner.borrow_mut();
        let current = -f32::from(handle.offset().x);
        inner.physics_y.set_position(-f32::from(handle.offset().y));
        inner.physics_x.set_position(current);
        inner.physics_x.stop();
        inner.scroll_target = Some(ScrollAnimTarget {
            start_y: 0.0,
            end_y: 0.0,
            start_x: current,
            end_x: target,
            progress: 0.0,
            animate_y: false,
            animate_x: true,
        });
        inner.animating = true;
        inner.generation += 1;
        let gen = inner.generation;
        drop(inner);
        drive_physics_frame(self.clone(), handle.clone(), gen, window);
    }

    pub fn handle_scroll_event(
        &self,
        handle: &ScrollHandle,
        direction: ScrollDirection,
        event: &ScrollEvent,
        window: &Window,
    ) {
        let (delta_x, delta_y) = match &event.delta {
            ScrollDelta::Lines(d) => (d.x * PIXELS_PER_LINE, d.y * PIXELS_PER_LINE),
            ScrollDelta::Pixels(d) => (f32::from(d.x), f32::from(d.y)),
        };

        let mut inner = self.inner.borrow_mut();
        inner.scroll_target = None;
        inner.generation += 1;
        let gen = inner.generation;

        let max_offset = handle.max_offset();
        inner
            .physics_y
            .set_bounds(0.0, f32::from(max_offset.height).max(0.0));
        inner
            .physics_x
            .set_bounds(0.0, f32::from(max_offset.width).max(0.0));

        match direction {
            ScrollDirection::Vertical => inner.physics_y.apply_delta(-delta_y),
            ScrollDirection::Horizontal => inner.physics_x.apply_delta(-delta_x),
            ScrollDirection::Both => {
                inner.physics_y.apply_delta(-delta_y);
                inner.physics_x.apply_delta(-delta_x);
            }
        }

        let offset = handle.offset();
        inner.physics_y.set_position(-f32::from(offset.y));
        inner.physics_x.set_position(-f32::from(offset.x));

        inner.animating = true;
        drop(inner);

        drive_physics_frame(self.clone(), handle.clone(), gen, window);
    }

    pub fn is_animating(&self) -> bool {
        self.inner.borrow().animating
    }

    pub fn stop(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.physics_y.stop();
        inner.physics_x.stop();
        inner.scroll_target = None;
        inner.animating = false;
    }
}

fn drive_physics_frame(
    state: PhysicsScrollState,
    handle: ScrollHandle,
    generation: u64,
    window: &Window,
) {
    let state_c = state.clone();
    let handle_c = handle.clone();
    window.on_next_frame(move |window, _cx| {
        let mut inner = state_c.inner.borrow_mut();
        if inner.generation != generation || !inner.animating {
            return;
        }

        let dt = PHYSICS_FRAME_DT;
        let active;

        let target_result = if let Some(ref mut target) = inner.scroll_target {
            target.progress = (target.progress + dt * SCROLL_ANIM_SPEED).min(1.0);
            let t = easings::ease_out_cubic(target.progress);
            let y_pos = if target.animate_y {
                Some(lerp_f32(target.start_y, target.end_y, t))
            } else {
                None
            };
            let x_pos = if target.animate_x {
                Some(lerp_f32(target.start_x, target.end_x, t))
            } else {
                None
            };
            let done = target.progress >= 1.0;
            Some((y_pos, x_pos, done))
        } else {
            None
        };

        if let Some((y_pos, x_pos, done)) = target_result {
            if let Some(y) = y_pos {
                inner.physics_y.set_position(y);
            }
            if let Some(x) = x_pos {
                inner.physics_x.set_position(x);
            }
            if done {
                inner.scroll_target = None;
                active = false;
            } else {
                active = true;
            }
        } else {
            active = inner.physics_y.tick(dt) | inner.physics_x.tick(dt);
        }

        let mut offset = handle_c.offset();
        let max_offset = handle_c.max_offset();
        let y_managed = inner.physics_y.is_moving()
            || inner.physics_y.is_overscrolled()
            || inner.scroll_target.as_ref().is_some_and(|t| t.animate_y);
        let x_managed = inner.physics_x.is_moving()
            || inner.physics_x.is_overscrolled()
            || inner.scroll_target.as_ref().is_some_and(|t| t.animate_x);

        // A live overshoot is the bounce animation: physics pulls it back into
        // range. Anything else must settle inside the current scroll range so
        // the handle cannot drift past it.
        if y_managed {
            let position = inner.physics_y.position();
            offset.y = if inner.physics_y.is_overscrolled() {
                px(-position)
            } else {
                px(-position).clamp(-max_offset.height, px(0.))
            };
        }
        if x_managed {
            let position = inner.physics_x.position();
            offset.x = if inner.physics_x.is_overscrolled() {
                px(-position)
            } else {
                px(-position).clamp(-max_offset.width, px(0.))
            };
        }
        handle_c.set_offset(offset);

        if active {
            drop(inner);
            window.refresh();
            drive_physics_frame(state_c.clone(), handle_c.clone(), generation, window);
        } else {
            inner.animating = false;
        }
    });
}

#[derive(Default)]
struct ContainerPhysicsState {
    implicit: Option<PhysicsScrollState>,
    active: Option<PhysicsScrollState>,
}

#[derive(IntoElement)]
pub struct ScrollContainer {
    base: Div,
    direction: ScrollDirection,
    scroll_handle: Option<ScrollHandle>,
    auto_size: bool,
    smooth_scroll: bool,
    custom_id: Option<ElementId>,
    auto_id: ElementId,
    show_scrollbar: bool,
    horizontal_top: bool,
    scrollbar_state: ScrollbarState,
    physics_state: Option<PhysicsScrollState>,
    momentum_enabled: Option<bool>,
    overscroll_enabled: Option<bool>,
}

impl ScrollContainer {
    /// Create a scroll container whose implicit state follows this call site,
    /// so its offset and physics survive view rebuilds.
    ///
    /// Containers created at the same call site under the same parent share
    /// that state: siblings built in a loop, or by one helper function that is
    /// not `#[track_caller]`, scroll together. Give each one a distinct ID:
    ///
    /// ```ignore
    /// div().children(panels.iter().map(|panel| {
    ///     ScrollContainer::vertical()
    ///         .id(("panel-scroll", panel.id))
    ///         .child(panel.content())
    /// }))
    /// ```
    #[track_caller]
    pub fn new(direction: ScrollDirection) -> Self {
        let location = Location::caller();
        let auto_id = ElementId::CodeLocation(*location);

        Self {
            base: div(),
            direction,
            scroll_handle: None,
            auto_size: false,
            smooth_scroll: true,
            custom_id: None,
            auto_id,
            show_scrollbar: false,
            horizontal_top: false,
            scrollbar_state: ScrollbarState::default(),
            physics_state: None,
            momentum_enabled: None,
            overscroll_enabled: None,
        }
    }

    #[track_caller]
    pub fn vertical() -> Self {
        Self::new(ScrollDirection::Vertical)
    }

    #[track_caller]
    pub fn horizontal() -> Self {
        Self::new(ScrollDirection::Horizontal)
    }

    #[track_caller]
    pub fn both() -> Self {
        Self::new(ScrollDirection::Both)
    }

    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(handle.clone());
        self
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.custom_id = Some(id.into());
        self
    }

    #[deprecated(note = "no-op")]
    pub fn no_auto_size(mut self) -> Self {
        self.auto_size = false;
        self
    }

    /// Smooth scrolling reduces vibrating effect during scroll by allowing concurrent scroll in both axes
    pub fn smooth(mut self) -> Self {
        self.smooth_scroll = true;
        self
    }

    pub fn no_smooth(mut self) -> Self {
        self.smooth_scroll = false;
        self
    }

    pub fn flex_grow(mut self) -> Self {
        self.base = self.base.flex_1();
        self
    }

    pub fn with_scrollbar(mut self) -> Self {
        self.show_scrollbar = true;
        self
    }

    pub fn horizontal_bar_top(mut self) -> Self {
        self.horizontal_top = true;
        self
    }

    pub fn horizontal_bar_bottom(mut self) -> Self {
        self.horizontal_top = false;
        self
    }

    /// Drive scrolling with a caller-owned physics state. `momentum` and
    /// `overscroll_bounce` apply to it during rendering regardless of order.
    pub fn with_physics(mut self, state: &PhysicsScrollState) -> Self {
        self.physics_state = Some(state.clone());
        self
    }

    pub fn momentum(mut self, enabled: bool) -> Self {
        self.momentum_enabled = Some(enabled);
        self
    }

    pub fn overscroll_bounce(mut self, enabled: bool) -> Self {
        self.overscroll_enabled = Some(enabled);
        self
    }

    fn resolve_physics_state(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<PhysicsScrollState> {
        let id = self.custom_id.clone().unwrap_or(self.auto_id.clone());
        let persisted = window.use_keyed_state(id, cx, |_, _| ContainerPhysicsState::default());
        persisted.update(cx, |state, _| {
            let next = if !self.smooth_scroll {
                None
            } else if let Some(explicit) = self.physics_state.as_ref() {
                let mut explicit = explicit.clone();
                if let Some(enabled) = self.momentum_enabled {
                    explicit = explicit.momentum(enabled);
                }
                if let Some(enabled) = self.overscroll_enabled {
                    explicit = explicit.overscroll(enabled);
                }
                Some(explicit)
            } else if self.momentum_enabled.is_some() || self.overscroll_enabled.is_some() {
                Some(
                    state
                        .implicit
                        .get_or_insert_with(PhysicsScrollState::new)
                        .clone()
                        .momentum(self.momentum_enabled.unwrap_or(true))
                        .overscroll(self.overscroll_enabled.unwrap_or(true)),
                )
            } else {
                None
            };
            if let Some(previous) = state.active.as_ref() {
                if next
                    .as_ref()
                    .is_none_or(|next| !Rc::ptr_eq(&previous.inner, &next.inner))
                {
                    previous.stop();
                }
            }
            state.active = next.clone();
            next
        })
    }
}

impl ParentElement for ScrollContainer {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl Styled for ScrollContainer {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ScrollContainer {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for ScrollContainer {}

impl RenderOnce for ScrollContainer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id_to_use = self.custom_id.clone().unwrap_or(self.auto_id.clone());
        let physics_state = self.resolve_physics_state(window, cx);
        let handle = self.scroll_handle.clone().unwrap_or_else(|| {
            window
                .use_keyed_state(id_to_use.clone(), cx, |_, _| ScrollHandle::new())
                .read(cx)
                .clone()
        });

        if !self.show_scrollbar {
            let mut scrollable = self.base.id(id_to_use).track_scroll(&handle);

            scrollable = match self.direction {
                ScrollDirection::Vertical => scrollable.overflow_y_scroll(),
                ScrollDirection::Horizontal => scrollable.overflow_x_scroll(),
                ScrollDirection::Both => scrollable.overflow_scroll(),
            };

            scrollable = apply_scroll_behavior(
                scrollable,
                self.smooth_scroll,
                physics_state.as_ref(),
                &handle,
                self.direction,
            );

            scrollable
        } else {
            let scrollbar_state = self.scrollbar_state.clone();
            scrollbar_state.init_visible();

            let axis = match self.direction {
                ScrollDirection::Vertical => ScrollbarAxis::Vertical,
                ScrollDirection::Horizontal => ScrollbarAxis::Horizontal,
                ScrollDirection::Both => ScrollbarAxis::Both,
            };
            let mut scrollbar = Scrollbar::both(&scrollbar_state, &handle).axis(axis);
            if self.horizontal_top {
                scrollbar = scrollbar.horizontal_top();
            }

            let mut scrollable = self.base.id(id_to_use.clone()).track_scroll(&handle);

            scrollable = match self.direction {
                ScrollDirection::Vertical => scrollable.overflow_y_scroll(),
                ScrollDirection::Horizontal => scrollable.overflow_x_scroll(),
                ScrollDirection::Both => scrollable.overflow_scroll(),
            };

            scrollable = scrollable.relative().size_full();

            scrollable = apply_scroll_behavior(
                scrollable,
                self.smooth_scroll,
                physics_state.as_ref(),
                &handle,
                self.direction,
            );

            let wrapper = div()
                .id(ElementId::Name(format!("{}-wrapper", id_to_use).into()))
                .relative()
                .size_full()
                .child(scrollable)
                .child(scrollbar);

            wrapper
        }
    }
}

pub struct ScrollList {
    scroll_container: ScrollContainer,
    stack: VStack,
}

impl Default for ScrollList {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollList {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            scroll_container: ScrollContainer::vertical().flex_grow(),
            stack: VStack::new(),
        }
    }

    pub fn spacing(mut self, spacing: impl Into<Pixels>) -> Self {
        self.stack = self.stack.spacing(spacing);
        self
    }

    pub fn align(mut self, align: Align) -> Self {
        self.stack = self.stack.align(align);
        self
    }

    pub fn track_scroll(mut self, handle: &ScrollHandle) -> Self {
        self.scroll_container = self.scroll_container.track_scroll(handle);
        self
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.scroll_container = self.scroll_container.id(id);
        self
    }

    pub fn no_flex_grow(mut self) -> Self {
        self.scroll_container.base = self.scroll_container.base.flex_initial();
        self
    }

    pub fn momentum(mut self, enabled: bool) -> Self {
        self.scroll_container = self.scroll_container.momentum(enabled);
        self
    }

    pub fn overscroll_bounce(mut self, enabled: bool) -> Self {
        self.scroll_container = self.scroll_container.overscroll_bounce(enabled);
        self
    }

    pub fn with_physics(mut self, state: &PhysicsScrollState) -> Self {
        self.scroll_container = self.scroll_container.with_physics(state);
        self
    }
}

impl ParentElement for ScrollList {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.stack.extend(elements);
    }
}

impl Styled for ScrollList {
    fn style(&mut self) -> &mut StyleRefinement {
        self.scroll_container.style()
    }
}

impl InteractiveElement for ScrollList {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.scroll_container.interactivity()
    }
}

impl StatefulInteractiveElement for ScrollList {}

impl IntoElement for ScrollList {
    type Element = <ScrollContainer as IntoElement>::Element;

    fn into_element(self) -> Self::Element {
        let mut scroll_container = self.scroll_container;
        scroll_container = scroll_container.child(self.stack);
        scroll_container.into_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::scroll::uniform_virtual_list::{
        hlist_uniform, hlist_variable, vlist_uniform, vlist_variable, ItemExtentProvider,
    };
    use gpui::{TestAppContext, VisualTestContext};
    use std::cell::Cell;

    struct FixedExtents;
    impl ItemExtentProvider for FixedExtents {
        fn extent(&self, _: usize) -> Pixels {
            px(20.)
        }
    }

    struct PhysicsHost {
        physics: PhysicsScrollState,
        handle: Option<ScrollHandle>,
        variable: bool,
    }

    struct ImplicitPhysicsHost {
        states: Rc<RefCell<Vec<PhysicsScrollState>>>,
        momentum: bool,
    }

    impl Render for ImplicitPhysicsHost {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let container = ScrollContainer::vertical()
                .id("implicit-physics-lifecycle")
                .momentum(self.momentum)
                .overscroll_bounce(false)
                .size(px(100.))
                .child(div().h(px(2000.)).w_full().flex_shrink_0());
            self.states.borrow_mut().push(
                container
                    .resolve_physics_state(window, cx)
                    .expect("momentum enables physics"),
            );
            // Exercise the production binding and wheel handler using the
            // same resolution scope in which the selected state is observed.
            container.render(window, cx).into_any_element()
        }
    }

    struct SiblingContainersHost {
        tops: [Rc<Cell<Pixels>>; 2],
    }

    impl Render for SiblingContainersHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .children(self.tops.iter().enumerate().map(|(index, top)| {
                    let top = top.clone();
                    ScrollContainer::vertical()
                        .id(("sibling-scroll", index))
                        .w(px(100.))
                        .h(px(100.))
                        .child(
                            canvas(
                                move |bounds, _, _| top.set(bounds.origin.y),
                                |_, _, _, _| {},
                            )
                            .w(px(100.))
                            .h(px(2000.))
                            .flex_shrink_0(),
                        )
                }))
        }
    }

    #[gpui::test]
    fn distinct_ids_isolate_sibling_containers_from_one_call_site(cx: &mut TestAppContext) {
        let tops = [Rc::new(Cell::new(px(0.))), Rc::new(Cell::new(px(0.)))];
        let (_, cx) = cx.add_window_view(|_, _| SiblingContainersHost { tops: tops.clone() });
        send_wheel(cx);
        cx.update(|window, cx| window.draw(cx).clear());
        assert_eq!(tops[0].get(), px(-40.));
        assert_eq!(tops[1].get(), px(0.), "the sibling keeps its own offset");
    }

    struct OrderedOptionsHost {
        physics: PhysicsScrollState,
        physics_first: bool,
    }

    impl Render for OrderedOptionsHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let container = ScrollContainer::vertical().id("ordered-physics-options");
            let container = if self.physics_first {
                container
                    .with_physics(&self.physics)
                    .momentum(false)
                    .overscroll_bounce(false)
            } else {
                container
                    .momentum(false)
                    .overscroll_bounce(false)
                    .with_physics(&self.physics)
            };
            container
                .size(px(100.))
                .child(div().h(px(2000.)).w_full().flex_shrink_0())
        }
    }

    #[gpui::test]
    fn physics_options_apply_to_an_explicit_state_in_either_order(cx: &mut TestAppContext) {
        for physics_first in [true, false] {
            let physics = PhysicsScrollState::new();
            let (_, cx) = cx.add_window_view(|_, _| OrderedOptionsHost {
                physics: physics.clone(),
                physics_first,
            });
            cx.update(|window, cx| window.draw(cx).clear());
            let mut simulation = physics.inner.borrow().physics_y.clone();
            simulation.apply_delta(40.);
            assert!(
                !simulation.tick(PHYSICS_FRAME_DT),
                "momentum(false) applies when physics_first={physics_first}"
            );
            simulation.apply_delta(-80.);
            assert!(
                !simulation.is_overscrolled(),
                "overscroll_bounce(false) applies when physics_first={physics_first}"
            );
        }
    }

    #[gpui::test]
    fn implicit_momentum_preserves_its_driver_across_rebuilds(cx: &mut TestAppContext) {
        let states = Rc::new(RefCell::new(Vec::new()));
        let (host, cx) = cx.add_window_view(|_, _| ImplicitPhysicsHost {
            states: states.clone(),
            momentum: true,
        });
        send_wheel(cx);
        let first = states.borrow().last().unwrap().clone();
        assert_eq!(
            first.inner.borrow().generation,
            1,
            "observed generations: {:?}",
            states
                .borrow()
                .iter()
                .map(|state| state.inner.borrow().generation)
                .collect::<Vec<_>>()
        );
        host.update(cx, |_, cx| cx.notify());
        send_wheel(cx);
        let second = states.borrow().last().unwrap().clone();
        assert!(
            Rc::ptr_eq(&first.inner, &second.inner),
            "rebuilding .momentum(true) retains one owner for the active driver"
        );
        assert_eq!(
            first.inner.borrow().generation,
            2,
            "the second wheel event cancels the first driver's generation"
        );
        host.update(cx, |host, cx| {
            host.momentum = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear());
        let third = states.borrow().last().unwrap().clone();
        assert!(Rc::ptr_eq(&first.inner, &third.inner));
        let mut simulation = first.inner.borrow().physics_y.clone();
        assert!(
            !simulation.tick(PHYSICS_FRAME_DT),
            "new builder options are applied to the retained simulation"
        );
    }

    struct AnimatedReflowHost {
        physics: PhysicsScrollState,
        item_count: usize,
        request: Option<usize>,
        horizontal: bool,
        variable: bool,
    }

    impl Render for AnimatedReflowHost {
        fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let renderer = |range: std::ops::Range<usize>, _: &mut Window, _: &mut App| {
                range.map(|_| div().size(px(20.))).collect::<Vec<_>>()
            };
            if self.variable {
                let list = if self.horizontal {
                    hlist_variable("animated-reflow", self.item_count, FixedExtents, renderer)
                } else {
                    vlist_variable("animated-reflow", self.item_count, FixedExtents, renderer)
                }
                .with_physics(&self.physics)
                .size(px(100.));
                if let Some(index) = self.request.take() {
                    list.scroll_to_animated(index, window);
                }
                list.into_any_element()
            } else {
                let list = if self.horizontal {
                    hlist_uniform("animated-reflow", self.item_count, px(20.), renderer)
                } else {
                    vlist_uniform("animated-reflow", self.item_count, px(20.), renderer)
                }
                .with_physics(&self.physics)
                .size(px(100.));
                if let Some(index) = self.request.take() {
                    list.scroll_to_animated(index, window);
                }
                list.into_any_element()
            }
        }
    }

    fn assert_animation_uses_current_geometry(
        cx: &mut TestAppContext,
        horizontal: bool,
        variable: bool,
    ) {
        let physics = PhysicsScrollState::new();
        let (host, cx) = cx.add_window_view(|_, _| AnimatedReflowHost {
            physics: physics.clone(),
            item_count: 20,
            request: None,
            horizontal,
            variable,
        });
        // Publish the old 300px scroll range into the default handle first.
        cx.update(|window, cx| window.draw(cx).clear());
        for (item_count, expected_target) in [(100, 1000.), (20, 300.)] {
            host.update(cx, |host, cx| {
                host.item_count = item_count;
                host.request = Some(50);
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear());
            let inner = physics.inner.borrow();
            let target = inner
                .scroll_target
                .as_ref()
                .expect("animation starts during layout");
            assert_eq!(
                if horizontal { target.end_x } else { target.end_y },
                expected_target,
                "deferred animation uses this frame's content and viewport after growing or shrinking"
            );
        }
    }

    #[gpui::test]
    fn uniform_vertical_animation_uses_current_content_geometry(cx: &mut TestAppContext) {
        assert_animation_uses_current_geometry(cx, false, false);
    }

    #[gpui::test]
    fn uniform_horizontal_animation_uses_current_content_geometry(cx: &mut TestAppContext) {
        assert_animation_uses_current_geometry(cx, true, false);
    }

    #[gpui::test]
    fn variable_vertical_animation_uses_current_content_geometry(cx: &mut TestAppContext) {
        assert_animation_uses_current_geometry(cx, false, true);
    }

    #[gpui::test]
    fn variable_horizontal_animation_uses_current_content_geometry(cx: &mut TestAppContext) {
        assert_animation_uses_current_geometry(cx, true, true);
    }

    impl Render for PhysicsHost {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            if let Some(handle) = &self.handle {
                if self.variable {
                    vlist_variable("ordered-variable", 100, FixedExtents, |range, _, _| {
                        range.map(|_| div().h(px(20.))).collect::<Vec<_>>()
                    })
                    .with_physics(&self.physics)
                    .with_physics(&self.physics)
                    .track_scroll(handle)
                    .size(px(100.))
                    .into_any_element()
                } else {
                    vlist_uniform("ordered-uniform", 100, px(20.), |range, _, _| {
                        range.map(|_| div().h(px(20.))).collect::<Vec<_>>()
                    })
                    .with_physics(&self.physics)
                    .with_physics(&self.physics)
                    .track_scroll(handle)
                    .size(px(100.))
                    .into_any_element()
                }
            } else {
                ScrollContainer::vertical()
                    .with_physics(&self.physics)
                    .size(px(100.))
                    .child(div().h(px(2000.)).w_full().flex_shrink_0())
                    .into_any_element()
            }
        }
    }

    fn send_wheel(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear());
        cx.simulate_event(ScrollEvent {
            position: point(px(50.), px(50.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(-40.))),
            ..Default::default()
        });
    }

    #[gpui::test]
    fn repeated_physics_configuration_uses_the_final_virtual_list_handle(cx: &mut TestAppContext) {
        for variable in [false, true] {
            let physics = PhysicsScrollState::new().overscroll(false);
            let handle = ScrollHandle::new();
            handle.set_offset(point(px(0.), px(-100.)));
            let (_, cx) = cx.add_window_view(|_, _| PhysicsHost {
                physics: physics.clone(),
                handle: Some(handle.clone()),
                variable,
            });
            send_wheel(cx);
            let inner = physics.inner.borrow();
            assert_eq!(
                inner.generation, 1,
                "with_physics replaces its configuration rather than adding handlers"
            );
            assert_eq!(
                inner.physics_y.position(),
                -f32::from(handle.offset().y),
                "physics reads the final explicit handle after GPUI applies the wheel delta"
            );
        }
    }

    #[gpui::test]
    fn default_container_physics_uses_the_laid_out_scroll_range(cx: &mut TestAppContext) {
        let physics = PhysicsScrollState::new().overscroll(false);
        let (_, cx) = cx.add_window_view(|_, _| PhysicsHost {
            physics: physics.clone(),
            handle: None,
            variable: false,
        });
        send_wheel(cx);
        // The test platform has no frame loop. Advance the same simulation
        // used by the scheduled callback, after real layout and wheel input.
        let mut simulation = physics.inner.borrow().physics_y.clone();
        assert!(simulation.tick(PHYSICS_FRAME_DT));
        assert!(
            simulation.position() > 0.,
            "the default handle must publish the content's nonzero scroll range"
        );
    }

    #[::core::prelude::v1::test]
    fn removing_flex_growth_preserves_the_scroll_list_configuration() {
        let handle = ScrollHandle::new();
        handle.set_offset(point(px(0.), px(-40.)));
        let physics = PhysicsScrollState::new();
        let mut list = ScrollList::new()
            .id("configured-list")
            .track_scroll(&handle)
            .with_physics(&physics)
            .p(px(12.))
            .no_flex_grow();

        assert_eq!(
            list.scroll_container.custom_id,
            Some("configured-list".into())
        );
        assert_eq!(
            list.scroll_container
                .scroll_handle
                .as_ref()
                .unwrap()
                .offset(),
            handle.offset()
        );
        assert!(Rc::ptr_eq(
            &list.scroll_container.physics_state.as_ref().unwrap().inner,
            &physics.inner
        ));
        assert_eq!(list.style().padding.top, Some(px(12.).into()));
        assert_eq!(list.style().flex_grow, Some(0.));
        assert_eq!(list.style().flex_basis, Some(Length::Auto));
    }
}
