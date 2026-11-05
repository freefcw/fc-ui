//! Generic flat list primitives.
//!
//! [`List`] and [`ListItem`] started inside `tree` and now live here; `tree`
//! re-exports them so the historical `tree::List` / `tree::ListItem` paths
//! keep resolving.

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::primitives::icon::Icon;
use crate::capabilities::primitives::icon_source::IconSource;
use gpui::{prelude::*, *};
use std::sync::Arc;

#[derive(Clone)]
pub struct ListItem<T: Clone> {
    pub id: T,
    pub label: SharedString,
    pub icon: Option<IconSource>,
    pub badge: Option<SharedString>,
    pub disabled: bool,
}

impl<T: Clone> ListItem<T> {
    pub fn new(id: T, label: impl Into<SharedString>) -> Self {
        Self {
            id,
            label: label.into(),
            icon: None,
            badge: None,
            disabled: false,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<IconSource>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn with_badge(mut self, badge: impl Into<SharedString>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
pub struct List<T: Clone + PartialEq + 'static> {
    items: Vec<ListItem<T>>,
    selected_id: Option<T>,
    on_select: Option<Arc<dyn Fn(&T, &mut Window, &mut App) + Send + Sync + 'static>>,
}

impl<T: Clone + PartialEq + 'static> Default for List<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone + PartialEq + 'static> List<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected_id: None,
            on_select: None,
        }
    }

    pub fn items(mut self, items: Vec<ListItem<T>>) -> Self {
        self.items = items;
        self
    }

    pub fn selected_id(mut self, id: T) -> Self {
        self.selected_id = Some(id);
        self
    }

    pub fn on_select<F>(mut self, f: F) -> Self
    where
        F: Fn(&T, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(f));
        self
    }

    fn is_selected(&self, item_id: &T) -> bool {
        self.selected_id.as_ref() == Some(item_id)
    }

    fn render_item(
        &self,
        item: &ListItem<T>,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> impl IntoElement {
        let is_selected = self.is_selected(&item.id);

        let base = div()
            .flex()
            .items_center()
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .cursor(if item.disabled {
                CursorStyle::Arrow
            } else {
                CursorStyle::PointingHand
            });

        let styled = base
            .bg(if is_selected {
                theme.tokens.accent
            } else {
                gpui::transparent_black()
            })
            .text_color(if is_selected {
                theme.tokens.accent_foreground
            } else if item.disabled {
                theme.tokens.muted_foreground
            } else {
                theme.tokens.primary
            })
            .when(!item.disabled && !is_selected, |div| {
                div.hover(|mut style| {
                    style.background = Some(theme.tokens.accent.opacity(0.5).into());
                    style
                })
            });

        let element = if let Some(icon) = item.icon.as_ref() {
            styled.child(
                div()
                    .mr(px(10.0))
                    .child(
                        Icon::new(icon.clone())
                            .size(px(18.0))
                            .color(if is_selected {
                                theme.tokens.accent_foreground
                            } else if item.disabled {
                                theme.tokens.muted_foreground
                            } else {
                                theme.tokens.primary
                            }),
                    ),
            )
        } else {
            styled
        };

        let with_label = element.child(
            div()
                .flex_1()
                .text_size(px(14.0))
                .font_family(theme.tokens.font_family.clone())
                .font_weight(if is_selected {
                    FontWeight::SEMIBOLD
                } else {
                    FontWeight::NORMAL
                })
                .child(item.label.clone()),
        );

        let with_badge = with_label.when_some(item.badge.as_ref(), |parent, badge| {
            parent.child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(10.0))
                    .bg(if is_selected {
                        theme.tokens.accent_foreground.opacity(0.2)
                    } else {
                        theme.tokens.muted
                    })
                    .text_size(px(11.0))
                    .font_family(theme.tokens.font_family.clone())
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(if is_selected {
                        theme.tokens.accent_foreground
                    } else {
                        theme.tokens.muted_foreground
                    })
                    .child(badge.clone()),
            )
        });

        with_badge.when(!item.disabled, |this| {
            let on_select = self.on_select.clone();
            let item_id = item.id.clone();

            this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                if let Some(on_select) = on_select.clone() {
                    on_select(&item_id, window, cx);
                }
            })
        })
    }
}

impl<T: Clone + PartialEq + 'static> RenderOnce for List<T> {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);

        div()
            .flex()
            .flex_col()
            .w_full()
            .bg(theme.tokens.background)
            .children(
                self.items
                    .iter()
                    .map(|item| self.render_item(item, &theme).into_any_element()),
            )
    }
}
