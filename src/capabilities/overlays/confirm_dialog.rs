//! Dialog frame - a modal dialog layout shell with backdrop and
//! customizable slots.
//!
//! Note: [`DialogFrame`] (formerly `Dialog`) is a simple `RenderOnce`
//! element, distinct from [`super::dialog::Dialog`], which is a stateful
//! entity with focus management, escape / backdrop dismissal and an
//! `on_close` callback. Use `super::dialog::Dialog` for interactive modals;
//! use `DialogFrame` when the host manages interaction itself.

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::layout::VStack;
use gpui::{prelude::FluentBuilder as _, *};
#[derive(IntoElement)]
pub struct DialogFrame {
    width: Option<Pixels>,
    max_width: Option<Length>,
    header: Option<AnyElement>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    on_backdrop_click: Option<Box<dyn Fn(&mut Window, &mut App)>>,
    style: StyleRefinement,
}

impl Default for DialogFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl DialogFrame {
    pub fn new() -> Self {
        Self {
            width: None,
            max_width: None,
            header: None,
            content: None,
            footer: None,
            on_backdrop_click: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn max_width(mut self, max_width: Length) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn header(mut self, header: impl IntoElement) -> Self {
        self.header = Some(header.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn on_backdrop_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_backdrop_click = Some(Box::new(handler));
        self
    }
}

/// Deprecated alias for [`DialogFrame`]; the interactive dialog is
/// [`super::dialog::Dialog`].
#[deprecated(
    note = "renamed to DialogFrame; the dialog.rs Dialog is the interactive one; alias removed in 0.10"
)]
pub type Dialog = DialogFrame;

impl Styled for DialogFrame {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DialogFrame {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let backdrop_click_handler = self.on_backdrop_click;
        let user_style = self.style;

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                crate::capabilities::overlays::overlay_shell::dismissible_backdrop(
                    gpui::black().opacity(0.5),
                    move |_, window, cx| {
                        if let Some(handler) = backdrop_click_handler.as_ref() {
                            handler(window, cx);
                        }
                    },
                ),
            )
            .child(
                VStack::new()
                    .when_some(self.width, |this, width| this.w(width))
                    .when(self.width.is_none(), |this| this.w(px(500.0)))
                    .when_some(self.max_width, |this, max_width| this.max_w(max_width))
                    .when(self.max_width.is_none(), |this| this.max_w(relative(0.9)))
                    .bg(theme.tokens.card)
                    .border_1()
                    .border_color(theme.tokens.border)
                    .rounded(theme.tokens.radius_lg)
                    .shadow_xl()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _window, cx| {
                        cx.stop_propagation();
                    })
                    .spacing(0.0)
                    .when_some(self.header, |this, header| this.child(header))
                    .when_some(self.content, |this, content| this.child(content))
                    .when_some(self.footer, |this, footer| this.child(footer))
                    .map(|this| {
                        let mut div = this;
                        div.style().refine(&user_style);
                        div
                    }),
            )
    }
}
