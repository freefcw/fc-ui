//! Context menu component for right-click menus.

use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::overlays::overlay_shell::{
    dismiss_animation, menu_panel, menu_row, MenuRow, MenuRowChrome, CONTEXT_MENU_TIMING,
};

#[derive(Clone)]
pub struct ContextMenuItem {
    id: SharedString,
    label: SharedString,
    icon: Option<SharedString>,
    disabled: bool,
    divider: bool,
    on_click: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl ContextMenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            disabled: false,
            divider: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = divider;
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn separator() -> Self {
        Self {
            id: "separator".into(),
            label: "".into(),
            icon: None,
            disabled: true,
            divider: true,
            on_click: None,
        }
    }
}

#[derive(IntoElement)]
pub struct ContextMenu {
    position: Point<Pixels>,
    items: Vec<ContextMenuItem>,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    dismissing: bool,
    style: StyleRefinement,
}

impl ContextMenu {
    pub fn new(position: Point<Pixels>) -> Self {
        Self {
            position,
            items: Vec::new(),
            on_close: None,
            dismissing: false,
            style: StyleRefinement::default(),
        }
    }

    pub fn dismissing(mut self, dismissing: bool) -> Self {
        self.dismissing = dismissing;
        self
    }

    pub fn item(mut self, item: ContextMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<ContextMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Styled for ContextMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ContextMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let position = self.position;
        let on_close_handler = self.on_close.clone();
        let user_style = self.style;
        let dismissing = self.dismissing;

        div()
            .absolute()
            .inset_0()
            .when(on_close_handler.is_some(), |this: Div| {
                let handler = on_close_handler.clone().unwrap();
                let handler2 = handler.clone();
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    handler(window, cx);
                })
                .on_mouse_down(MouseButton::Right, move |_, window, cx| {
                    handler2(window, cx);
                })
            })
            .child(
                menu_panel(cx)
                    .absolute()
                    .occlude()
                    .left(position.x)
                    .top(position.y)
                    .map(|this| {
                        let mut div = this;
                        div.style().refine(&user_style);
                        div
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(self.items.into_iter().map(|item| {
                        if item.label.is_empty() && item.divider {
                            return div()
                                .h(px(1.0))
                                .my(px(4.0))
                                .bg(theme.tokens.border)
                                .into_any_element();
                        }

                        menu_row(
                            cx,
                            MenuRow {
                                id: item.id,
                                label: item.label,
                                icon: item.icon,
                                disabled: item.disabled,
                                on_click: item.on_click,
                                on_close: on_close_handler.clone(),
                            },
                            MenuRowChrome {
                                padding_x: px(8.0),
                                padding_y: px(6.0),
                                radius: theme.tokens.radius_sm,
                                icon_size: px(14.0),
                                icon_color: None,
                            },
                        )
                        .into_any_element()
                    }))
                    .with_animation(
                        if self.dismissing {
                            "ctx-menu-exit"
                        } else {
                            "ctx-menu-enter"
                        },
                        dismiss_animation(CONTEXT_MENU_TIMING, dismissing),
                        move |el, delta| {
                            if dismissing {
                                el.opacity(1.0 - delta).mt(px(4.0 * delta))
                            } else {
                                el.opacity(delta).mt(px(4.0 * (1.0 - delta)))
                            }
                        },
                    ),
            )
    }
}
