use fc_ui::components::{button::Button, icon_button::IconButton};
use gpui::{
    div, Context, FocusHandle, IntoElement, ParentElement as _, Render, TestAppContext,
    VisualTestContext, Window,
};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

fn decorate<T: gpui::InteractiveElement>(element: T, handle: &FocusHandle) -> T {
    element.track_focus(handle)
}

struct FocusHost {
    handle: FocusHandle,
    icon: bool,
    stop: bool,
}
impl Render for FocusHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let handle = self.handle.clone().tab_index(3).tab_stop(self.stop);
        if self.icon {
            decorate(IconButton::new("plus"), &handle).into_any_element()
        } else {
            decorate(Button::new("external-focus", "External"), &handle).into_any_element()
        }
    }
}

fn assert_external_focus(cx: &mut TestAppContext, icon: bool) {
    cx.update(fc_ui::init);
    let (host, cx) = cx.add_window_view(|_, cx| FocusHost {
        handle: cx.focus_handle(),
        icon,
        stop: true,
    });
    draw(cx);
    cx.update(|window, _| window.focus_next());
    let handle = host.read_with(cx, |host, _| host.handle.clone());
    assert_eq!(cx.update(|window, cx| window.focused(cx)), Some(handle));
}

#[gpui::test]
fn button_respects_focus_from_generic_decoration(cx: &mut TestAppContext) {
    assert_external_focus(cx, false);
}
#[gpui::test]
fn icon_button_respects_focus_from_generic_decoration(cx: &mut TestAppContext) {
    assert_external_focus(cx, true);
}

#[gpui::test]
fn external_focus_preserves_a_non_tab_stop_handle(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    for icon in [false, true] {
        let (_, cx) = cx.add_window_view(|_, cx| FocusHost {
            handle: cx.focus_handle(),
            icon,
            stop: false,
        });
        draw(cx);
        cx.update(|window, _| window.focus_next());
        assert!(cx.update(|window, cx| window.focused(cx)).is_none());
    }
}

struct TabPolicyHost {
    handle: FocusHandle,
    icon: bool,
    disabled: bool,
    loading: bool,
    stop: bool,
}
impl Render for TabPolicyHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        use gpui::InteractiveElement as _;
        let button = if self.icon {
            IconButton::new("plus")
                .disabled(self.disabled)
                .track_focus(&self.handle)
                .tab_index(7)
                .tab_stop(self.stop)
                .into_any_element()
        } else {
            Button::new("tab-policy", "Policy")
                .disabled(self.disabled)
                .loading(self.loading)
                .track_focus(&self.handle)
                .tab_index(7)
                .tab_stop(self.stop)
                .into_any_element()
        };
        div().child(button)
    }
}

#[gpui::test]
fn trait_tab_stop_and_disabled_states_are_preserved(cx: &mut TestAppContext) {
    cx.update(fc_ui::init);
    for (icon, disabled, loading) in [
        (false, false, false),
        (true, false, false),
        (false, true, false),
        (false, false, true),
        (true, true, false),
    ] {
        let (_, cx) = cx.add_window_view(|_, cx| TabPolicyHost {
            handle: cx.focus_handle().tab_stop(true),
            icon,
            disabled,
            loading,
            stop: disabled || loading,
        });
        draw(cx);
        cx.update(|window, _| window.focus_next());
        assert!(cx.update(|window, cx| window.focused(cx)).is_none());
    }
}
