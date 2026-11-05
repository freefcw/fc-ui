//! Shared plumbing for overlay components.
//!
//! This module centralizes the infrastructure that every overlay needs so the
//! individual components only carry their own layout and behavior:
//!
//! - [`OverlayFocusGuard`]: capture focus on open, restore it on close, and
//!   re-focus a sane target when focus is lost while mounted.
//! - [`DismissTiming`] + named constants: the single source of truth for
//!   enter/exit animation durations and dismissal (`reap`) timers, so the
//!   animation and the timer can no longer drift apart.
//! - [`dismiss_animation`] / [`fade_animation`]: animation constructors with
//!   the standard enter/exit easing pair.
//! - [`schedule_reap`]: run a callback after the exit duration elapses.
//! - [`backdrop`] / [`dismissible_backdrop`]: the dimmed layer behind modals.
//! - [`menu_panel`] / [`menu_row`]: the shared popover-panel shell and menu
//!   row renderer used by [`super::context_menu`] and [`super::popover_menu`].

use std::rc::Rc;
use std::time::Duration;

use gpui::{prelude::FluentBuilder as _, *};

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::motion::animations::{durations, easings};
use crate::capabilities::primitives::icon::Icon;

/// Captures window focus while an overlay is mounted and restores the
/// previously focused element when the overlay closes (or when focus is lost
/// while it is still mounted, e.g. the focused element is removed).
///
/// Hosts one guard per overlay entity and keep it for the entity's lifetime:
///
/// ```rust,ignore
/// struct MyOverlay {
///     focus_handle: FocusHandle,
///     focus_guard: OverlayFocusGuard,
/// }
///
/// // On every render:
/// self.focus_guard.capture_focus(window, cx, &self.focus_handle, |this| {
///     &mut this.focus_guard
/// });
/// // On close:
/// self.focus_guard.restore_focus(window, cx);
/// ```
pub(crate) struct OverlayFocusGuard {
    focused: bool,
    previous_focus: Option<FocusHandle>,
    subscription: Option<Subscription>,
}

impl Default for OverlayFocusGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayFocusGuard {
    pub(crate) fn new() -> Self {
        Self {
            focused: false,
            previous_focus: None,
            subscription: None,
        }
    }

    /// On the first call (and again after [`Self::restore_focus`]), remember
    /// the currently focused element, move focus onto `target` and subscribe
    /// to window focus-lost events so focus can be restored if the overlay
    /// loses it while mounted. Subsequent calls while captured are no-ops.
    ///
    /// `guard_in` lenses the guard out of its host entity so the focus-lost
    /// subscription can reset and restore it.
    pub(crate) fn capture_focus<T: 'static>(
        &mut self,
        window: &mut Window,
        cx: &mut Context<T>,
        target: &FocusHandle,
        guard_in: impl Fn(&mut T) -> &mut OverlayFocusGuard + 'static,
    ) {
        if self.focused {
            return;
        }
        self.previous_focus = window.focused(cx);
        window.focus(target);
        self.focused = true;
        self.subscription = Some(cx.on_focus_lost(window, move |owner, window, cx| {
            let guard = guard_in(owner);
            if guard.focused {
                guard.focused = false;
                guard.restore_focus(window, cx);
                cx.notify();
            }
        }));
    }

    /// Move focus back to the element that was focused before the overlay
    /// captured it, falling back to the window's focus-lost restore target.
    pub(crate) fn restore_focus(&mut self, window: &mut Window, cx: &App) {
        if let Some(previous_focus) = self.previous_focus.take() {
            window.focus(&previous_focus);
        } else if let Some(restore_target) = window.focus_lost_restore_target(cx) {
            window.focus(&restore_target);
        }
    }
}

/// Enter/exit durations for an overlay's dismiss animation. `exit` is also
/// the single source for dismissal timers via [`schedule_reap`], so the
/// animation and the reap timer can never disagree.
#[derive(Copy, Clone, Debug)]
pub(crate) struct DismissTiming {
    /// Entrance animation duration.
    pub enter: Duration,
    /// Exit animation duration; also drives dismissal timers.
    pub exit: Duration,
}

/// Popover enter/exit timing (150ms / 120ms).
pub(crate) const POPOVER_TIMING: DismissTiming = DismissTiming {
    enter: durations::FASTEST,
    exit: durations::EXIT_QUICK,
};

/// Modal dialog enter/exit timing (250ms / 200ms); the backdrop fade uses
/// `exit` for both directions.
pub(crate) const DIALOG_TIMING: DismissTiming = DismissTiming {
    enter: durations::EXIT_STANDARD,
    exit: durations::FAST,
};

/// Context menu enter/exit timing (120ms / 100ms).
pub(crate) const CONTEXT_MENU_TIMING: DismissTiming = DismissTiming {
    enter: durations::EXIT_QUICK,
    exit: durations::ULTRA_FAST,
};

/// Toast enter/exit timing (300ms / 250ms).
pub(crate) const TOAST_TIMING: DismissTiming = DismissTiming {
    enter: durations::NORMAL,
    exit: durations::EXIT_STANDARD,
};

impl DismissTiming {
    /// The animation duration for the current phase.
    pub(crate) fn animation_duration(&self, dismissing: bool) -> Duration {
        if dismissing {
            self.exit
        } else {
            self.enter
        }
    }
}

/// Build an animation with the given duration and the standard overlay
/// easing pair: `ease_out_cubic` on entrance, `ease_in_cubic` on exit.
pub(crate) fn fade_animation(duration: Duration, dismissing: bool) -> Animation {
    let easing = if dismissing {
        easings::ease_in_cubic as fn(f32) -> f32
    } else {
        easings::ease_out_cubic as fn(f32) -> f32
    };
    Animation::new(duration).with_easing(easing)
}

/// Build the enter/exit animation for an overlay from its [`DismissTiming`].
pub(crate) fn dismiss_animation(timing: DismissTiming, dismissing: bool) -> Animation {
    fade_animation(timing.animation_duration(dismissing), dismissing)
}

/// Wait for `timing.exit` to elapse (matching the exit animation) and then
/// run `on_done` against `entity` with its window and context. Used to reap
/// dismissed overlays after their exit animation finishes. The timer holds
/// only a weak reference so releasing an overlay cancels its callback.
pub(crate) fn schedule_reap<E: 'static>(
    entity: &Entity<E>,
    window: &Window,
    cx: &mut App,
    timing: DismissTiming,
    on_done: impl FnOnce(&mut E, &mut Window, &mut Context<E>) + 'static,
) {
    let entity = entity.downgrade();
    window
        .spawn(cx, async move |cx: &mut AsyncWindowContext| {
            cx.background_executor().timer(timing.exit).await;
            let _ = cx.update(|window, cx| {
                let _ = entity.update(cx, |entity, entity_cx| on_done(entity, window, entity_cx));
            });
        })
        .detach();
}

/// A dimmed, full-cover backdrop layer without dismissal wiring (used by
/// overlays that close only through their own controls, or that dismiss via
/// `on_mouse_down_out` on the panel itself, like
/// [`super::dialog::Dialog`]).
pub(crate) fn backdrop(bg: Hsla) -> Div {
    div().absolute().inset_0().bg(bg)
}

/// A dimmed, full-cover backdrop layer that invokes `on_dismiss` on left
/// mouse-down. Overlay panels rendered on top must call
/// `cx.stop_propagation()` in their own mouse-down handlers to stay open.
pub(crate) fn dismissible_backdrop(
    bg: Hsla,
    on_dismiss: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> Div {
    backdrop(bg).on_mouse_down(MouseButton::Left, move |event, window, cx| {
        (on_dismiss)(event, window, cx);
    })
}

/// The shared popover-style menu panel shell: min width, popover background
/// and foreground, border, radius, shadow and 4px padding. Position-specific
/// styling (`occlude`, coordinates, max width, user style refinement) is
/// applied by the caller.
pub(crate) fn menu_panel(cx: &App) -> Div {
    let theme = use_theme(cx);
    div()
        .min_w(px(200.0))
        .flex()
        .flex_col()
        .bg(theme.tokens.popover)
        .text_color(theme.tokens.popover_foreground)
        .border_1()
        .border_color(theme.tokens.border)
        .rounded(theme.tokens.radius_md)
        .shadow_lg()
        .p(px(4.0))
}

/// The per-component visual knobs where context menus and popover menus
/// intentionally differ; everything else about a row is shared.
pub(crate) struct MenuRowChrome {
    /// Horizontal padding.
    pub padding_x: Pixels,
    /// Vertical padding.
    pub padding_y: Pixels,
    /// Corner radius.
    pub radius: Pixels,
    /// Icon size.
    pub icon_size: Pixels,
    /// Explicit icon color; `None` inherits the row's text color.
    pub icon_color: Option<Hsla>,
}

/// The data needed to render one shared menu row.
pub(crate) struct MenuRow {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    pub disabled: bool,
    pub on_click: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    /// Invoked after `on_click`, e.g. to close the menu host.
    pub on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

/// Render one menu row: hover highlight when enabled, muted and translucent
/// when disabled, optional leading icon, and click wiring (`on_click`, then
/// `on_close`, then propagation is stopped).
pub(crate) fn menu_row(cx: &App, row: MenuRow, chrome: MenuRowChrome) -> Stateful<Div> {
    let theme = use_theme(cx);
    let MenuRow {
        id,
        label,
        icon,
        disabled,
        on_click,
        on_close,
    } = row;

    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(chrome.padding_x)
        .py(chrome.padding_y)
        .rounded(chrome.radius)
        .text_size(px(14.0))
        .cursor(if disabled {
            CursorStyle::Arrow
        } else {
            CursorStyle::PointingHand
        })
        .when(disabled, |this: Stateful<Div>| {
            this.text_color(theme.tokens.muted_foreground).opacity(0.5)
        })
        .when(!disabled, |this: Stateful<Div>| {
            this.text_color(theme.tokens.popover_foreground)
                .hover(|style| style.bg(theme.tokens.accent.opacity(0.1)))
        })
        .when(!disabled, move |this: Stateful<Div>| match on_click {
            Some(handler) => {
                let on_close = on_close.clone();
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    (handler)(window, cx);
                    if let Some(close_handler) = on_close.as_ref() {
                        close_handler(window, cx);
                    }
                    cx.stop_propagation();
                })
            }
            None => this,
        })
        .when_some(icon, |this: Stateful<Div>, icon| {
            let mut icon = Icon::new(icon).size(chrome.icon_size);
            if let Some(color) = chrome.icon_color {
                icon = icon.color(color);
            }
            this.child(icon)
        })
        .child(label)
}
