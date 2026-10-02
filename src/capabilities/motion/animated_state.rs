use gpui::*;
use std::time::Duration;

use crate::capabilities::motion::animations::{easings, lerp_color, lerp_f32};

const DEFAULT_TRANSITION: Duration = Duration::from_millis(150);

#[derive(Clone, Debug)]
pub struct AnimatedInteraction {
    hover_progress: f32,
    press_progress: f32,
    focus_progress: f32,
    hover_target: f32,
    press_target: f32,
    focus_target: f32,
    hover_from: f32,
    press_from: f32,
    focus_from: f32,
    hover_elapsed: f32,
    press_elapsed: f32,
    focus_elapsed: f32,
    transition_duration: Duration,
    easing: fn(f32) -> f32,
}

impl Default for AnimatedInteraction {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimatedInteraction {
    pub fn new() -> Self {
        Self {
            hover_progress: 0.0,
            press_progress: 0.0,
            focus_progress: 0.0,
            hover_target: 0.0,
            press_target: 0.0,
            focus_target: 0.0,
            hover_from: 0.0,
            press_from: 0.0,
            focus_from: 0.0,
            hover_elapsed: 0.0,
            press_elapsed: 0.0,
            focus_elapsed: 0.0,
            transition_duration: DEFAULT_TRANSITION,
            easing: easings::ease_out_cubic,
        }
    }

    /// Duration of a full transition from one target to the other. Ticking
    /// accumulates elapsed time and drives each progress value linearly from
    /// its value at the last target change to the new target over exactly
    /// this duration.
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.transition_duration = duration;
        self
    }

    pub fn with_easing(mut self, easing: fn(f32) -> f32) -> Self {
        self.easing = easing;
        self
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        Self::retarget(
            &mut self.hover_progress,
            &mut self.hover_target,
            &mut self.hover_from,
            &mut self.hover_elapsed,
            if hovered { 1.0 } else { 0.0 },
        );
    }

    pub fn set_pressed(&mut self, pressed: bool) {
        Self::retarget(
            &mut self.press_progress,
            &mut self.press_target,
            &mut self.press_from,
            &mut self.press_elapsed,
            if pressed { 1.0 } else { 0.0 },
        );
    }

    pub fn set_focused(&mut self, focused: bool) {
        Self::retarget(
            &mut self.focus_progress,
            &mut self.focus_target,
            &mut self.focus_from,
            &mut self.focus_elapsed,
            if focused { 1.0 } else { 0.0 },
        );
    }

    fn retarget(
        progress: &mut f32,
        target: &mut f32,
        from: &mut f32,
        elapsed: &mut f32,
        new_target: f32,
    ) {
        if new_target != *target {
            *target = new_target;
            *from = *progress;
            *elapsed = 0.0;
        }
    }

    /// Advance all channels by `dt` seconds. Returns `true` while any channel
    /// is still moving toward its target. Each channel completes exactly
    /// `transition_duration` of accumulated `dt` after its target changed.
    pub fn tick(&mut self, dt: f32) -> bool {
        let duration = self.transition_duration.as_secs_f32();
        let mut changed = false;

        changed |= Self::advance(
            &mut self.hover_progress,
            self.hover_from,
            self.hover_target,
            &mut self.hover_elapsed,
            dt,
            duration,
        );
        changed |= Self::advance(
            &mut self.press_progress,
            self.press_from,
            self.press_target,
            &mut self.press_elapsed,
            dt,
            duration,
        );
        changed |= Self::advance(
            &mut self.focus_progress,
            self.focus_from,
            self.focus_target,
            &mut self.focus_elapsed,
            dt,
            duration,
        );

        changed
    }

    fn advance(
        progress: &mut f32,
        from: f32,
        target: f32,
        elapsed: &mut f32,
        dt: f32,
        duration: f32,
    ) -> bool {
        if *progress == target {
            return false;
        }
        *elapsed += dt;
        if duration <= 0.0 || *elapsed >= duration {
            *progress = target;
        } else {
            let t = (*elapsed / duration).clamp(0.0, 1.0);
            *progress = from + (target - from) * t;
        }
        true
    }

    pub fn hover(&self) -> f32 {
        (self.easing)(self.hover_progress)
    }

    pub fn press(&self) -> f32 {
        (self.easing)(self.press_progress)
    }

    pub fn focus(&self) -> f32 {
        (self.easing)(self.focus_progress)
    }

    pub fn is_animating(&self) -> bool {
        (self.hover_progress - self.hover_target).abs() > 0.001
            || (self.press_progress - self.press_target).abs() > 0.001
            || (self.focus_progress - self.focus_target).abs() > 0.001
    }

    pub fn blend_color(&self, normal: Hsla, hover: Hsla, press: Hsla) -> Hsla {
        let base = lerp_color(normal, hover, self.hover());
        lerp_color(base, press, self.press())
    }

    pub fn blend_f32(&self, normal: f32, hover: f32, press: f32) -> f32 {
        let base = lerp_f32(normal, hover, self.hover());
        lerp_f32(base, press, self.press())
    }
}
