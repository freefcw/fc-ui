//! Popover menu component with positioned menu items.

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::overlays::overlay_shell::{menu_panel, menu_row, MenuRow, MenuRowChrome};
use gpui::prelude::FluentBuilder;
use gpui::*;
use std::rc::Rc;

pub struct PopoverMenuItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: Option<SharedString>,
    pub on_click: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub disabled: bool,
}

impl PopoverMenuItem {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            on_click: None,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
pub struct PopoverMenu {
    position: Point<Pixels>,
    items: Vec<PopoverMenuItem>,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl PopoverMenu {
    pub fn new(position: Point<Pixels>, items: Vec<PopoverMenuItem>) -> Self {
        Self {
            position,
            items,
            on_close: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl Styled for PopoverMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for PopoverMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let on_close_backdrop = self.on_close.clone();
        let user_style = self.style;

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if let Some(ref handler) = on_close_backdrop {
                    handler(window, cx);
                }
            })
            .child(
                deferred(
                    anchored().snap_to_window().position(self.position).child(
                        div().occlude().child(
                            menu_panel(cx)
                                .max_w(px(300.0))
                                .map(|this| {
                                    let mut div = this;
                                    div.style().refine(&user_style);
                                    div
                                })
                                .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                    cx.stop_propagation();
                                })
                                .children(self.items.into_iter().map(|item| {
                                    menu_row(
                                        cx,
                                        MenuRow {
                                            id: item.id,
                                            label: item.label,
                                            icon: item.icon,
                                            disabled: item.disabled,
                                            on_click: item.on_click,
                                            on_close: None,
                                        },
                                        MenuRowChrome {
                                            padding_x: px(12.0),
                                            padding_y: px(8.0),
                                            radius: px(4.0),
                                            icon_size: px(16.0),
                                            icon_color: Some(theme.tokens.foreground),
                                        },
                                    )
                                })),
                        ),
                    ),
                )
                .with_priority(1),
            )
    }
}
