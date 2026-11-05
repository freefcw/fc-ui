use crate::capabilities::controls::slider::{
    bounds_canvas, filled_track, focus_shell, horizontal_drag_percentage, keyboard_value,
    slider_thumb, snap_to_step, value_label, value_percentage, vertical_drag_percentage,
    with_drag_handlers, SliderAxis, SliderDrag, SliderPaint, SliderSize, TrackBounds, TrackSpec,
};
use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::*, *};
use std::rc::Rc;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ActiveThumb {
    None,
    Start,
    End,
}

pub struct RangeSliderState {
    min: f32,
    max: f32,
    start_value: f32,
    end_value: f32,
    step: f32,
    focus_handle: FocusHandle,
    active_thumb: ActiveThumb,
    bounds: Bounds<Pixels>,
}

impl RangeSliderState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            start_value: 25.0,
            end_value: 75.0,
            step: 1.0,
            focus_handle: cx.focus_handle(),
            active_thumb: ActiveThumb::None,
            bounds: Bounds::default(),
        }
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    /// Set the minimum. If `min` exceeds the current maximum, the maximum is
    /// raised to `min`, so the ordering `min <= max` is always preserved. Both
    /// thumbs are then re-clamped into range with `start <= end` preserved.
    pub fn set_min(&mut self, min: f32, cx: &mut Context<Self>) {
        self.min = min;
        self.max = self.max.max(self.min);
        self.start_value = self.start_value.clamp(self.min, self.max);
        self.end_value = self.end_value.clamp(self.start_value, self.max);
        cx.notify();
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    /// Set the maximum. If `max` is below the current minimum, the minimum is
    /// lowered to `max`, so the ordering `min <= max` is always preserved. Both
    /// thumbs are then re-clamped into range with `start <= end` preserved.
    pub fn set_max(&mut self, max: f32, cx: &mut Context<Self>) {
        self.max = max;
        self.min = self.min.min(self.max);
        self.end_value = self.end_value.clamp(self.min, self.max);
        self.start_value = self.start_value.clamp(self.min, self.end_value);
        cx.notify();
    }

    pub fn start_value(&self) -> f32 {
        self.start_value
    }

    pub fn end_value(&self) -> f32 {
        self.end_value
    }

    pub fn range(&self) -> (f32, f32) {
        (self.start_value, self.end_value)
    }

    pub fn set_start_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let stepped = snap_to_step(value, self.step, self.min, self.end_value);

        if (self.start_value - stepped).abs() > f32::EPSILON {
            self.start_value = stepped;
            cx.notify();
        }
    }

    pub fn set_end_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let stepped = snap_to_step(value, self.step, self.start_value, self.max);

        if (self.end_value - stepped).abs() > f32::EPSILON {
            self.end_value = stepped;
            cx.notify();
        }
    }

    pub fn set_range(&mut self, start: f32, end: f32, cx: &mut Context<Self>) {
        let clamped_start = start.clamp(self.min, self.max);
        let clamped_end = end.clamp(clamped_start, self.max);

        let stepped_start = snap_to_step(clamped_start, self.step, self.min, self.max);
        let stepped_end = snap_to_step(clamped_end, self.step, stepped_start, self.max);

        let changed = (self.start_value - stepped_start).abs() > f32::EPSILON
            || (self.end_value - stepped_end).abs() > f32::EPSILON;

        if changed {
            self.start_value = stepped_start;
            self.end_value = stepped_end;
            cx.notify();
        }
    }

    pub fn step(&self) -> f32 {
        self.step
    }

    pub fn set_step(&mut self, step: f32, cx: &mut Context<Self>) {
        self.step = step;
        cx.notify();
    }

    fn start_percentage(&self) -> f32 {
        value_percentage(self.start_value, self.min, self.max)
    }

    fn end_percentage(&self) -> f32 {
        value_percentage(self.end_value, self.min, self.max)
    }

    /// Move whichever thumb is active to `new_value`; with no active thumb,
    /// the nearer thumb becomes active (ties favor the start thumb).
    fn apply_to_active_thumb(&mut self, new_value: f32, cx: &mut Context<Self>) {
        match self.active_thumb {
            ActiveThumb::Start => self.set_start_value(new_value, cx),
            ActiveThumb::End => self.set_end_value(new_value, cx),
            ActiveThumb::None => {
                let start_dist = (new_value - self.start_value).abs();
                let end_dist = (new_value - self.end_value).abs();

                if start_dist <= end_dist {
                    self.active_thumb = ActiveThumb::Start;
                    self.set_start_value(new_value, cx);
                } else {
                    self.active_thumb = ActiveThumb::End;
                    self.set_end_value(new_value, cx);
                }
            }
        }
    }

    /// Map a pointer `position` on the horizontal/vertical track to a value
    /// and move the active thumb there.
    fn update_at(&mut self, position: Point<Pixels>, vertical: bool, cx: &mut Context<Self>) {
        let percentage = if vertical {
            vertical_drag_percentage(self.bounds, position)
        } else {
            horizontal_drag_percentage(self.bounds, position)
        };
        let new_value = percentage.map_or(self.min, |percentage| {
            self.min + percentage * (self.max - self.min)
        });
        self.apply_to_active_thumb(new_value, cx);
    }
}

impl TrackBounds for RangeSliderState {
    fn set_track_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

impl SliderDrag for RangeSliderState {
    fn begin_drag(&mut self, position: Point<Pixels>, vertical: bool, cx: &mut Context<Self>) {
        // No active thumb yet: the nearest one is picked.
        self.update_at(position, vertical, cx);
    }

    fn drag_move(
        &mut self,
        position: Point<Pixels>,
        vertical: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active_thumb == ActiveThumb::None {
            return false;
        }
        self.update_at(position, vertical, cx);
        true
    }

    fn end_drag(&mut self) {
        self.active_thumb = ActiveThumb::None;
    }
}

impl Focusable for RangeSliderState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for RangeSliderState {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(IntoElement)]
pub struct RangeSlider {
    state: Entity<RangeSliderState>,
    size: SliderSize,
    axis: SliderAxis,
    disabled: bool,
    show_values: bool,
    on_change: Option<Rc<dyn Fn(f32, f32, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl RangeSlider {
    pub fn new(state: Entity<RangeSliderState>) -> Self {
        Self {
            state,
            size: SliderSize::Md,
            axis: SliderAxis::Horizontal,
            disabled: false,
            show_values: false,
            on_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: SliderSize) -> Self {
        self.size = size;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.axis = SliderAxis::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.axis = SliderAxis::Vertical;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(f32, f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for RangeSlider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RangeSlider {
    fn key_down_listener(
        &self,
        window: &mut Window,
    ) -> Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static> {
        let on_change = self.on_change.clone();
        Box::new(
            window.listener_for(&self.state, move |state, e: &KeyDownEvent, window, cx| {
                // Keys move the most recently used thumb, defaulting to start.
                let use_start = !matches!(state.active_thumb, ActiveThumb::End);
                let active_value = if use_start {
                    state.start_value
                } else {
                    state.end_value
                };
                let Some(new_value) = keyboard_value(
                    e.keystroke.key.as_str(),
                    active_value,
                    state.step,
                    state.min,
                    state.max,
                ) else {
                    return;
                };
                if use_start {
                    state.set_start_value(new_value, cx);
                } else {
                    state.set_end_value(new_value, cx);
                }
                cx.stop_propagation();
                if let Some(ref handler) = on_change {
                    handler(state.start_value, state.end_value, window, cx);
                }
            }),
        )
    }

    fn thumb_press_listener(
        &self,
        window: &mut Window,
        thumb: ActiveThumb,
        vertical: bool,
    ) -> Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static> {
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        Box::new(
            window.listener_for(&state, move |state, e: &MouseDownEvent, window, cx| {
                state.active_thumb = thumb;
                state.update_at(e.position, vertical, cx);
                if let Some(ref handler) = on_change {
                    handler(state.start_value, state.end_value, window, cx);
                }
                cx.stop_propagation();
            }),
        )
    }

    fn emit_change(
        &self,
    ) -> impl Fn(&mut RangeSliderState, &mut Window, &mut App) + Clone + 'static {
        let on_change = self.on_change.clone();
        move |state, window, cx| {
            if let Some(ref handler) = on_change {
                handler(state.start_value, state.end_value, window, cx);
            }
        }
    }

    /// Render the range slider for either axis from the shared paint inputs.
    /// Horizontal places the start label before the track and the end label
    /// after it; vertical mirrors that top-to-bottom.
    fn render_range(
        self,
        window: &mut Window,
        paint: SliderPaint,
        spec: TrackSpec,
        start_percentage: f32,
        end_percentage: f32,
        start_value: f32,
        end_value: f32,
    ) -> Div {
        let axis = self.axis;
        let vertical = matches!(axis, SliderAxis::Vertical);

        let shell = focus_shell(
            axis,
            &spec,
            &paint,
            self.disabled,
            self.key_down_listener(window),
        )
        .child(bounds_canvas(&self.state))
        .child(filled_track(axis, &spec, start_percentage, end_percentage))
        .child(slider_thumb(
            axis,
            &spec,
            &paint.theme,
            start_percentage,
            self.disabled,
            self.thumb_press_listener(window, ActiveThumb::Start, vertical),
        ))
        .child(slider_thumb(
            axis,
            &spec,
            &paint.theme,
            end_percentage,
            self.disabled,
            self.thumb_press_listener(window, ActiveThumb::End, vertical),
        ));
        let shell = with_drag_handlers(
            shell,
            window,
            &self.state,
            axis,
            self.disabled,
            self.emit_change(),
        );

        let container = div()
            .flex()
            .items_center()
            .gap_3()
            .map(|container| match axis {
                SliderAxis::Horizontal => container.w_full(),
                SliderAxis::Vertical => container.flex_col().h_full(),
            })
            .map(|container| {
                let mut div = container;
                div.style().refine(&paint.user_style);
                div
            });

        let (label_before, label_after) = match axis {
            SliderAxis::Horizontal => (start_value, end_value),
            SliderAxis::Vertical => (end_value, start_value),
        };

        container
            .when(self.show_values, |this| {
                this.child(value_label(axis, &paint.theme, label_before))
            })
            .child(shell)
            .when(self.show_values, |this| {
                this.child(value_label(axis, &paint.theme, label_after))
            })
    }
}

impl RenderOnce for RangeSlider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let state = self.state.read(cx);
        let focus_handle = state.focus_handle(cx);
        let is_focused = focus_handle.is_focused(window);
        let start_percentage = state.start_percentage();
        let end_percentage = state.end_percentage();
        let start_value = state.start_value;
        let end_value = state.end_value;

        let spec = TrackSpec::resolve(self.size, self.disabled, &theme);
        let paint = SliderPaint {
            theme,
            focus_handle,
            is_focused,
            user_style: self.style.clone(),
        };

        self.render_range(
            window,
            paint,
            spec,
            start_percentage,
            end_percentage,
            start_value,
            end_value,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn set_min_above_current_max_preserves_range_ordering() {
        let mut app = TestApp::new();
        let slider = app.update(|cx| cx.new(RangeSliderState::new));

        app.update(|cx| {
            slider.update(cx, |state, cx| {
                state.set_range(40.0, 60.0, cx);
                // 修复前：clamp(min, max) 因 min(110) > max(100) panic
                state.set_min(110.0, cx);
                assert!(state.min() <= state.max(), "min must never exceed max");
                let (start, end) = state.range();
                assert!(start <= end, "start {start} must not exceed end {end}");
                assert!(state.min() <= start && end <= state.max());
                assert_eq!((start, end), (110.0, 110.0));
            });
        });
    }

    #[gpui::test]
    fn set_max_below_current_min_preserves_range_ordering() {
        let mut app = TestApp::new();
        let slider = app.update(|cx| cx.new(RangeSliderState::new));

        app.update(|cx| {
            slider.update(cx, |state, cx| {
                state.set_range(40.0, 60.0, cx);
                // 修复前：end_value.clamp(min, max) 因 max(-20) < min(0) panic
                state.set_max(-20.0, cx);
                assert!(state.min() <= state.max(), "min must never exceed max");
                let (start, end) = state.range();
                assert!(start <= end, "start {start} must not exceed end {end}");
                assert_eq!(state.min(), -20.0);
                assert_eq!((start, end), (-20.0, -20.0));
            });
        });
    }

    #[gpui::test]
    fn pressing_track_picks_the_nearest_thumb_and_follows_drags() {
        let mut app = TestApp::new();
        let slider = app.update(|cx| cx.new(RangeSliderState::new));

        app.update(|cx| {
            slider.update(cx, |state, cx| {
                // Give the track a 0..=100px horizontal bounds so pointer
                // positions map to percentages.
                state.set_track_bounds(Bounds::new(
                    Point::new(px(0.0), px(0.0)),
                    size(px(100.0), px(20.0)),
                ));

                // A press at 90% of 0..=100 is nearest the end thumb (75).
                state.begin_drag(Point::new(px(90.0), px(0.0)), false, cx);
                assert_eq!(state.end_value(), 90.0);

                // Moving applies to the now-active end thumb only.
                state.drag_move(Point::new(px(60.0), px(0.0)), false, cx);
                assert_eq!(state.end_value(), 60.0);
                assert_eq!(state.start_value(), 25.0);

                // After release, moves no longer apply.
                state.end_drag();
                assert!(!state.drag_move(Point::new(px(10.0), px(0.0)), false, cx));
                assert_eq!(state.start_value(), 25.0);
            });
        });
    }
}
