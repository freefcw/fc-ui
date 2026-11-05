use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::*, *};
use std::rc::Rc;

/// Shared slider math used by both `Slider` and `RangeSlider`.
///
/// Normalized 0..=1 track position of `value` between `min` and `max`.
pub(crate) fn value_percentage(value: f32, min: f32, max: f32) -> f32 {
    if max == min {
        return 0.0;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

/// Clamp `value` into `lo..=hi`, snap it to the nearest `step` multiple, and
/// clamp again so the result stays in range.
pub(crate) fn snap_to_step(value: f32, step: f32, lo: f32, hi: f32) -> f32 {
    ((value.clamp(lo, hi) / step).round() * step).clamp(lo, hi)
}

/// Track-relative drag percentage for a horizontal slider. `None` when the
/// track has no width yet.
pub(crate) fn horizontal_drag_percentage(
    bounds: Bounds<Pixels>,
    position: Point<Pixels>,
) -> Option<f32> {
    let width = bounds.size.width;
    if width <= px(0.0) {
        return None;
    }
    let relative_x = (position.x - bounds.left()).clamp(px(0.0), width);
    Some((relative_x / width).clamp(0.0, 1.0))
}

/// Track-relative drag percentage for a vertical slider (0 at the bottom).
/// `None` when the track has no height yet.
pub(crate) fn vertical_drag_percentage(
    bounds: Bounds<Pixels>,
    position: Point<Pixels>,
) -> Option<f32> {
    let height = bounds.size.height;
    if height <= px(0.0) {
        return None;
    }
    let relative_y = (position.y - bounds.top()).clamp(px(0.0), height);
    Some(1.0 - (relative_y / height).clamp(0.0, 1.0))
}

/// New (unclamped) value for a slider navigation key, or `None` when the key
/// is not a slider key. Callers pass the result through their clamping
/// setters.
pub(crate) fn keyboard_value(key: &str, value: f32, step: f32, min: f32, max: f32) -> Option<f32> {
    match key {
        "left" | "down" => Some(value - step),
        "right" | "up" => Some(value + step),
        "home" => Some(min),
        "end" => Some(max),
        _ => None,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SliderSize {
    Sm,
    Md,
    Lg,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SliderAxis {
    Horizontal,
    Vertical,
}

impl SliderSize {
    pub fn track_height(&self) -> Pixels {
        match self {
            SliderSize::Sm => px(2.0),
            SliderSize::Md => px(4.0),
            SliderSize::Lg => px(6.0),
        }
    }

    pub fn thumb_width(&self) -> Pixels {
        match self {
            SliderSize::Sm => px(16.0),
            SliderSize::Md => px(20.0),
            SliderSize::Lg => px(24.0),
        }
    }

    pub fn thumb_height(&self) -> Pixels {
        match self {
            SliderSize::Sm => px(12.0),
            SliderSize::Md => px(16.0),
            SliderSize::Lg => px(20.0),
        }
    }
}

/// Theme and focus passthrough inputs shared by the slider track painters.
pub(crate) struct SliderPaint {
    pub(crate) theme: crate::capabilities::foundation::theme::Theme,
    pub(crate) focus_handle: FocusHandle,
    pub(crate) is_focused: bool,
    pub(crate) user_style: StyleRefinement,
}

/// Track and thumb geometry/colors for one slider render.
pub(crate) struct TrackSpec {
    pub(crate) track_height: Pixels,
    pub(crate) thumb_width: Pixels,
    pub(crate) thumb_height: Pixels,
    pub(crate) track_bg: Hsla,
    pub(crate) active_bg: Hsla,
    pub(crate) thumb_bg: Hsla,
    pub(crate) focus_ring: BoxShadow,
}

impl TrackSpec {
    /// Resolve the track/thumb geometry and colors (including the disabled
    /// translucency) shared by `Slider` and `RangeSlider`.
    pub(crate) fn resolve(
        size: SliderSize,
        disabled: bool,
        theme: &crate::capabilities::foundation::theme::Theme,
    ) -> Self {
        let (track_bg, active_bg, thumb_bg) = if disabled {
            (
                theme.tokens.muted.opacity(0.3),
                theme.tokens.primary.opacity(0.3),
                theme.tokens.primary.opacity(0.3),
            )
        } else {
            (
                theme.tokens.muted,
                theme.tokens.primary,
                theme.tokens.primary,
            )
        };

        Self {
            track_height: size.track_height(),
            thumb_width: size.thumb_width(),
            thumb_height: size.thumb_height(),
            track_bg,
            active_bg,
            thumb_bg,
            focus_ring: theme.tokens.focus_ring_light(),
        }
    }
}

/// Slider states whose track bounds are captured by the shared
/// [`bounds_canvas`] reporter.
pub(crate) trait TrackBounds {
    fn set_track_bounds(&mut self, bounds: Bounds<Pixels>);
}

/// Pointer-driven dragging shared by `SliderState` and `RangeSliderState`;
/// the shared drag handlers in [`with_drag_handlers`] drive it.
pub(crate) trait SliderDrag: TrackBounds + 'static {
    /// A thumb or the track was pressed at `position`: begin dragging and
    /// move the value there.
    fn begin_drag(&mut self, position: Point<Pixels>, vertical: bool, cx: &mut Context<Self>)
    where
        Self: Sized;
    /// The drag moved to `position`; returns whether the value changed.
    fn drag_move(
        &mut self,
        position: Point<Pixels>,
        vertical: bool,
        cx: &mut Context<Self>,
    ) -> bool
    where
        Self: Sized;
    /// The drag ended (button released inside or outside the track).
    fn end_drag(&mut self);
}

/// Absolute, size-tracking canvas reporting the track bounds to the state.
pub(crate) fn bounds_canvas<E: TrackBounds + 'static>(entity: &Entity<E>) -> Canvas<()> {
    let entity = entity.clone();
    canvas(
        move |bounds, _, cx| {
            entity.update(cx, |state, _| state.set_track_bounds(bounds));
        },
        |_, _, _, _| {},
    )
    .absolute()
    .size_full()
}

/// The rounded track bar with the active fill spanning
/// `start_percentage..=end_percentage` (single thumbs pass `0.0` as the
/// start).
pub(crate) fn filled_track(
    axis: SliderAxis,
    spec: &TrackSpec,
    start_percentage: f32,
    end_percentage: f32,
) -> Div {
    let span = end_percentage - start_percentage;
    div()
        .relative()
        .map(|track| match axis {
            SliderAxis::Horizontal => track.w_full().h(spec.track_height),
            SliderAxis::Vertical => track.h_full().w(spec.track_height),
        })
        .rounded_full()
        .bg(spec.track_bg)
        .overflow_hidden()
        .child(match axis {
            SliderAxis::Horizontal => div()
                .absolute()
                .left(relative(start_percentage))
                .top_0()
                .h_full()
                .w(relative(span))
                .bg(spec.active_bg),
            SliderAxis::Vertical => div()
                .absolute()
                .left_0()
                .bottom(relative(start_percentage))
                .w_full()
                .h(relative(span))
                .bg(spec.active_bg),
        })
}

/// One draggable thumb at `percentage` along the track. `on_press` is only
/// attached when enabled.
pub(crate) fn slider_thumb(
    axis: SliderAxis,
    spec: &TrackSpec,
    theme: &crate::capabilities::foundation::theme::Theme,
    percentage: f32,
    disabled: bool,
    on_press: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .absolute()
        .map(|thumb| match axis {
            SliderAxis::Horizontal => thumb
                .left(relative(percentage))
                .top_0()
                .ml(-(spec.thumb_width / 2.0))
                .rounded(spec.thumb_height / 2.0),
            SliderAxis::Vertical => thumb
                .left_0()
                .bottom(relative(percentage))
                .mb(-(spec.thumb_height / 2.0))
                .rounded(spec.thumb_width / 2.0),
        })
        .w(spec.thumb_width)
        .h(spec.thumb_height)
        .bg(spec.thumb_bg)
        .border_2()
        .border_color(theme.tokens.background)
        .when(!disabled, |this| {
            this.shadow(smallvec::smallvec![theme.tokens.shadow_sm.clone()])
                .cursor(CursorStyle::PointingHand)
        })
        .when(!disabled, |this| {
            this.on_mouse_down(MouseButton::Left, on_press)
        })
}

/// The focusable shell wrapping track and thumbs: focus tracking, focus ring,
/// corner radius and the keyboard listener.
pub(crate) fn focus_shell(
    axis: SliderAxis,
    spec: &TrackSpec,
    paint: &SliderPaint,
    disabled: bool,
    on_key_down: impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
) -> Div {
    div()
        .relative()
        .flex_1()
        .map(|shell| match axis {
            SliderAxis::Horizontal => shell.h(spec.thumb_height),
            SliderAxis::Vertical => shell.w(spec.thumb_width).justify_center(),
        })
        .flex()
        .items_center()
        .when(!disabled, |this| {
            let handle = paint.focus_handle.clone().tab_index(0).tab_stop(true);
            this.track_focus(&handle)
        })
        .when(paint.is_focused && !disabled, |this| {
            this.shadow(smallvec::smallvec![spec.focus_ring.clone()])
        })
        .rounded(paint.theme.tokens.radius_md)
        .when(!disabled, |this| this.on_key_down(on_key_down))
}

/// Attach the shared press/move/release drag wiring to the focus shell.
/// `emit` runs after each applied change (both press and move).
pub(crate) fn with_drag_handlers<E, F>(
    this: Div,
    window: &mut Window,
    entity: &Entity<E>,
    axis: SliderAxis,
    disabled: bool,
    emit: F,
) -> Div
where
    E: SliderDrag + Render,
    F: Fn(&mut E, &mut Window, &mut App) + Clone + 'static,
{
    if disabled {
        return this;
    }
    let vertical = matches!(axis, SliderAxis::Vertical);
    let entity_down = entity.clone();
    let emit_down = emit.clone();
    let entity_move = entity.clone();
    let emit_move = emit;
    let entity_up = entity.clone();
    let entity_up_out = entity.clone();

    this.on_mouse_down(
        MouseButton::Left,
        window.listener_for(
            &entity_down,
            move |state, e: &MouseDownEvent, window, cx| {
                state.begin_drag(e.position, vertical, cx);
                emit_down(state, window, cx);
            },
        ),
    )
    .on_mouse_move(window.listener_for(
        &entity_move,
        move |state, e: &MouseMoveEvent, window, cx| {
            if state.drag_move(e.position, vertical, cx) {
                emit_move(state, window, cx);
            }
        },
    ))
    .on_mouse_up(
        MouseButton::Left,
        window.listener_for(&entity_up, move |state, _: &MouseUpEvent, _, _| {
            state.end_drag();
        }),
    )
    .on_mouse_up_out(
        MouseButton::Left,
        window.listener_for(&entity_up_out, move |state, _: &MouseUpEvent, _, _| {
            state.end_drag();
        }),
    )
}

/// The trailing value label shown when `show_value(s)` is enabled.
pub(crate) fn value_label(
    axis: SliderAxis,
    theme: &crate::capabilities::foundation::theme::Theme,
    value: f32,
) -> Div {
    div()
        .text_center()
        .text_sm()
        .text_color(theme.tokens.foreground)
        .map(|label| match axis {
            SliderAxis::Horizontal => label.min_w(px(40.0)),
            SliderAxis::Vertical => label.min_h(px(24.0)),
        })
        .child(format!("{:.0}", value))
}

pub struct SliderState {
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    focus_handle: FocusHandle,
    is_dragging: bool,
    bounds: Bounds<Pixels>,
}

impl SliderState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            value: 0.0,
            step: 1.0,
            focus_handle: cx.focus_handle(),
            is_dragging: false,
            bounds: Bounds::default(),
        }
    }

    pub fn min(&self) -> f32 {
        self.min
    }

    /// Set the minimum. If `min` exceeds the current maximum, the maximum is
    /// raised to `min`, so the ordering `min <= max` is always preserved.
    pub fn set_min(&mut self, min: f32, cx: &mut Context<Self>) {
        self.min = min;
        self.max = self.max.max(self.min);
        self.value = self.value.clamp(self.min, self.max);
        cx.notify();
    }

    pub fn max(&self) -> f32 {
        self.max
    }

    /// Set the maximum. If `max` is below the current minimum, the minimum is
    /// lowered to `max`, so the ordering `min <= max` is always preserved.
    pub fn set_max(&mut self, max: f32, cx: &mut Context<Self>) {
        self.max = max;
        self.min = self.min.min(self.max);
        self.value = self.value.clamp(self.min, self.max);
        cx.notify();
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let stepped = snap_to_step(value, self.step, self.min, self.max);

        if (self.value - stepped).abs() > f32::EPSILON {
            self.value = stepped;
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

    fn percentage(&self) -> f32 {
        value_percentage(self.value, self.min, self.max)
    }

    /// Move the value to `position` along the horizontal or vertical track.
    fn update_at(&mut self, position: Point<Pixels>, vertical: bool, cx: &mut Context<Self>) {
        let percentage = if vertical {
            vertical_drag_percentage(self.bounds, position)
        } else {
            horizontal_drag_percentage(self.bounds, position)
        };
        let Some(percentage) = percentage else {
            return;
        };
        self.set_value(self.min + percentage * (self.max - self.min), cx);
    }
}

impl TrackBounds for SliderState {
    fn set_track_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

impl SliderDrag for SliderState {
    fn begin_drag(&mut self, position: Point<Pixels>, vertical: bool, cx: &mut Context<Self>) {
        self.is_dragging = true;
        self.update_at(position, vertical, cx);
    }

    fn drag_move(
        &mut self,
        position: Point<Pixels>,
        vertical: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.is_dragging {
            return false;
        }
        self.update_at(position, vertical, cx);
        true
    }

    fn end_drag(&mut self) {
        self.is_dragging = false;
    }
}

impl Focusable for SliderState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SliderState {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[derive(IntoElement)]
pub struct Slider {
    state: Entity<SliderState>,
    size: SliderSize,
    axis: SliderAxis,
    disabled: bool,
    show_value: bool,
    on_change: Option<Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>>,
    style: StyleRefinement,
}

impl Slider {
    pub fn new(state: Entity<SliderState>) -> Self {
        Self {
            state,
            size: SliderSize::Md,
            axis: SliderAxis::Horizontal,
            disabled: false,
            show_value: false,
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

    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for Slider {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Slider {
    fn key_down_listener(
        &self,
        window: &mut Window,
    ) -> Box<dyn Fn(&KeyDownEvent, &mut Window, &mut App) + 'static> {
        let on_change = self.on_change.clone();
        Box::new(
            window.listener_for(&self.state, move |state, e: &KeyDownEvent, window, cx| {
                let Some(new_value) = keyboard_value(
                    e.keystroke.key.as_str(),
                    state.value,
                    state.step,
                    state.min,
                    state.max,
                ) else {
                    return;
                };
                state.set_value(new_value, cx);
                cx.stop_propagation();
                if let Some(ref handler) = on_change {
                    handler(state.value, window, cx);
                }
            }),
        )
    }

    fn thumb_press_listener(
        &self,
        window: &mut Window,
        vertical: bool,
    ) -> Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static> {
        let state = self.state.clone();
        let on_change = self.on_change.clone();
        Box::new(
            window.listener_for(&state, move |state, e: &MouseDownEvent, window, cx| {
                state.begin_drag(e.position, vertical, cx);
                if let Some(ref handler) = on_change {
                    handler(state.value, window, cx);
                }
                cx.stop_propagation();
            }),
        )
    }

    fn emit_change(&self) -> impl Fn(&mut SliderState, &mut Window, &mut App) + Clone + 'static {
        let on_change = self.on_change.clone();
        move |state, window, cx| {
            if let Some(ref handler) = on_change {
                handler(state.value, window, cx);
            }
        }
    }

    /// Render the slider for either axis from the shared paint inputs.
    fn render_slider(
        self,
        window: &mut Window,
        paint: SliderPaint,
        spec: TrackSpec,
        percentage: f32,
        value: f32,
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
        .child(filled_track(axis, &spec, 0.0, percentage))
        .child(slider_thumb(
            axis,
            &spec,
            &paint.theme,
            percentage,
            self.disabled,
            self.thumb_press_listener(window, vertical),
        ));
        let shell = with_drag_handlers(
            shell,
            window,
            &self.state,
            axis,
            self.disabled,
            self.emit_change(),
        );

        div()
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
            })
            .child(shell)
            .when(self.show_value, |this| {
                this.child(value_label(axis, &paint.theme, value))
            })
    }
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let state = self.state.read(cx);
        let focus_handle = state.focus_handle(cx);
        let is_focused = focus_handle.is_focused(window);
        let percentage = state.percentage();
        let value = state.value;

        let spec = TrackSpec::resolve(self.size, self.disabled, &theme);
        let paint = SliderPaint {
            theme,
            focus_handle,
            is_focused,
            user_style: self.style.clone(),
        };

        self.render_slider(window, paint, spec, percentage, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn set_min_above_current_max_preserves_ordering() {
        let mut app = TestApp::new();
        let slider = app.update(|cx| cx.new(SliderState::new));

        app.update(|cx| {
            slider.update(cx, |state, cx| {
                state.set_value(50.0, cx);
                // 修复前：max 仍是 100，value.clamp(min, max) 因 min > max 在 f32::clamp 内 panic
                state.set_min(120.0, cx);
                assert!(state.min() <= state.max(), "min must never exceed max");
                assert_eq!(state.max(), 120.0);
                assert_eq!(state.value(), 120.0, "value must clamp into the new range");
            });
        });
    }

    #[gpui::test]
    fn set_max_below_current_min_preserves_ordering() {
        let mut app = TestApp::new();
        let slider = app.update(|cx| cx.new(SliderState::new));

        app.update(|cx| {
            slider.update(cx, |state, cx| {
                state.set_value(50.0, cx);
                // 修复前：min 仍是 0，value.clamp(0, -10) 同样 panic
                state.set_max(-10.0, cx);
                assert!(state.min() <= state.max(), "min must never exceed max");
                assert_eq!(state.min(), -10.0);
                assert_eq!(state.value(), -10.0, "value must clamp into the new range");
            });
        });
    }
}
