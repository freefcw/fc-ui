//! Hover card component with popover on hover.

use gpui::{prelude::FluentBuilder as _, *};
use std::time::Duration;

use crate::capabilities::foundation::theme::use_theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HoverCardPosition {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum HoverCardAlignment {
    Start,
    #[default]
    Center,
    End,
}

pub struct HoverCardState {
    visible: bool,
    pending_open: bool,
    generation: u64,
}

impl HoverCardState {
    fn open_delayed(&mut self, delay: Duration, window: &mut Window, cx: &mut Context<Self>) {
        self.pending_open = true;
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        cx.spawn_in(window, async move |this, cx| {
            smol::Timer::after(delay).await;
            let _ = this.update(cx, |this, cx| {
                if this.pending_open && this.generation == generation && !this.visible {
                    this.visible = true;
                    this.pending_open = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn schedule_close(
        &mut self,
        delay: Option<Duration>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_open = false;
        self.generation = self.generation.wrapping_add(1);
        match delay {
            Some(delay) => {
                let generation = self.generation;
                cx.spawn_in(window, async move |this, cx| {
                    smol::Timer::after(delay).await;
                    let _ = this.update(cx, |this, cx| {
                        if !this.pending_open && this.generation == generation && this.visible {
                            this.visible = false;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            None => {
                if self.visible {
                    self.visible = false;
                    cx.notify();
                }
            }
        }
    }
}

#[derive(IntoElement)]
pub struct HoverCard {
    id: Option<ElementId>,
    trigger: AnyElement,
    content: AnyElement,
    position: HoverCardPosition,
    alignment: HoverCardAlignment,
    open_delay: Duration,
    close_delay: Duration,
    is_open: bool,
    style: StyleRefinement,
}

impl HoverCard {
    pub fn new() -> Self {
        Self {
            id: None,
            trigger: div().into_any_element(),
            content: div().into_any_element(),
            position: HoverCardPosition::default(),
            alignment: HoverCardAlignment::default(),
            open_delay: Duration::from_millis(200),
            close_delay: Duration::from_millis(300),
            is_open: false,
            style: StyleRefinement::default(),
        }
    }

    /// Sets a unique element id. Required for the internal hover tracking
    /// that honors [`HoverCard::open_delay`] and [`HoverCard::close_delay`];
    /// without an id the card only renders while `is_open` is set.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = trigger.into_any_element();
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = content.into_any_element();
        self
    }

    pub fn position(mut self, position: HoverCardPosition) -> Self {
        self.position = position;
        self
    }

    pub fn alignment(mut self, alignment: HoverCardAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn open_delay(mut self, delay: Duration) -> Self {
        self.open_delay = delay;
        self
    }

    pub fn close_delay(mut self, delay: Duration) -> Self {
        self.close_delay = delay;
        self
    }

    /// Not implemented: no arrow element is rendered and this setting has no
    /// effect.
    #[deprecated(note = "arrow rendering is not implemented; this method has no effect")]
    pub fn arrow(self, _show_arrow: bool) -> Self {
        self
    }

    pub fn is_open(mut self, is_open: bool) -> Self {
        self.is_open = is_open;
        self
    }
}

impl Default for HoverCard {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;
        let position = self.position;
        let alignment = self.alignment;
        let open_delay = self.open_delay;
        let close_delay = self.close_delay;
        let is_open = self.is_open;

        let hover_managed = !is_open && self.id.is_some();
        let wrapper_id = self.id.clone();
        let card_id = self
            .id
            .as_ref()
            .map(|id| ElementId::Name(format!("{id:?}::content").into()));
        let state = self.id.clone().map(|id| {
            window.use_keyed_state(id, cx, |_, _| HoverCardState {
                visible: false,
                pending_open: false,
                generation: 0,
            })
        });
        let visible = is_open
            || state
                .as_ref()
                .map(|state| state.read(cx).visible)
                .unwrap_or(false);

        div()
            .relative()
            .child(self.trigger)
            .when(visible, |this: Div| {
                let state_for_card = state.clone();
                this.child(
                    div()
                        .absolute()
                        .when(position == HoverCardPosition::Bottom, |this: Div| {
                            this.top_full().mt(px(8.0))
                        })
                        .when(position == HoverCardPosition::Top, |this: Div| {
                            this.bottom_full().mb(px(8.0))
                        })
                        .when(position == HoverCardPosition::Left, |this: Div| {
                            this.right_full().mr(px(8.0))
                        })
                        .when(position == HoverCardPosition::Right, |this: Div| {
                            this.left_full().ml(px(8.0))
                        })
                        .when(
                            (position == HoverCardPosition::Top
                                || position == HoverCardPosition::Bottom)
                                && alignment == HoverCardAlignment::Start,
                            |this: Div| this.left_0(),
                        )
                        .when(
                            (position == HoverCardPosition::Top
                                || position == HoverCardPosition::Bottom)
                                && alignment == HoverCardAlignment::End,
                            |this: Div| this.right_0(),
                        )
                        .child(
                            div()
                                .min_w(px(200.0))
                                .max_w(px(400.0))
                                .bg(theme.tokens.popover)
                                .border_1()
                                .border_color(theme.tokens.border)
                                .rounded(theme.tokens.radius_md)
                                .shadow(smallvec::smallvec![BoxShadow {
                                    color: hsla(0.0, 0.0, 0.0, 0.1),
                                    offset: point(px(0.0), px(4.0)),
                                    blur_radius: px(12.0),
                                    spread_radius: px(0.0),
                                    inset: false,
                                }])
                                .map(|this| {
                                    let mut div = this;
                                    div.style().refine(&user_style);
                                    div
                                })
                                .child(self.content),
                        )
                        .map(|this| {
                            if hover_managed {
                                let state = state_for_card.unwrap();
                                this.id(card_id.clone().unwrap())
                                    .on_hover(move |&hovered, window, cx| {
                                        state.update(cx, |state, cx| {
                                            if hovered {
                                                state.pending_open = false;
                                                state.generation = state.generation.wrapping_add(1);
                                            } else {
                                                let delay = (close_delay > Duration::ZERO)
                                                    .then_some(close_delay);
                                                state.schedule_close(delay, window, cx);
                                            }
                                        });
                                    })
                                    .into_any_element()
                            } else {
                                this.into_any_element()
                            }
                        }),
                )
            })
            .map(|this| {
                if hover_managed {
                    let state = state.unwrap();
                    this.id(wrapper_id.unwrap())
                        .on_hover(move |&hovered, window, cx| {
                            state.update(cx, |state, cx| {
                                if hovered {
                                    if !state.visible && !state.pending_open {
                                        state.open_delayed(open_delay, window, cx);
                                    }
                                } else {
                                    let delay =
                                        (close_delay > Duration::ZERO).then_some(close_delay);
                                    state.schedule_close(delay, window, cx);
                                }
                            });
                        })
                        .into_any_element()
                } else {
                    this.into_any_element()
                }
            })
    }
}
