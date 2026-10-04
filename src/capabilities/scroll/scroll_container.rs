//! Scroll containers, lists, and physics-backed scrolling.

use crate::capabilities::layout::{Align, VStack};
use crate::capabilities::motion::animations::{easings, lerp_f32};
use crate::capabilities::motion::scroll_physics::ScrollPhysics;
use crate::capabilities::scroll::scrollbar::{Scrollbar, ScrollbarAxis, ScrollbarState};
use gpui::*;
use std::cell::RefCell;
use std::panic::Location;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;

static SCROLL_CONTAINER_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

const PIXELS_PER_LINE: f32 = 20.0;

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
        event: &ScrollWheelEvent,
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

        let dt = 1.0 / 60.0;
        let active;

        let target_result = if let Some(ref mut target) = inner.scroll_target {
            target.progress = (target.progress + dt * 3.33).min(1.0);
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
}

impl ScrollContainer {
    #[track_caller]
    pub fn new(direction: ScrollDirection) -> Self {
        let location = Location::caller();
        let auto_id = ElementId::Name(
            format!(
                "scroll-container:{}:{}:{}:{}",
                location.file(),
                location.line(),
                location.column(),
                SCROLL_CONTAINER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            )
            .into(),
        );

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

    pub fn with_physics(mut self, state: &PhysicsScrollState) -> Self {
        self.physics_state = Some(state.clone());
        self
    }

    pub fn momentum(mut self, enabled: bool) -> Self {
        let state = self
            .physics_state
            .get_or_insert_with(PhysicsScrollState::new);
        {
            let mut inner = state.inner.borrow_mut();
            inner.physics_y = inner.physics_y.clone().momentum(enabled);
            inner.physics_x = inner.physics_x.clone().momentum(enabled);
        }
        self
    }

    pub fn overscroll_bounce(mut self, enabled: bool) -> Self {
        let state = self
            .physics_state
            .get_or_insert_with(PhysicsScrollState::new);
        {
            let mut inner = state.inner.borrow_mut();
            inner.physics_y = inner.physics_y.clone().overscroll(enabled);
            inner.physics_x = inner.physics_x.clone().overscroll(enabled);
        }
        self
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

impl IntoElement for ScrollContainer {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let id_to_use = self.custom_id.clone().unwrap_or(self.auto_id.clone());
        let handle = self.scroll_handle.clone().unwrap_or_default();

        if !self.show_scrollbar {
            let mut scrollable = self.base.id(id_to_use);

            if let Some(handle) = &self.scroll_handle {
                scrollable = scrollable.track_scroll(handle);
            }

            scrollable = match self.direction {
                ScrollDirection::Vertical => scrollable.overflow_y_scroll(),
                ScrollDirection::Horizontal => scrollable.overflow_x_scroll(),
                ScrollDirection::Both => scrollable.overflow_scroll(),
            };

            // With smooth scrolling disabled, GPUI's built-in wheel handling
            // applies the delta directly, skipping the physics simulation.
            if self.smooth_scroll {
                if let Some(ref physics) = self.physics_state {
                    let physics_c = physics.clone();
                    let handle_c = handle.clone();
                    let dir = self.direction;
                    scrollable = scrollable.on_scroll_wheel(move |event, window, _cx| {
                        physics_c.handle_scroll_event(&handle_c, dir, event, window);
                    });
                }
            }

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

            // With smooth scrolling disabled, GPUI's built-in wheel handling
            // applies the delta directly, skipping the physics simulation.
            if self.smooth_scroll {
                if let Some(ref physics) = self.physics_state {
                    let physics_c = physics.clone();
                    let handle_c = handle.clone();
                    let dir = self.direction;
                    scrollable = scrollable.on_scroll_wheel(move |event, window, _cx| {
                        physics_c.handle_scroll_event(&handle_c, dir, event, window);
                    });
                }
            }

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
    fn default() -> Self {
        Self::new()
    }
}

impl ScrollList {
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
        let id = SharedString::from(format!(
            "scroll-list-{}",
            SCROLL_CONTAINER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        self.scroll_container = ScrollContainer::vertical().id(id);
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
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        let mut scroll_container = self.scroll_container;
        scroll_container = scroll_container.child(self.stack);
        scroll_container.into_element()
    }
}
