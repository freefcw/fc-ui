//! Dialog component with focus trap and backdrop.
//!
//! Note: this [`Dialog`] is a stateful entity; do not confuse it with
//! [`super::confirm_dialog::DialogFrame`], a lightweight `RenderOnce`
//! element for simple confirmation layouts without focus management.

use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::overlays::overlay_shell::{
    dismiss_animation, fade_animation, schedule_reap, OverlayFocusGuard, DIALOG_TIMING,
};
use crate::capabilities::primitives::button::{Button, ButtonSize, ButtonVariant};

actions!(dialog, [DialogCancel]);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DialogSize {
    Sm,
    Md,
    Lg,
    Xl,
    Full,
}

impl DialogSize {
    fn width(&self) -> Length {
        match self {
            Self::Sm => px(400.0).into(),
            Self::Md => px(500.0).into(),
            Self::Lg => px(600.0).into(),
            Self::Xl => px(800.0).into(),
            Self::Full => relative(0.95).into(),
        }
    }

    fn max_height(&self) -> Length {
        match self {
            Self::Full => relative(0.95).into(),
            _ => relative(0.85).into(),
        }
    }
}

type DialogElementBuilder = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

fn dialog_element_builder<E, F>(builder: F) -> DialogElementBuilder
where
    E: IntoElement + 'static,
    F: Fn(&mut Window, &mut App) -> E + 'static,
{
    Rc::new(move |window, cx| builder(window, cx).into_any_element())
}

pub struct Dialog {
    focus_handle: FocusHandle,
    header: Option<DialogElementBuilder>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    size: DialogSize,
    children: Vec<DialogElementBuilder>,
    footer: Option<DialogElementBuilder>,
    show_close_button: bool,
    close_on_backdrop_click: bool,
    close_on_escape: bool,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    focus_guard: OverlayFocusGuard,
    dismissing: bool,
    dismiss_complete: bool,
    style: StyleRefinement,
}

impl Dialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            header: None,
            title: None,
            description: None,
            size: DialogSize::Md,
            children: vec![],
            footer: None,
            show_close_button: true,
            close_on_backdrop_click: true,
            close_on_escape: true,
            on_close: None,
            focus_guard: OverlayFocusGuard::new(),
            dismissing: false,
            dismiss_complete: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn header<E, F>(mut self, header: F) -> Self
    where
        E: IntoElement + 'static,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.header = Some(dialog_element_builder(header));
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn size(mut self, size: DialogSize) -> Self {
        self.size = size;
        self
    }

    pub fn child<E, F>(mut self, child: F) -> Self
    where
        E: IntoElement + 'static,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.children.push(dialog_element_builder(child));
        self
    }

    pub fn children<E, F>(mut self, children: impl IntoIterator<Item = F>) -> Self
    where
        E: IntoElement + 'static,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.children
            .extend(children.into_iter().map(dialog_element_builder));
        self
    }

    pub fn footer<E, F>(mut self, footer: F) -> Self
    where
        E: IntoElement + 'static,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.footer = Some(dialog_element_builder(footer));
        self
    }

    pub fn show_close_button(mut self, show: bool) -> Self {
        self.show_close_button = show;
        self
    }

    pub fn close_on_backdrop_click(mut self, close: bool) -> Self {
        self.close_on_backdrop_click = close;
        self
    }

    pub fn close_on_escape(mut self, close: bool) -> Self {
        self.close_on_escape = close;
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    fn restore_previous_focus(&mut self, window: &mut Window, cx: &App) {
        self.focus_guard.restore_focus(window, cx);
    }

    fn handle_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dismissing {
            return;
        }
        self.dismissing = true;
        cx.notify();

        schedule_reap(
            &cx.entity(),
            window,
            cx,
            DIALOG_TIMING,
            |dialog, window, cx| {
                dialog.dismiss_complete = true;
                dialog.restore_previous_focus(window, cx);
                if let Some(handler) = dialog.on_close.take() {
                    (handler)(window, cx);
                }
                cx.notify();
            },
        );
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Dialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.dismiss_complete {
            return div().into_any_element();
        }

        let theme = use_theme(cx);
        let header = self.header.as_ref().map(|header| header(window, cx));
        let children = self
            .children
            .iter()
            .map(|child| child(window, cx))
            .collect::<Vec<_>>();
        let footer = self.footer.as_ref().map(|footer| footer(window, cx));
        let has_slot_header = header.is_some();
        let has_header = has_slot_header
            || self.title.is_some()
            || self.description.is_some()
            || self.show_close_button;
        let has_children = !children.is_empty();

        let dialog_entity = cx.entity().clone();
        let user_style = self.style.clone();
        let dismissing = self.dismissing;

        self.focus_guard
            .capture_focus(window, cx, &self.focus_handle, |dialog: &mut Dialog| {
                &mut dialog.focus_guard
            });

        div()
            .id("dialog-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.5))
            .child(
                div()
                    .id("dialog-content")
                    .debug_selector(|| "dialog-content".to_string())
                    .role(Role::Dialog)
                    .when_some(self.title.clone(), |this, title| this.aria_label(title))
                    .when_some(self.description.clone(), |this, description| {
                        this.aria_description(description)
                    })
                    .aria_modal(true)
                    .occlude()
                    .key_context("Dialog")
                    .track_focus(&self.focus_handle)
                    .when(self.close_on_backdrop_click, |this| {
                        this.on_mouse_down_out(cx.listener(|this, _, window, cx| {
                            this.handle_close(window, cx);
                        }))
                    })
                    .on_action(cx.listener(|this, _: &DialogCancel, window, cx| {
                        if this.close_on_escape {
                            this.handle_close(window, cx);
                        }
                    }))
                    .w(self.size.width())
                    .max_h(self.size.max_height())
                    .flex()
                    .flex_col()
                    .bg(theme.tokens.card)
                    .border_1()
                    .border_color(theme.tokens.border)
                    .rounded(theme.tokens.radius_lg)
                    .shadow_xl()
                    .overflow_hidden()
                    .when(has_header, |this| {
                        if has_slot_header {
                            this.child(header.unwrap())
                        } else {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(8.0))
                                    .px(px(24.0))
                                    .pt(px(24.0))
                                    .pb(px(16.0))
                                    .when(footer.is_none() && !has_children, |this| {
                                        this.pb(px(24.0))
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .items_start()
                                            .justify_between()
                                            .gap(px(16.0))
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(4.0))
                                                    .flex_1()
                                                    .when_some(self.title.clone(), |this, title| {
                                                        this.child(
                                                            div()
                                                                .text_size(px(18.0))
                                                                .font_family(
                                                                    theme
                                                                        .tokens
                                                                        .font_family
                                                                        .clone(),
                                                                )
                                                                .font_weight(FontWeight::SEMIBOLD)
                                                                .text_color(theme.tokens.foreground)
                                                                .line_height(relative(1.2))
                                                                .child(title),
                                                        )
                                                    })
                                                    .when_some(
                                                        self.description.clone(),
                                                        |this, desc| {
                                                            this.child(
                                                                div()
                                                                    .text_size(px(14.0))
                                                                    .font_family(
                                                                        theme
                                                                            .tokens
                                                                            .font_family
                                                                            .clone(),
                                                                    )
                                                                    .text_color(
                                                                        theme
                                                                            .tokens
                                                                            .muted_foreground,
                                                                    )
                                                                    .line_height(relative(1.5))
                                                                    .child(desc),
                                                            )
                                                        },
                                                    ),
                                            )
                                            .when(self.show_close_button, |this| {
                                                let dialog_entity = dialog_entity.clone();
                                                this.child(
                                                    Button::new("dialog-close-btn", "×")
                                                        .aria_label("Close dialog")
                                                        .variant(ButtonVariant::Ghost)
                                                        .size(ButtonSize::Icon)
                                                        .on_click(move |_, window, cx| {
                                                            cx.update_entity(
                                                                &dialog_entity,
                                                                |dialog, cx| {
                                                                    dialog.handle_close(window, cx);
                                                                },
                                                            );
                                                        }),
                                                )
                                            }),
                                    ),
                            )
                        }
                    })
                    .when(has_children, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(16.0))
                                .px(px(24.0))
                                .py(px(16.0))
                                .flex_1()
                                .children(children),
                        )
                    })
                    .map(|this| {
                        let mut div = this;
                        div.style().refine(&user_style);
                        div
                    })
                    .when_some(footer, |this, footer| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap(px(8.0))
                                .px(px(24.0))
                                .py(px(16.0))
                                .border_t_1()
                                .border_color(theme.tokens.border)
                                .child(footer),
                        )
                    })
                    .with_animation(
                        if dismissing {
                            "dialog-content-exit"
                        } else {
                            "dialog-content-enter"
                        },
                        dismiss_animation(DIALOG_TIMING, dismissing),
                        move |el, delta| {
                            if dismissing {
                                el.opacity(1.0 - delta).mt(px(8.0 * delta))
                            } else {
                                el.opacity(delta).mt(px(12.0 * (1.0 - delta)))
                            }
                        },
                    ),
            )
            .with_animation(
                if dismissing {
                    "dialog-backdrop-exit"
                } else {
                    "dialog-backdrop-fade"
                },
                fade_animation(DIALOG_TIMING.exit, dismissing),
                move |el, delta| {
                    if dismissing {
                        el.opacity(1.0 - delta)
                    } else {
                        el.opacity(delta)
                    }
                },
            )
            .into_any_element()
    }
}

impl Focusable for Dialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<()> for Dialog {}

pub fn init_dialog(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "dialog") {
        return;
    }
    cx.bind_keys([KeyBinding::new("escape", DialogCancel, Some("Dialog"))]);
}
