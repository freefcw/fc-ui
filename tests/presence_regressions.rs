use std::time::Duration;

use fc_ui::{
    components::animated_presence::{AnimatedPresence, AnimatedPresenceState},
    overlays::toast::{ToastItem, ToastManager},
};
use gpui::{
    AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, TestAppContext,
    VisualTestContext, Window,
};

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear());
}

struct PresenceHost {
    state: Entity<AnimatedPresenceState>,
    show: Option<bool>,
    duration: Duration,
}
impl Render for PresenceHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let presence = AnimatedPresence::new("presence", self.state.clone())
            .exit_duration(self.duration)
            .child("content");
        if let Some(show) = self.show {
            presence.show(show)
        } else {
            presence
        }
    }
}

#[gpui::test]
fn direct_presence_hide_uses_the_elements_exit_duration(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|_, cx| PresenceHost {
        state: cx.new(|_| AnimatedPresenceState::new()),
        show: None,
        duration: Duration::from_secs(1),
    });
    let state = host.read_with(cx, |host, _| host.state.clone());
    state.update(cx, |state, cx| state.set_visible(true, cx));
    draw(cx);
    state.update(cx, |state, cx| state.set_visible(false, cx));
    draw(cx);
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.is_visible()
        && state.is_animating_out()));
    draw(cx);
    cx.executor().advance_clock(Duration::from_millis(650));
    cx.run_until_parked();
    assert!(!state.read_with(cx, |state, _| state.is_visible()));
}

#[gpui::test]
fn presence_exit_deadline_survives_a_delayed_first_task_poll(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|_, cx| PresenceHost {
        state: cx.new(|_| AnimatedPresenceState::new()),
        show: None,
        duration: Duration::from_secs(1),
    });
    let state = host.read_with(cx, |host, _| host.state.clone());
    state.update(cx, |state, cx| state.set_visible(true, cx));
    draw(cx);
    let _clock_marker = cx.executor().timer(Duration::from_millis(400));
    state.update(cx, |state, cx| state.set_visible(false, cx));

    // Unlike advance_clock, this moves fake time without draining ready tasks.
    // The spawned exit task therefore first runs 400ms after visibility changed.
    assert!(cx.dispatcher.advance_clock_to_next_delayed());
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.is_animating_out()));
    cx.executor().advance_clock(Duration::from_millis(650));
    cx.run_until_parked();
    assert!(
        !state.read_with(cx, |state, _| state.is_visible()),
        "the exit ends one second after the visibility change, rather than after its task is first polled"
    );
}

#[gpui::test]
fn declarative_show_cancels_an_exit_and_its_timer(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|_, cx| PresenceHost {
        state: cx.new(|_| AnimatedPresenceState::new()),
        show: Some(true),
        duration: Duration::from_secs(1),
    });
    draw(cx);
    let state = host.read_with(cx, |host, _| host.state.clone());
    host.update(cx, |host, cx| {
        host.show = Some(false);
        cx.notify();
    });
    draw(cx);
    host.update(cx, |host, cx| {
        host.show = Some(true);
        cx.notify();
    });
    draw(cx);
    assert!(state.read_with(cx, |state, _| state.is_visible()
        && !state.is_animating_out()));
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_secs(2));
    cx.run_until_parked();
    assert!(state.read_with(cx, |state, _| state.is_visible()));
}

#[gpui::test]
fn declarative_exit_finishes_once_without_restarting_on_redraw(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|_, cx| PresenceHost {
        state: cx.new(|_| AnimatedPresenceState::new()),
        show: Some(true),
        duration: Duration::from_secs(1),
    });
    draw(cx);
    let state = host.read_with(cx, |host, _| host.state.clone());
    host.update(cx, |host, cx| {
        host.show = Some(false);
        cx.notify();
    });
    draw(cx);
    cx.run_until_parked();
    for _ in 0..3 {
        cx.executor().advance_clock(Duration::from_millis(300));
        cx.run_until_parked();
        host.update(cx, |_, cx| cx.notify());
        draw(cx);
        assert!(state.read_with(cx, |state, _| state.is_visible()
            && state.is_animating_out()));
    }
    cx.executor().advance_clock(Duration::from_millis(110));
    cx.run_until_parked();
    assert!(!state.read_with(cx, |state, _| state.is_visible()));
    draw(cx);
    assert!(!state.read_with(cx, |state, _| state.is_animating_out()));
}

#[gpui::test]
fn changing_exit_duration_keeps_the_original_transition_start(cx: &mut TestAppContext) {
    let (host, cx) = cx.add_window_view(|_, cx| PresenceHost {
        state: cx.new(|_| AnimatedPresenceState::new()),
        show: None,
        duration: Duration::from_secs(2),
    });
    let state = host.read_with(cx, |host, _| host.state.clone());
    state.update(cx, |state, cx| state.set_visible(true, cx));
    draw(cx);
    state.update(cx, |state, cx| state.set_visible(false, cx));
    draw(cx);
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    host.update(cx, |host, cx| {
        host.duration = Duration::from_secs(1);
        cx.notify();
    });
    draw(cx);
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(650));
    cx.run_until_parked();
    assert!(
        !state.read_with(cx, |state, _| state.is_visible()),
        "changing duration updates the original deadline rather than starting another full second"
    );
}

#[gpui::test]
fn zero_toast_capacity_still_accepts_toasts(cx: &mut TestAppContext) {
    let manager = cx.new(|cx| ToastManager::new(cx).max_toasts(0));
    manager.update(cx, |manager, cx| {
        manager.add_toast_no_dismiss(ToastItem::new(0, "first"), cx);
        manager.add_toast_no_dismiss(ToastItem::new(0, "second"), cx);
    });
}
