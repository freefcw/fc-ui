//! Pure playback state and drag-tracking helpers shared by the audio and video
//! players.
//!
//! Both players model the same concepts — a playhead inside a duration, a
//! clamped volume with mute semantics, a remembered pre-mute volume and a
//! playback speed — so the clamp and derivation rules live here exactly once.
//! The players keep their public APIs (their own speed enums, and `f32` times
//! for audio vs `f64` for video) and delegate internally; float widths are
//! converted at the API boundary, which round-trips losslessly through `f32`.

use std::rc::Rc;

use gpui::*;

pub(crate) type PlaybackValueCallback<T> = Rc<dyn Fn(T, &mut Window, &mut App) + 'static>;

/// Shared playback state owned by [`crate::capabilities::media::audio_player`]
/// and [`crate::capabilities::media::video_player`].
pub(crate) struct PlaybackCore<S> {
    current_time: f64,
    duration: f64,
    volume: f32,
    muted: bool,
    previous_volume: f32,
    speed: S,
}

impl<S> PlaybackCore<S> {
    pub(crate) fn new(volume: f32, speed: S) -> Self {
        Self {
            current_time: 0.0,
            duration: 0.0,
            volume,
            muted: false,
            previous_volume: volume,
            speed,
        }
    }

    pub(crate) fn current_time(&self) -> f64 {
        self.current_time
    }

    /// Move the playhead, clamped into `0..=duration`.
    pub(crate) fn set_current_time(&mut self, time: f64) {
        self.current_time = time.clamp(0.0, self.duration);
    }

    pub(crate) fn duration(&self) -> f64 {
        self.duration
    }

    /// Set the duration, clamping negatives to zero and leaving the playhead
    /// untouched (the video player's semantics).
    pub(crate) fn set_duration(&mut self, duration: f64) {
        self.duration = duration.max(0.0);
    }

    /// Set the duration, rejecting negatives entirely and re-clamping the
    /// playhead into the new duration (the audio player's semantics). Returns
    /// whether the duration was applied.
    pub(crate) fn try_set_duration(&mut self, duration: f64) -> bool {
        if duration < 0.0 {
            return false;
        }
        self.duration = duration;
        self.current_time = self.current_time.min(duration);
        true
    }

    /// Playhead position as a fraction of the duration, clamped to `0..=1`.
    pub(crate) fn progress(&self) -> f64 {
        if self.duration <= 0.0 {
            return 0.0;
        }
        (self.current_time / self.duration).clamp(0.0, 1.0)
    }

    pub(crate) fn volume(&self) -> f32 {
        self.volume
    }

    /// Clamp `volume` into `0..=1`; a positive volume clears the mute flag.
    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if self.volume > 0.0 {
            self.muted = false;
        }
    }

    pub(crate) fn muted(&self) -> bool {
        self.muted
    }

    pub(crate) fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }

    /// The volume actually heard: zero while muted.
    pub(crate) fn effective_volume(&self) -> f32 {
        if self.muted {
            0.0
        } else {
            self.volume
        }
    }

    /// Toggle mute while remembering the pre-mute volume (the video player's
    /// semantics): unmuting restores it when the volume was dragged to zero in
    /// the meantime. The audio player's simpler flag flip goes through
    /// [`Self::set_muted`] instead, preserving its behaviour.
    pub(crate) fn toggle_mute_restoring_volume(&mut self) {
        if self.muted {
            self.muted = false;
            if self.volume == 0.0 {
                self.volume = self.previous_volume;
            }
        } else {
            self.previous_volume = self.volume;
            self.muted = true;
        }
    }

    pub(crate) fn speed(&self) -> &S {
        &self.speed
    }

    pub(crate) fn set_speed(&mut self, speed: S) {
        self.speed = speed;
    }
}

/// Convert a mouse position over a horizontal track into a `0..=1` fraction,
/// or `None` when the track has no width yet (nothing has been laid out), in
/// which case callers leave their state untouched.
pub(crate) fn track_fraction(position: Point<Pixels>, bounds: Bounds<Pixels>) -> Option<f32> {
    let track_width = bounds.size.width;
    if track_width <= px(0.0) {
        return None;
    }
    let relative_x = (position.x - bounds.left()).clamp(px(0.0), track_width);
    Some((relative_x / track_width).clamp(0.0, 1.0))
}

/// Speaker icon name for the given volume/mute state, one of `volume-x`,
/// `volume-1` or `volume-2`.
pub(crate) fn volume_icon(volume: f32, muted: bool) -> &'static str {
    if muted || volume == 0.0 {
        "volume-x"
    } else if volume < 0.5 {
        "volume-1"
    } else {
        "volume-2"
    }
}

/// Everything the shared slider mouse handlers need to drive one draggable
/// horizontal track (progress or volume) on a player state.
pub(crate) struct SliderDrag<T, V> {
    pub state: Entity<T>,
    pub on_change: Option<PlaybackValueCallback<V>>,
    pub is_dragging: fn(&T) -> bool,
    pub set_dragging: fn(&mut T, bool),
    pub apply_position: fn(&mut T, Point<Pixels>, &mut Context<T>),
    pub current_value: fn(&T) -> V,
}

/// Attach the shared down/move/up/up-out mouse handling for a draggable slider
/// track: pressing starts a drag and applies the position, moving applies it
/// while dragging, and any release ends the drag. After each applied position,
/// `on_change` (if set) fires with the track's current value.
pub(crate) fn attach_slider_drag<E, T, V>(
    element: E,
    window: &mut Window,
    drag: SliderDrag<T, V>,
) -> E
where
    E: InteractiveElement,
    T: Render,
    V: 'static,
{
    let SliderDrag {
        state,
        on_change,
        is_dragging,
        set_dragging,
        apply_position,
        current_value,
    } = drag;

    let state_down = state.clone();
    let on_change_down = on_change.clone();
    let state_move = state.clone();
    let on_change_move = on_change.clone();
    let state_release = state.clone();

    element
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(&state_down, move |state, e: &MouseDownEvent, window, cx| {
                set_dragging(state, true);
                apply_position(state, e.position, cx);
                if let Some(handler) = &on_change_down {
                    handler(current_value(state), window, cx);
                }
            }),
        )
        .on_mouse_move(window.listener_for(
            &state_move,
            move |state, e: &MouseMoveEvent, window, cx| {
                if is_dragging(state) {
                    apply_position(state, e.position, cx);
                    if let Some(handler) = &on_change_move {
                        handler(current_value(state), window, cx);
                    }
                }
            },
        ))
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(&state_release, move |state, _: &MouseUpEvent, _, _| {
                set_dragging(state, false);
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            window.listener_for(&state_release, move |state, _: &MouseUpEvent, _, _| {
                set_dragging(state, false);
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::{track_fraction, volume_icon, PlaybackCore};
    use gpui::{point, px, size, Bounds};

    #[test]
    fn positive_volume_unmutes_and_clamps() {
        let mut core = PlaybackCore::new(0.8, ());
        core.set_muted(true);
        core.set_volume(1.5);
        assert_eq!(core.volume(), 1.0);
        assert!(!core.muted());

        core.set_volume(-0.2);
        assert_eq!(core.volume(), 0.0);
        assert!(!core.muted(), "zero volume must not flip the mute flag");
        assert_eq!(core.effective_volume(), 0.0);
    }

    #[test]
    fn muted_volume_reports_zero_but_is_remembered() {
        let mut core = PlaybackCore::new(0.8, ());
        core.set_volume(0.6);
        core.toggle_mute_restoring_volume();
        assert_eq!(core.effective_volume(), 0.0);
        assert_eq!(core.volume(), 0.6);

        core.toggle_mute_restoring_volume();
        assert_eq!(core.effective_volume(), 0.6);
    }

    #[test]
    fn unmuting_restores_a_volume_dragged_to_zero() {
        let mut core = PlaybackCore::new(0.8, ());
        core.toggle_mute_restoring_volume();
        core.set_volume(0.0);
        core.toggle_mute_restoring_volume();
        assert_eq!(core.volume(), 0.8);
    }

    #[test]
    fn plain_flag_flip_keeps_a_zero_volume() {
        let mut core = PlaybackCore::new(0.8, ());
        core.set_volume(0.0);
        core.set_muted(true);
        core.set_muted(false);
        assert_eq!(core.volume(), 0.0, "audio semantics never restore volume");
    }

    #[test]
    fn progress_is_clamped_and_zero_without_duration() {
        let mut core = PlaybackCore::new(0.8, ());
        assert_eq!(core.progress(), 0.0);

        core.set_duration(100.0);
        core.set_current_time(150.0);
        assert_eq!(core.current_time(), 100.0);
        assert_eq!(core.progress(), 1.0);
    }

    #[test]
    fn duration_semantics_differ_per_player() {
        let mut audio = PlaybackCore::new(0.8, ());
        audio.set_duration(50.0);
        audio.set_current_time(40.0);
        assert!(!audio.try_set_duration(-1.0), "audio rejects negatives");
        assert_eq!(audio.duration(), 50.0);
        assert!(audio.try_set_duration(10.0));
        assert_eq!(audio.current_time(), 10.0, "audio re-clamps the playhead");

        let mut video = PlaybackCore::new(1.0, ());
        video.set_duration(-1.0);
        assert_eq!(video.duration(), 0.0, "video clamps negatives to zero");
    }

    #[test]
    fn track_fraction_clamps_into_the_track() {
        let bounds = Bounds::new(point(px(100.0), px(0.0)), size(px(200.0), px(4.0)));
        assert_eq!(track_fraction(point(px(100.0), px(0.0)), bounds), Some(0.0));
        assert_eq!(track_fraction(point(px(200.0), px(0.0)), bounds), Some(0.5));
        assert_eq!(track_fraction(point(px(999.0), px(0.0)), bounds), Some(1.0));
        assert_eq!(
            track_fraction(point(px(0.0), px(0.0)), bounds),
            Some(0.0),
            "positions left of the track clamp to the start"
        );
        assert_eq!(
            track_fraction(point(px(0.0), px(0.0)), Bounds::default()),
            None
        );
    }

    #[test]
    fn volume_icon_picks_the_three_audible_ranges() {
        assert_eq!(volume_icon(0.0, false), "volume-x");
        assert_eq!(volume_icon(0.3, false), "volume-1");
        assert_eq!(volume_icon(0.5, false), "volume-2");
        assert_eq!(volume_icon(0.9, true), "volume-x");
    }
}
