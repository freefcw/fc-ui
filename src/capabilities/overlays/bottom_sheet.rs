//! Bottom sheet component for slide-up panels.
//!
//! Accessibility alignment: `BottomSheet` captures focus while mounted,
//! restores the previously focused element on close, and closes on `Escape`
//! through the `BottomSheet` key context, matching the behavior of
//! [`super::sheet::Sheet`]. The public element stays a stateless
//! `RenderOnce`; the focus state lives in an internal keyed entity.

use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;

use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::motion::animations::presets;
use crate::capabilities::overlays::overlay_shell::{dismissible_backdrop, OverlayFocusGuard};
use crate::capabilities::primitives::text::{Text, TextVariant};

const BOTTOM_SHEET_CONTEXT: &str = "BottomSheet";

actions!(bottom_sheet, [BottomSheetClose]);

pub fn init_bottom_sheet(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "bottom_sheet") {
        return;
    }
    cx.bind_keys([KeyBinding::new(
        "escape",
        BottomSheetClose,
        Some(BOTTOM_SHEET_CONTEXT),
    )]);
}

/// Focus state backing the stateless [`BottomSheet`] element: the panel's
/// focus handle plus the shared capture/restore guard. Lives as long as the
/// sheet is mounted (keyed element state).
struct BottomSheetFocusState {
    focus_handle: FocusHandle,
    guard: OverlayFocusGuard,
}

/// Keep the focus listener alive through the frame that removes the sheet.
/// GPUI dispatches focus-lost events before releasing unused keyed state.
struct BottomSheetMountState {
    focus_state: Entity<BottomSheetFocusState>,
}

fn close_bottom_sheet(
    focus_state: &Entity<BottomSheetFocusState>,
    on_close: &Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    window: &mut Window,
    cx: &mut App,
) {
    focus_state.update(cx, |state, cx| state.guard.restore_focus(window, cx));
    if let Some(handler) = on_close.as_ref() {
        handler(window, cx);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BottomSheetSize {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
    Custom,
}

impl BottomSheetSize {
    fn height(&self) -> Pixels {
        match self {
            Self::Sm => px(300.0),
            Self::Md => px(400.0),
            Self::Lg => px(500.0),
            Self::Xl => px(600.0),
            Self::Custom => px(400.0),
        }
    }
}

#[derive(IntoElement)]
pub struct BottomSheet {
    size: BottomSheetSize,
    custom_height: Option<Pixels>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    content: Option<AnyElement>,
    actions: Option<AnyElement>,
    show_drag_handle: bool,
    close_on_backdrop_click: bool,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    style: StyleRefinement,
}

impl BottomSheet {
    pub fn new() -> Self {
        Self {
            size: BottomSheetSize::default(),
            custom_height: None,
            title: None,
            description: None,
            content: None,
            actions: None,
            show_drag_handle: true,
            close_on_backdrop_click: true,
            on_close: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: BottomSheetSize) -> Self {
        self.size = size;
        self
    }

    pub fn height(mut self, height: impl Into<Pixels>) -> Self {
        self.custom_height = Some(height.into());
        self.size = BottomSheetSize::Custom;
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

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }

    pub fn show_drag_handle(mut self, show: bool) -> Self {
        self.show_drag_handle = show;
        self
    }

    pub fn close_on_backdrop_click(mut self, close: bool) -> Self {
        self.close_on_backdrop_click = close;
        self
    }

    pub fn on_close<F>(mut self, handler: F) -> Self
    where
        F: Fn(&mut Window, &mut App) + 'static,
    {
        self.on_close = Some(Rc::new(handler));
        self
    }

    fn get_sheet_height(&self) -> Pixels {
        if let Some(height) = self.custom_height {
            return height;
        }
        self.size.height()
    }
}

impl Default for BottomSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for BottomSheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for BottomSheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let has_header =
            self.title.is_some() || self.description.is_some() || self.actions.is_some();
        let sheet_height = self.get_sheet_height();
        let on_close = self.on_close.clone();
        let close_on_backdrop_click = self.close_on_backdrop_click;
        let user_style = self.style;

        let mount_state = window.use_keyed_state(
            ElementId::Name("bottom-sheet-focus".into()),
            cx,
            |window, cx| {
                let focus_state = cx.new(|cx| BottomSheetFocusState {
                    focus_handle: cx.focus_handle(),
                    guard: OverlayFocusGuard::new(),
                });
                cx.observe(&focus_state, |_, _, cx| cx.notify()).detach();
                cx.on_release_in(window, |mount: &mut BottomSheetMountState, window, cx| {
                    mount.focus_state.update(cx, |state, cx| {
                        // Also cover removal before the sheet had a focus
                        // path, preserving any explicit host focus handoff.
                        if state.focus_handle.contains_focused(window, cx)
                            || window.focused(cx).is_none()
                        {
                            state.guard.restore_focus(window, cx);
                        }
                    });
                })
                .detach();
                BottomSheetMountState { focus_state }
            },
        );
        let focus_state = mount_state.read(cx).focus_state.clone();
        let focus_handle = focus_state.read(cx).focus_handle.clone();

        // Capture focus on the first frame; it is restored on close (and on
        // window focus loss) by the guard.
        focus_state.update(cx, |state, cx| {
            state.guard.capture_focus(
                window,
                cx,
                &state.focus_handle,
                |state: &mut BottomSheetFocusState| &mut state.guard,
            );
        });

        let backdrop_focus_state = focus_state.clone();
        let backdrop_on_close = on_close.clone();
        let escape_focus_state = focus_state;
        let escape_on_close = on_close;

        deferred(
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .child(dismissible_backdrop(
                    hsla(0.0, 0.0, 0.0, 0.6),
                    move |_, window, cx| {
                        if close_on_backdrop_click {
                            close_bottom_sheet(
                                &backdrop_focus_state,
                                &backdrop_on_close,
                                window,
                                cx,
                            );
                        }
                    },
                ))
                .child(
                    div()
                        .id("bottom-sheet-panel")
                        .occlude()
                        .track_focus(&focus_handle)
                        .key_context(BOTTOM_SHEET_CONTEXT)
                        .on_action(move |_: &BottomSheetClose, window, cx| {
                            close_bottom_sheet(&escape_focus_state, &escape_on_close, window, cx);
                        })
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(sheet_height)
                        .flex()
                        .flex_col()
                        .bg(theme.tokens.background)
                        .border_t_1()
                        .border_color(theme.tokens.border)
                        .rounded_tl(theme.tokens.radius_xl)
                        .rounded_tr(theme.tokens.radius_xl)
                        .shadow(smallvec::smallvec![BoxShadow {
                            color: hsla(0.0, 0.0, 0.0, 0.3),
                            offset: point(px(0.0), px(-4.0)),
                            blur_radius: px(24.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }])
                        .map(|this| {
                            let mut div = this;
                            div.style().refine(&user_style);
                            div
                        })
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .when(self.show_drag_handle, |this| {
                            this.child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .pt(px(12.0))
                                    .pb(px(8.0))
                                    .child(
                                        div()
                                            .w(px(40.0))
                                            .h(px(4.0))
                                            .bg(theme.tokens.muted.opacity(0.5))
                                            .rounded(px(2.0)),
                                    ),
                            )
                        })
                        .when(has_header, |this| {
                            this.child(
                                div()
                                    .flex()
                                    .items_start()
                                    .justify_between()
                                    .px(px(24.0))
                                    .pt(px(16.0))
                                    .pb(px(16.0))
                                    .border_b_1()
                                    .border_color(theme.tokens.border)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(4.0))
                                            .when_some(self.title, |this: Div, title| {
                                                this.child(
                                                    Text::new(title).variant(TextVariant::H4),
                                                )
                                            })
                                            .when_some(self.description, |this: Div, desc| {
                                                this.child(
                                                    Text::new(desc)
                                                        .variant(TextVariant::Caption)
                                                        .color(theme.tokens.muted_foreground),
                                                )
                                            }),
                                    )
                                    .when_some(self.actions, |this: Div, actions| {
                                        this.child(
                                            div().flex().items_center().gap(px(8.0)).child(actions),
                                        )
                                    }),
                            )
                        })
                        .when_some(self.content, |this, content| {
                            this.child(div().flex_1().overflow_hidden().child(content))
                        })
                        .with_animation(
                            "bottom-sheet-slide",
                            presets::slide_in_bottom(),
                            |div, delta| div.mb(px(-600.0 * (1.0 - delta))),
                        ),
                ),
        )
        .with_priority(1)
    }
}
