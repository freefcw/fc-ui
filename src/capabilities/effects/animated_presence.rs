use gpui::*;
use std::time::{Duration, Instant};

use crate::capabilities::motion::animations::{durations, easings, lerp_pixels};

pub struct AnimatedPresenceState {
    is_visible: bool,
    is_animating_out: bool,
    version: usize,
    exit_duration: Duration,
    exit_started_at: Option<Instant>,
    exit_timer_version: usize,
}

impl AnimatedPresenceState {
    pub fn new() -> Self {
        Self {
            is_visible: false,
            is_animating_out: false,
            version: 0,
            exit_duration: durations::FAST,
            exit_started_at: None,
            exit_timer_version: 0,
        }
    }

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if visible {
            if self.is_visible && !self.is_animating_out {
                return;
            }
            self.is_visible = true;
            self.is_animating_out = false;
            self.exit_started_at = None;
            self.version += 1;
            cx.notify();
        } else {
            if !self.is_visible || self.is_animating_out {
                return;
            }
            self.is_animating_out = true;
            self.exit_started_at = Some(cx.background_executor().now());
            self.version += 1;
            cx.notify();
            self.schedule_exit(cx);
        }
    }

    fn set_exit_duration(&mut self, duration: Duration, cx: &mut Context<Self>) {
        if self.exit_duration == duration {
            return;
        }
        self.exit_duration = duration;
        if self.is_animating_out {
            self.schedule_exit(cx);
        }
    }

    fn schedule_exit(&mut self, cx: &mut Context<Self>) {
        let Some(started_at) = self.exit_started_at else {
            return;
        };
        self.exit_timer_version += 1;
        let timer_version = self.exit_timer_version;
        let version = self.version;
        let elapsed = cx
            .background_executor()
            .now()
            .saturating_duration_since(started_at);
        let remaining = self.exit_duration.saturating_sub(elapsed);
        let timer = cx.background_executor().timer(remaining);
        cx.spawn(async move |this, cx| {
            timer.await;
            _ = this.update(cx, |state, cx| {
                if state.version == version
                    && state.exit_timer_version == timer_version
                    && state.is_animating_out
                {
                    state.is_visible = false;
                    state.is_animating_out = false;
                    state.exit_started_at = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn is_animating_out(&self) -> bool {
        self.is_animating_out
    }
}

impl Default for AnimatedPresenceState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(IntoElement)]
pub struct AnimatedPresence {
    base: Div,
    id: ElementId,
    state: Entity<AnimatedPresenceState>,
    show: Option<bool>,
    enter_duration: Duration,
    exit_duration: Duration,
}

impl AnimatedPresence {
    pub fn new(id: impl Into<ElementId>, state: Entity<AnimatedPresenceState>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            state,
            show: None,
            enter_duration: durations::NORMAL,
            exit_duration: durations::FAST,
        }
    }

    pub fn show(mut self, visible: bool) -> Self {
        self.show = Some(visible);
        self
    }

    pub fn enter_duration(mut self, duration: Duration) -> Self {
        self.enter_duration = duration;
        self
    }

    pub fn exit_duration(mut self, duration: Duration) -> Self {
        self.exit_duration = duration;
        self
    }
}

impl RenderOnce for AnimatedPresence {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // State owns the phase and its deadline for both imperative and
        // declarative visibility changes. A redraw does not restart a timer.
        if self.state.read(cx).exit_duration != self.exit_duration {
            self.state.update(cx, |state, cx| {
                state.set_exit_duration(self.exit_duration, cx)
            });
        }
        if let Some(desired) = self.show {
            let state = self.state.read(cx);
            let needs_show = desired && (!state.is_visible || state.is_animating_out);
            let needs_hide = !desired && state.is_visible && !state.is_animating_out;

            if needs_show || needs_hide {
                self.state.update(cx, |state, cx| {
                    state.set_visible(desired, cx);
                });
            }
        }

        let state = self.state.read(cx);
        let is_visible = state.is_visible;
        let is_animating_out = state.is_animating_out;
        let version = state.version;

        if !is_visible && !is_animating_out {
            return div().into_any_element();
        }

        let enter_dur = self.enter_duration;
        let exit_dur = state.exit_duration;
        let slide_offset = px(8.0);

        if is_animating_out {
            let elapsed = state
                .exit_started_at
                .map(|started| {
                    cx.background_executor()
                        .now()
                        .saturating_duration_since(started)
                })
                .unwrap_or_default();
            let progress = if exit_dur.is_zero() {
                1.
            } else {
                (elapsed.as_secs_f32() / exit_dur.as_secs_f32()).clamp(0., 1.)
            };
            if progress < 1. {
                window.request_animation_frame();
            }
            let delta = easings::ease_in_cubic(progress);
            self.base
                .opacity(1.0 - delta)
                .mt(lerp_pixels(px(0.0), slide_offset, delta))
                .into_any_element()
        } else {
            self.base
                .with_animation(
                    ElementId::Name(format!("{}-enter-{}", self.id, version).into()),
                    Animation::new(enter_dur).with_easing(easings::ease_out_cubic),
                    move |el, delta| {
                        el.opacity(delta)
                            .mt(lerp_pixels(slide_offset, px(0.0), delta))
                    },
                )
                .into_any_element()
        }
    }
}

impl Styled for AnimatedPresence {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for AnimatedPresence {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for AnimatedPresence {}

impl ParentElement for AnimatedPresence {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements)
    }
}
