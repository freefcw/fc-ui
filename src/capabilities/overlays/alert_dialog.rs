//! Alert dialog component for confirmations.

use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

const ALERT_DIALOG_CONTEXT: &str = "AlertDialog";

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::primitives::button::{Button, ButtonSize, ButtonVariant};

actions!(alert_dialog, [AlertDialogCancel]);

pub struct AlertDialog {
    focus_handle: FocusHandle,
    title: SharedString,
    description: SharedString,
    cancel_text: SharedString,
    action_text: SharedString,
    destructive: bool,
    on_cancel: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    on_action: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    focused: bool,
    previous_focus: Option<FocusHandle>,
    _focus_lost_subscription: Option<Subscription>,
    style: StyleRefinement,
}

impl AlertDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            title: "Are you sure?".into(),
            description: "This action cannot be undone.".into(),
            cancel_text: "Cancel".into(),
            action_text: "Continue".into(),
            destructive: false,
            on_cancel: None,
            on_action: None,
            focused: false,
            previous_focus: None,
            _focus_lost_subscription: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = description.into();
        self
    }

    pub fn cancel_text(mut self, text: impl Into<SharedString>) -> Self {
        self.cancel_text = text.into();
        self
    }

    pub fn action_text(mut self, text: impl Into<SharedString>) -> Self {
        self.action_text = text.into();
        self
    }

    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    pub fn on_cancel<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_cancel = Some(Rc::new(handler));
        self
    }

    pub fn on_action<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_action = Some(Rc::new(handler));
        self
    }

    fn restore_previous_focus(&mut self, window: &mut Window, cx: &App) {
        if let Some(previous_focus) = self.previous_focus.take() {
            window.focus(&previous_focus);
        } else if let Some(restore_target) = window.focus_lost_restore_target(cx) {
            window.focus(&restore_target);
        }
    }

    fn handle_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.restore_previous_focus(window, cx);
        if let Some(handler) = &self.on_cancel {
            handler(window, cx);
        }
    }

    fn handle_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.restore_previous_focus(window, cx);
        if let Some(handler) = &self.on_action {
            handler(window, cx);
        }
    }

    fn handle_escape(
        &mut self,
        _: &AlertDialogCancel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_cancel(window, cx);
    }
}

pub fn init_alert_dialog(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "alert_dialog") {
        return;
    }
    cx.bind_keys([KeyBinding::new(
        "escape",
        AlertDialogCancel,
        Some(ALERT_DIALOG_CONTEXT),
    )]);
}

impl Styled for AlertDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for AlertDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style.clone();
        let title = self.title.clone();
        let description = self.description.clone();
        let cancel_text = self.cancel_text.clone();
        let action_text = self.action_text.clone();
        let destructive = self.destructive;

        if !self.focused {
            self.previous_focus = window.focused(cx);
            window.focus(&self.focus_handle);
            self.focused = true;
            self._focus_lost_subscription = Some(cx.on_focus_lost(window, |this, window, cx| {
                if this.focused {
                    this.focused = false;
                    this.restore_previous_focus(window, cx);
                    cx.notify();
                }
            }));
        }

        div()
            .track_focus(&self.focus_handle)
            .key_context(ALERT_DIALOG_CONTEXT)
            .on_action(cx.listener(Self::handle_escape))
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(hsla(0.0, 0.0, 0.0, 0.5))
            .child(
                div()
                    .w(px(500.0))
                    .bg(theme.tokens.card)
                    .border_1()
                    .border_color(theme.tokens.border)
                    .rounded(theme.tokens.radius_lg)
                    .shadow(smallvec::smallvec![BoxShadow {
                        color: hsla(0.0, 0.0, 0.0, 0.25),
                        offset: point(px(0.0), px(8.0)),
                        blur_radius: px(24.0),
                        spread_radius: px(0.0),
                        inset: false,
                    }])
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(16.0))
                            .p(px(24.0))
                            .child(
                                div()
                                    .text_size(px(18.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.tokens.foreground)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .text_color(theme.tokens.muted_foreground)
                                    .line_height(relative(1.5))
                                    .child(description),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(12.0))
                                    .justify_end()
                                    .child(
                                        Button::new("alert-cancel-btn", cancel_text)
                                            .variant(ButtonVariant::Outline)
                                            .size(ButtonSize::Md)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.handle_cancel(window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("alert-action-btn", action_text)
                                            .variant(if destructive {
                                                ButtonVariant::Destructive
                                            } else {
                                                ButtonVariant::Default
                                            })
                                            .size(ButtonSize::Md)
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.handle_action(window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .map(|this| {
                        let mut div = this;
                        div.style().refine(&user_style);
                        div
                    }),
            )
    }
}
