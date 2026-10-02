//! Popover component with anchored positioning.

use gpui::{prelude::FluentBuilder as _, *};
use std::time::Duration;
use std::{cell::RefCell, rc::Rc};

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::motion::animations::easings;

const POPOVER_MARGIN: Pixels = px(8.0);
const CONTEXT: &str = "Popover";

actions!(popover, [ClosePopover]);

pub fn init(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "popover") {
        return;
    }
    cx.bind_keys([KeyBinding::new("escape", ClosePopover, Some(CONTEXT))]);
}

pub struct PopoverContent {
    focus_handle: FocusHandle,
    content: Rc<dyn Fn(&mut Window, &mut Context<Self>) -> AnyElement>,
    dismissing: bool,
}

impl PopoverContent {
    pub fn new<F>(_window: &mut Window, cx: &mut App, content: F) -> Self
    where
        F: Fn(&mut Window, &mut Context<Self>) -> AnyElement + 'static,
    {
        Self {
            focus_handle: cx.focus_handle(),
            content: Rc::new(content),
            dismissing: false,
        }
    }
}

impl EventEmitter<DismissEvent> for PopoverContent {}

impl Focusable for PopoverContent {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for PopoverContent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);
        let dismissing = self.dismissing;

        div()
            .p(px(12.0))
            .min_w(px(200.0))
            .max_w(px(400.0))
            .bg(theme.tokens.popover)
            .text_color(theme.tokens.popover_foreground)
            .border_1()
            .border_color(theme.tokens.border)
            .rounded(theme.tokens.radius_md)
            .shadow_lg()
            .overflow_hidden()
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(|this, _: &ClosePopover, window, cx| {
                if this.dismissing {
                    return;
                }
                this.dismissing = true;
                cx.notify();

                cx.spawn_in(window, async move |entity, cx| {
                    smol::Timer::after(Duration::from_millis(120)).await;
                    let _ = entity.update(cx, |_, cx| {
                        cx.emit(DismissEvent);
                    });
                })
                .detach();
            }))
            .child((self.content)(window, cx))
            .with_animation(
                if dismissing {
                    "popover-exit"
                } else {
                    "popover-enter"
                },
                Animation::new(Duration::from_millis(if dismissing { 120 } else { 150 }))
                    .with_easing(if dismissing {
                        easings::ease_in_cubic as fn(f32) -> f32
                    } else {
                        easings::ease_out_cubic as fn(f32) -> f32
                    }),
                move |el, delta| {
                    if dismissing {
                        el.opacity(1.0 - delta).mt(px(4.0 * delta))
                    } else {
                        el.opacity(delta).mt(px(4.0 * (1.0 - delta)))
                    }
                },
            )
    }
}

pub struct Popover {
    id: ElementId,
    anchor: Corner,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<Rc<dyn Fn(&mut Window, &mut App) -> Entity<PopoverContent> + 'static>>,
    mouse_button: MouseButton,
    style: StyleRefinement,
}

impl Popover {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            anchor: Corner::TopLeft,
            trigger: None,
            content: None,
            mouse_button: MouseButton::Left,
            style: StyleRefinement::default(),
        }
    }

    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: IntoElement + 'static,
    {
        self.trigger = Some(Box::new(move |_is_open, _, _| trigger.into_any_element()));
        self
    }

    pub fn content<F>(mut self, content: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> Entity<PopoverContent> + 'static,
    {
        self.content = Some(Rc::new(content));
        self
    }

    fn render_trigger(&mut self, open: bool, window: &mut Window, cx: &mut App) -> AnyElement {
        let Some(trigger) = self.trigger.take() else {
            return div().child("Trigger").into_any_element();
        };
        let user_style = self.style.clone();
        let trigger_element = (trigger)(open, window, cx);

        div()
            .map(|mut this| {
                this.style().refine(&user_style);
                this
            })
            .child(trigger_element)
            .into_any_element()
    }

    fn resolved_corner(&self, bounds: Bounds<Pixels>) -> Point<Pixels> {
        bounds.corner(match self.anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        })
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut PopoverElementState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<PopoverElementState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl IntoElement for Popover {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[derive(Default)]
pub struct PopoverElementState {
    trigger_layout_id: Option<LayoutId>,
    popover_element: Option<AnyElement>,
    trigger_element: Option<AnyElement>,
    content_view: Rc<RefCell<Option<Entity<PopoverContent>>>>,
    dismiss_subscription: Rc<RefCell<Option<Subscription>>>,
    trigger_bounds: Option<Bounds<Pixels>>,
}

pub struct PrepaintState {
    hitbox: Hitbox,
    trigger_bounds: Option<Bounds<Pixels>>,
}

impl Element for Popover {
    type RequestLayoutState = PopoverElementState;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style::default();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |view, element_state, window, cx| {
                let mut popover_layout_id = None;
                let mut popover_element = None;
                let mut is_open = false;

                if let Some(content_view) = element_state.content_view.borrow_mut().as_mut() {
                    is_open = true;

                    let mut anchored = anchored()
                        .snap_to_window_with_margin(POPOVER_MARGIN)
                        .anchor(view.anchor);

                    if let Some(trigger_bounds) = element_state.trigger_bounds {
                        anchored = anchored.position(view.resolved_corner(trigger_bounds));
                    }

                    let mut element = {
                        let content_view_mut = element_state.content_view.clone();
                        let dismiss_subscription = element_state.dismiss_subscription.clone();
                        let anchor = view.anchor;

                        deferred(
                            anchored.child(
                                div()
                                    .occlude()
                                    .map(|this| match anchor {
                                        Corner::TopLeft | Corner::TopRight => {
                                            this.mt(POPOVER_MARGIN)
                                        }
                                        Corner::BottomLeft | Corner::BottomRight => {
                                            this.mb(POPOVER_MARGIN)
                                        }
                                    })
                                    .child(content_view.clone())
                                    .on_mouse_down_out(move |_, window, _| {
                                        *content_view_mut.borrow_mut() = None;
                                        dismiss_subscription.borrow_mut().take();
                                        window.refresh();
                                    }),
                            ),
                        )
                        .with_priority(1)
                        .into_any()
                    };

                    popover_layout_id = Some(element.request_layout(window, cx));
                    popover_element = Some(element);
                }

                let mut trigger_element = view.render_trigger(is_open, window, cx);
                let trigger_layout_id = trigger_element.request_layout(window, cx);

                let layout_id = window.request_layout(
                    style,
                    Some(trigger_layout_id).into_iter().chain(popover_layout_id),
                    cx,
                );

                (
                    layout_id,
                    PopoverElementState {
                        trigger_layout_id: Some(trigger_layout_id),
                        popover_element,
                        trigger_element: Some(trigger_element),
                        content_view: element_state.content_view.clone(),
                        dismiss_subscription: element_state.dismiss_subscription.clone(),
                        trigger_bounds: element_state.trigger_bounds,
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.trigger_element {
            element.prepaint(window, cx);
        }
        if let Some(element) = &mut request_layout.popover_element {
            element.prepaint(window, cx);
        }

        let trigger_bounds = request_layout
            .trigger_layout_id
            .map(|id| window.layout_bounds(id));

        let hitbox =
            window.insert_hitbox(trigger_bounds.unwrap_or_default(), HitboxBehavior::Normal);

        PrepaintState {
            trigger_bounds,
            hitbox,
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, element_state, window, cx| {
                element_state.trigger_bounds = prepaint.trigger_bounds;

                if let Some(mut element) = request_layout.trigger_element.take() {
                    element.paint(window, cx);
                }

                if let Some(mut element) = request_layout.popover_element.take() {
                    element.paint(window, cx);
                    return;
                }

                let Some(content_build) = this.content.take() else {
                    return;
                };

                let old_content_view = element_state.content_view.clone();
                let dismiss_subscription = element_state.dismiss_subscription.clone();
                let hitbox_id = prepaint.hitbox.id;
                let mouse_button = this.mouse_button;

                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && event.button == mouse_button
                        && hitbox_id.is_hovered(window)
                    {
                        cx.stop_propagation();
                        window.prevent_default();

                        let new_content_view = (content_build)(window, cx);
                        let old_content_view1 = old_content_view.clone();
                        let dismiss_subscription1 = dismiss_subscription.clone();

                        let previous_focus_handle = window.focused(cx);

                        let subscription = window.subscribe(
                            &new_content_view,
                            cx,
                            move |modal, _: &DismissEvent, window, cx| {
                                if modal.focus_handle(cx).contains_focused(window, cx) {
                                    if let Some(previous_focus_handle) =
                                        previous_focus_handle.as_ref()
                                    {
                                        window.focus(previous_focus_handle);
                                    }
                                }
                                *old_content_view1.borrow_mut() = None;
                                dismiss_subscription1.borrow_mut().take();
                                window.refresh();
                            },
                        );

                        window.focus(&new_content_view.focus_handle(cx));
                        *old_content_view.borrow_mut() = Some(new_content_view);
                        *dismiss_subscription.borrow_mut() = Some(subscription);
                        window.refresh();
                    }
                });
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{Popover, PopoverContent};
    use crate::capabilities::controls::{
        combobox::{Combobox, ComboboxState},
        select::{Select, SelectOption},
    };
    use gpui::{
        div, point, prelude::*, px, AppContext, Context, Modifiers, MouseButton, Render,
        TestAppContext, VisualTestContext, Window,
    };

    #[derive(Clone, Copy)]
    enum NestedControl {
        Select,
        Combobox,
    }

    struct NestedPopoverView {
        control: NestedControl,
    }

    impl Render for NestedPopoverView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let control = self.control;

            div().size_full().p(px(24.0)).child(
                Popover::new("nested-control-popover")
                    .trigger(
                        div()
                            .debug_selector(|| "nested-popover-trigger".into())
                            .w(px(120.0))
                            .h(px(40.0)),
                    )
                    .content(move |window, cx| match control {
                        NestedControl::Select => {
                            let select = cx.new(|cx| {
                                Select::new(cx)
                                    .options(vec![SelectOption::new("one", "One")])
                                    .placeholder("Choose")
                            });
                            cx.new(|cx| {
                                PopoverContent::new(window, cx, move |_, _| {
                                    div().w(px(240.0)).child(select.clone()).into_any_element()
                                })
                            })
                        }
                        NestedControl::Combobox => {
                            let state = cx.new(|_| ComboboxState::new());
                            let combobox = cx.new(|cx| {
                                Combobox::new(vec!["One"], &state, cx)
                                    .render_item(|item| (*item).into())
                            });
                            cx.new(|cx| {
                                PopoverContent::new(window, cx, move |_, _| {
                                    div()
                                        .w(px(240.0))
                                        .child(combobox.clone())
                                        .into_any_element()
                                })
                            })
                        }
                    }),
            )
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| window.draw(cx).clear());
    }

    fn click(selector: &'static str, cx: &mut VisualTestContext) {
        draw(cx);
        let bounds = cx
            .debug_bounds(selector)
            .unwrap_or_else(|| panic!("missing bounds for {selector}"));
        let position = point(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        cx.simulate_mouse_move(position, None, Modifiers::none());
        cx.simulate_mouse_down(position, MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_up(position, MouseButton::Left, Modifiers::none());
        draw(cx);
    }

    fn assert_nested_dropdown_survives_redraws(
        control: NestedControl,
        trigger_selector: &'static str,
        dropdown_selector: &'static str,
        cx: &mut TestAppContext,
    ) {
        let (_, cx) = cx.add_window_view(move |_, _| NestedPopoverView { control });

        click("nested-popover-trigger", cx);
        click(trigger_selector, cx);

        for _ in 0..3 {
            draw(cx);
            let bounds = cx
                .debug_bounds(dropdown_selector)
                .unwrap_or_else(|| panic!("missing bounds for {dropdown_selector}"));
            assert!(bounds.size.width > px(0.0));
            assert!(bounds.size.height > px(0.0));
        }
    }

    #[gpui::test]
    fn select_inside_popover_survives_repeated_redraws(cx: &mut TestAppContext) {
        assert_nested_dropdown_survives_redraws(
            NestedControl::Select,
            "select-trigger",
            "select-dropdown",
            cx,
        );
    }

    #[gpui::test]
    fn combobox_inside_popover_survives_repeated_redraws(cx: &mut TestAppContext) {
        assert_nested_dropdown_survives_redraws(
            NestedControl::Combobox,
            "combobox-trigger",
            "combobox-dropdown",
            cx,
        );
    }
}
