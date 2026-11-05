use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::media::format_time;
use crate::capabilities::media::playback::{self, PlaybackCore, SliderDrag};
use gpui::{prelude::FluentBuilder as _, *};
use std::rc::Rc;
use std::time::{Duration, Instant};

type WindowCallback = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type ValueCallback<T> = Rc<dyn Fn(T, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VideoPlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    #[deprecated(note = "never constructed; removal in 0.10")]
    Buffering,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VideoPlayerSize {
    Sm,
    #[default]
    Md,
    Lg,
    Full,
}

impl VideoPlayerSize {
    pub fn dimensions(&self) -> (Pixels, Pixels) {
        match self {
            Self::Sm => (px(400.0), px(225.0)),
            Self::Md => (px(640.0), px(360.0)),
            Self::Lg => (px(854.0), px(480.0)),
            Self::Full => (px(1280.0), px(720.0)),
        }
    }

    pub fn controls_height(&self) -> Pixels {
        match self {
            Self::Sm => px(36.0),
            Self::Md => px(44.0),
            Self::Lg => px(52.0),
            Self::Full => px(56.0),
        }
    }

    pub fn icon_size(&self) -> Pixels {
        match self {
            Self::Sm => px(16.0),
            Self::Md => px(20.0),
            Self::Lg => px(24.0),
            Self::Full => px(28.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VideoPlaybackSpeed {
    Quarter,
    Half,
    ThreeQuarter,
    #[default]
    Normal,
    OneAndQuarter,
    OneAndHalf,
    Double,
}

impl VideoPlaybackSpeed {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::ThreeQuarter => 0.75,
            Self::Normal => 1.0,
            Self::OneAndQuarter => 1.25,
            Self::OneAndHalf => 1.5,
            Self::Double => 2.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Quarter => "0.25x",
            Self::Half => "0.5x",
            Self::ThreeQuarter => "0.75x",
            Self::Normal => "1x",
            Self::OneAndQuarter => "1.25x",
            Self::OneAndHalf => "1.5x",
            Self::Double => "2x",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Quarter => Self::Half,
            Self::Half => Self::ThreeQuarter,
            Self::ThreeQuarter => Self::Normal,
            Self::Normal => Self::OneAndQuarter,
            Self::OneAndQuarter => Self::OneAndHalf,
            Self::OneAndHalf => Self::Double,
            Self::Double => Self::Quarter,
        }
    }

    pub fn all() -> &'static [VideoPlaybackSpeed] {
        &[
            Self::Quarter,
            Self::Half,
            Self::ThreeQuarter,
            Self::Normal,
            Self::OneAndQuarter,
            Self::OneAndHalf,
            Self::Double,
        ]
    }
}

pub struct VideoPlayerState {
    playback_state: VideoPlaybackState,
    core: PlaybackCore<VideoPlaybackSpeed>,
    is_fullscreen: bool,
    show_controls: bool,
    last_interaction: Instant,
    controls_timeout: Duration,
    auto_hide_generation: u64,
    auto_hide_task: Option<Task<()>>,
    is_seeking: bool,
    progress_bounds: Bounds<Pixels>,
    volume_bounds: Bounds<Pixels>,
    is_volume_dragging: bool,
    show_speed_menu: bool,
    focus_handle: FocusHandle,
    current_frame: Option<SharedString>,
    video_title: Option<SharedString>,
}

impl VideoPlayerState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            playback_state: VideoPlaybackState::Stopped,
            core: PlaybackCore::new(1.0, VideoPlaybackSpeed::Normal),
            is_fullscreen: false,
            show_controls: true,
            last_interaction: Instant::now(),
            controls_timeout: Duration::from_secs(3),
            auto_hide_generation: 0,
            auto_hide_task: None,
            is_seeking: false,
            progress_bounds: Bounds::default(),
            volume_bounds: Bounds::default(),
            is_volume_dragging: false,
            show_speed_menu: false,
            focus_handle: cx.focus_handle(),
            current_frame: None,
            video_title: None,
        }
    }

    pub fn set_frame(&mut self, frame_path: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.current_frame = Some(frame_path.into());
        cx.notify();
    }

    pub fn clear_frame(&mut self, cx: &mut Context<Self>) {
        self.current_frame = None;
        cx.notify();
    }

    pub fn current_frame(&self) -> Option<&SharedString> {
        self.current_frame.as_ref()
    }

    pub fn set_title(&mut self, title: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.video_title = Some(title.into());
        cx.notify();
    }

    pub fn title(&self) -> Option<&SharedString> {
        self.video_title.as_ref()
    }

    pub fn playback_state(&self) -> VideoPlaybackState {
        self.playback_state
    }

    pub fn is_playing(&self) -> bool {
        self.playback_state == VideoPlaybackState::Playing
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        self.playback_state = VideoPlaybackState::Playing;
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.playback_state = VideoPlaybackState::Paused;
        self.show_controls = true;
        self.cancel_auto_hide();
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.playback_state = VideoPlaybackState::Stopped;
        self.core.set_current_time(0.0);
        self.show_controls = true;
        self.cancel_auto_hide();
        cx.notify();
    }

    pub fn toggle_play(&mut self, cx: &mut Context<Self>) {
        match self.playback_state {
            VideoPlaybackState::Playing => self.pause(cx),
            VideoPlaybackState::Paused | VideoPlaybackState::Stopped => self.play(cx),
            #[allow(deprecated)]
            VideoPlaybackState::Buffering => {}
        }
    }

    pub fn current_time(&self) -> f64 {
        self.core.current_time()
    }

    pub fn set_current_time(&mut self, time: f64, cx: &mut Context<Self>) {
        self.core.set_current_time(time);
        cx.notify();
    }

    pub fn duration(&self) -> f64 {
        self.core.duration()
    }

    pub fn set_duration(&mut self, duration: f64, cx: &mut Context<Self>) {
        self.core.set_duration(duration);
        cx.notify();
    }

    pub fn progress(&self) -> f64 {
        self.core.progress()
    }

    pub fn seek(&mut self, position: f64, cx: &mut Context<Self>) {
        let clamped = position.clamp(0.0, 1.0);
        self.core.set_current_time(clamped * self.core.duration());
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn seek_relative(&mut self, delta: f64, cx: &mut Context<Self>) {
        self.core.set_current_time(self.core.current_time() + delta);
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn volume(&self) -> f32 {
        self.core.volume()
    }

    pub fn effective_volume(&self) -> f32 {
        self.core.effective_volume()
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.core.set_volume(volume);
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn is_muted(&self) -> bool {
        self.core.muted()
    }

    pub fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        self.core.toggle_mute_restoring_volume();
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn playback_speed(&self) -> VideoPlaybackSpeed {
        *self.core.speed()
    }

    pub fn set_playback_speed(&mut self, speed: VideoPlaybackSpeed, cx: &mut Context<Self>) {
        self.core.set_speed(speed);
        self.show_speed_menu = false;
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn cycle_playback_speed(&mut self, cx: &mut Context<Self>) {
        let next = self.core.speed().next();
        self.core.set_speed(next);
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn is_fullscreen(&self) -> bool {
        self.is_fullscreen
    }

    pub fn toggle_fullscreen(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.is_fullscreen = !self.is_fullscreen;
        window.toggle_fullscreen();
        self.touch_controls(cx);
        cx.notify();
    }

    pub fn show_controls(&self) -> bool {
        self.show_controls
    }

    pub fn touch_controls(&mut self, cx: &mut Context<Self>) {
        self.show_controls = true;
        self.last_interaction = Instant::now();
        self.auto_hide_generation = self.auto_hide_generation.wrapping_add(1);
        let generation = self.auto_hide_generation;
        let timeout = self.controls_timeout;
        self.auto_hide_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(timeout).await;
            _ = this.update(cx, |state, cx| {
                if state.auto_hide_generation == generation {
                    state.hide_controls(cx);
                }
            });
        }));
        cx.notify();
    }

    fn cancel_auto_hide(&mut self) {
        self.auto_hide_generation = self.auto_hide_generation.wrapping_add(1);
        self.auto_hide_task = None;
    }

    pub fn hide_controls(&mut self, cx: &mut Context<Self>) {
        if self.playback_state == VideoPlaybackState::Playing
            && !self.is_seeking
            && !self.is_volume_dragging
            && self.show_controls
        {
            self.show_controls = false;
            cx.notify();
        }
    }

    pub fn check_auto_hide(&mut self, cx: &mut Context<Self>) {
        if self.last_interaction.elapsed() > self.controls_timeout {
            self.hide_controls(cx);
        }
    }

    pub fn toggle_speed_menu(&mut self, cx: &mut Context<Self>) {
        self.show_speed_menu = !self.show_speed_menu;
        self.touch_controls(cx);
        cx.notify();
    }

    fn update_seek_from_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(percentage) = playback::track_fraction(position, self.progress_bounds) else {
            return;
        };
        self.seek(f64::from(percentage), cx);
    }

    fn update_volume_from_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(volume) = playback::track_fraction(position, self.volume_bounds) {
            self.set_volume(volume, cx);
        }
    }
}

impl Focusable for VideoPlayerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for VideoPlayerState {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

actions!(
    video_player,
    [
        VideoPlayerTogglePlay,
        VideoPlayerMute,
        VideoPlayerFullscreen,
        VideoPlayerSeekForward,
        VideoPlayerSeekBackward,
        VideoPlayerVolumeUp,
        VideoPlayerVolumeDown,
    ]
);

pub fn init_video_player(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "video-player") {
        return;
    }
    cx.bind_keys([
        KeyBinding::new("space", VideoPlayerTogglePlay, Some("VideoPlayer")),
        KeyBinding::new("m", VideoPlayerMute, Some("VideoPlayer")),
        KeyBinding::new("f", VideoPlayerFullscreen, Some("VideoPlayer")),
        KeyBinding::new("right", VideoPlayerSeekForward, Some("VideoPlayer")),
        KeyBinding::new("left", VideoPlayerSeekBackward, Some("VideoPlayer")),
        KeyBinding::new("up", VideoPlayerVolumeUp, Some("VideoPlayer")),
        KeyBinding::new("down", VideoPlayerVolumeDown, Some("VideoPlayer")),
    ]);
}

/// State-derived values the video controls render from, snapshotted once per
/// frame so the split render methods share a single state read.
struct VideoControlSnapshot {
    is_playing: bool,
    current_time: f64,
    duration: f64,
    progress: f64,
    volume: f32,
    is_muted: bool,
    playback_speed: VideoPlaybackSpeed,
    is_fullscreen: bool,
    show_speed_menu: bool,
}

#[derive(IntoElement)]
pub struct VideoPlayer {
    state: Entity<VideoPlayerState>,
    size: VideoPlayerSize,
    poster: Option<SharedString>,
    show_poster: bool,
    overlay_only: bool,
    on_play: Option<WindowCallback>,
    on_pause: Option<WindowCallback>,
    on_seek: Option<ValueCallback<f64>>,
    on_volume_change: Option<ValueCallback<f32>>,
    on_fullscreen: Option<ValueCallback<bool>>,
    on_playback_speed_change: Option<ValueCallback<VideoPlaybackSpeed>>,
    style: StyleRefinement,
}

impl VideoPlayer {
    pub fn new(state: Entity<VideoPlayerState>) -> Self {
        Self {
            state,
            size: VideoPlayerSize::default(),
            poster: None,
            show_poster: true,
            overlay_only: false,
            on_play: None,
            on_pause: None,
            on_seek: None,
            on_volume_change: None,
            on_fullscreen: None,
            on_playback_speed_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: VideoPlayerSize) -> Self {
        self.size = size;
        self
    }

    pub fn poster(mut self, poster: impl Into<SharedString>) -> Self {
        self.poster = Some(poster.into());
        self
    }

    pub fn show_poster(mut self, show: bool) -> Self {
        self.show_poster = show;
        self
    }

    pub fn overlay_only(mut self) -> Self {
        self.overlay_only = true;
        self
    }

    pub fn on_play(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_play = Some(Rc::new(handler));
        self
    }

    pub fn on_pause(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_pause = Some(Rc::new(handler));
        self
    }

    pub fn on_seek(mut self, handler: impl Fn(f64, &mut Window, &mut App) + 'static) -> Self {
        self.on_seek = Some(Rc::new(handler));
        self
    }

    pub fn on_volume_change(
        mut self,
        handler: impl Fn(f32, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_volume_change = Some(Rc::new(handler));
        self
    }

    pub fn on_fullscreen(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_fullscreen = Some(Rc::new(handler));
        self
    }

    pub fn on_playback_speed_change(
        mut self,
        handler: impl Fn(VideoPlaybackSpeed, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_playback_speed_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for VideoPlayer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for VideoPlayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let state = self.state.read(cx);
        let focus_handle = state.focus_handle(cx);

        let playback_state = state.playback_state();
        let current_frame = state.current_frame().cloned();
        let show_controls = state.show_controls();
        let snapshot = VideoControlSnapshot {
            is_playing: state.is_playing(),
            current_time: state.current_time(),
            duration: state.duration(),
            progress: state.progress(),
            volume: state.volume(),
            is_muted: state.is_muted(),
            playback_speed: state.playback_speed(),
            is_fullscreen: state.is_fullscreen(),
            show_speed_menu: state.show_speed_menu,
        };

        let (width, height) = self.size.dimensions();

        let show_poster = self.show_poster
            && self.poster.is_some()
            && playback_state == VideoPlaybackState::Stopped;

        let overlay_only = self.overlay_only;
        let user_style = self.style.clone();

        let state_for_actions = self.state.clone();
        let state_for_mouse = self.state.clone();

        let on_play = self.on_play.clone();
        let on_pause = self.on_pause.clone();
        let on_seek = self.on_seek.clone();
        let on_volume_change = self.on_volume_change.clone();
        let on_fullscreen = self.on_fullscreen.clone();

        div()
            .id("video-player")
            .key_context("VideoPlayer")
            .track_focus(&focus_handle)
            .relative()
            .w(width)
            .h(height)
            .bg(gpui::black())
            .rounded(theme.tokens.radius_lg)
            .overflow_hidden()
            .cursor(CursorStyle::Arrow)
            .map(|this| {
                let mut div = this;
                div.style().refine(&user_style);
                div
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_play = on_play.clone();
                let on_pause = on_pause.clone();
                move |_: &VideoPlayerTogglePlay, window, cx| {
                    let is_playing = state.read(cx).is_playing();
                    cx.update_entity(&state, |state, cx| state.toggle_play(cx));
                    if is_playing {
                        if let Some(handler) = &on_pause {
                            handler(window, cx);
                        }
                    } else if let Some(handler) = &on_play {
                        handler(window, cx);
                    }
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                move |_: &VideoPlayerMute, _, cx| {
                    cx.update_entity(&state, |state, cx| state.toggle_mute(cx));
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_fullscreen = on_fullscreen.clone();
                move |_: &VideoPlayerFullscreen, window, cx| {
                    cx.update_entity(&state, |state, cx| state.toggle_fullscreen(window, cx));
                    let is_fullscreen = state.read(cx).is_fullscreen();
                    if let Some(handler) = &on_fullscreen {
                        handler(is_fullscreen, window, cx);
                    }
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_seek = on_seek.clone();
                move |_: &VideoPlayerSeekForward, window, cx| {
                    cx.update_entity(&state, |state, cx| state.seek_relative(10.0, cx));
                    if let Some(handler) = &on_seek {
                        handler(state.read(cx).current_time(), window, cx);
                    }
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_seek = on_seek.clone();
                move |_: &VideoPlayerSeekBackward, window, cx| {
                    cx.update_entity(&state, |state, cx| state.seek_relative(-10.0, cx));
                    if let Some(handler) = &on_seek {
                        handler(state.read(cx).current_time(), window, cx);
                    }
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_volume_change = on_volume_change.clone();
                move |_: &VideoPlayerVolumeUp, window, cx| {
                    let current = state.read(cx).volume();
                    let new_volume = (current + 0.1).min(1.0);
                    cx.update_entity(&state, |state, cx| state.set_volume(new_volume, cx));
                    if let Some(handler) = &on_volume_change {
                        handler(new_volume, window, cx);
                    }
                }
            })
            .on_action({
                let state = state_for_actions.clone();
                let on_volume_change = on_volume_change.clone();
                move |_: &VideoPlayerVolumeDown, window, cx| {
                    let current = state.read(cx).volume();
                    let new_volume = (current - 0.1).max(0.0);
                    cx.update_entity(&state, |state, cx| state.set_volume(new_volume, cx));
                    if let Some(handler) = &on_volume_change {
                        handler(new_volume, window, cx);
                    }
                }
            })
            .on_mouse_move({
                let state = state_for_mouse.clone();
                window.listener_for(&state, move |state, _: &MouseMoveEvent, _, cx| {
                    state.touch_controls(cx);
                })
            })
            .when(show_poster, |this| this.child(self.render_poster()))
            .when(!show_poster && !overlay_only, |this| {
                this.child(self.render_content_area(&theme, current_frame.as_ref()))
            })
            .child(self.render_center_play_button(
                show_controls || !snapshot.is_playing,
                snapshot.is_playing,
            ))
            .when(show_controls, |this| {
                this.child(self.render_controls_overlay(window, &theme, &snapshot))
            })
    }
}

impl VideoPlayer {
    fn render_poster(&self) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .when_some(self.poster.clone(), |this, poster_src| {
                this.child(img(poster_src).size_full().object_fit(ObjectFit::Cover))
            })
    }

    fn render_content_area(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        current_frame: Option<&SharedString>,
    ) -> impl IntoElement {
        if let Some(frame) = current_frame {
            div().absolute().inset_0().child(
                img(frame.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            )
        } else {
            div()
                .absolute()
                .inset_0()
                .bg(gpui::black())
                .flex()
                .items_center()
                .justify_center()
                .text_color(gpui::white().opacity(0.5))
                .text_sm()
                .font_family(theme.tokens.font_family.clone())
                .child("Video content area")
        }
    }

    fn render_center_play_button(&self, show: bool, is_playing: bool) -> impl IntoElement {
        let state_play = self.state.clone();
        let on_play_center = self.on_play.clone();
        let on_pause_center = self.on_pause.clone();

        div()
            .id("center-play-button")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .when(show, move |this| {
                this.child(
                    div()
                        .id("play-overlay")
                        .size(px(72.0))
                        .rounded_full()
                        .bg(gpui::black().opacity(0.6))
                        .border_2()
                        .border_color(gpui::white().opacity(0.3))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor(CursorStyle::PointingHand)
                        .hover(|style| style.bg(gpui::black().opacity(0.8)))
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            let is_playing_now = state_play.read(cx).is_playing();
                            cx.update_entity(&state_play, |state, cx| state.toggle_play(cx));
                            if is_playing_now {
                                if let Some(handler) = &on_pause_center {
                                    handler(window, cx);
                                }
                            } else if let Some(handler) = &on_play_center {
                                handler(window, cx);
                            }
                        })
                        .child(
                            svg()
                                .path(format!(
                                    "icons/{}.svg",
                                    if is_playing { "pause" } else { "play" }
                                ))
                                .size(px(32.0))
                                .text_color(gpui::white()),
                        ),
                )
            })
    }

    fn render_controls_overlay(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        let controls_height = self.size.controls_height();

        div()
            .id("controls-overlay")
            .absolute()
            .bottom_0()
            .left_0()
            .right_0()
            .h(controls_height + px(40.0))
            .bg(gpui::black().opacity(0.7))
            .flex()
            .flex_col()
            .justify_end()
            .child(
                div()
                    .id("progress-bar-container")
                    .px(px(12.0))
                    .pb(px(4.0))
                    .child(self.render_progress_bar(window, theme, snapshot.progress)),
            )
            .child(self.render_controls_bar(window, theme, snapshot))
    }

    fn render_progress_bar(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        progress: f64,
    ) -> impl IntoElement {
        let state_bounds = self.state.clone();
        let state_drag = self.state.clone();
        let on_seek = self.on_seek.clone();

        playback::attach_slider_drag(
            div()
                .id("progress-bar")
                .relative()
                .h(px(6.0))
                .w_full()
                .bg(gpui::white().opacity(0.3))
                .rounded_full()
                .cursor(CursorStyle::PointingHand)
                .child(
                    canvas(
                        move |bounds, _, cx| {
                            state_bounds.update(cx, |state, _| {
                                state.progress_bounds = bounds;
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .w(relative(progress as f32))
                        .bg(theme.tokens.primary)
                        .rounded_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left(relative(progress as f32))
                        .top(px(-3.0))
                        .ml(px(-6.0))
                        .size(px(12.0))
                        .rounded_full()
                        .bg(theme.tokens.primary)
                        .border_2()
                        .border_color(gpui::white()),
                ),
            window,
            SliderDrag {
                state: state_drag,
                on_change: on_seek,
                is_dragging: |s: &VideoPlayerState| s.is_seeking,
                set_dragging: |s: &mut VideoPlayerState, dragging| s.is_seeking = dragging,
                apply_position: |s, position, cx| s.update_seek_from_position(position, cx),
                current_value: |s: &VideoPlayerState| s.current_time(),
            },
        )
    }

    fn render_controls_bar(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        let controls_height = self.size.controls_height();

        div()
            .id("controls-bar")
            .h(controls_height)
            .px(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .child(self.render_transport_controls(theme, snapshot))
            .child(self.render_utility_controls(window, theme, snapshot))
    }

    fn render_transport_controls(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        let icon_size = self.size.icon_size();

        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(self.render_skip_button(
                theme,
                "skip-back-btn",
                "icons/rewind.svg",
                -10.0,
                icon_size,
            ))
            .child(self.render_play_pause_button(theme, snapshot.is_playing, icon_size))
            .child(self.render_skip_button(
                theme,
                "skip-forward-btn",
                "icons/fast-forward.svg",
                10.0,
                icon_size,
            ))
            .child(
                div()
                    .ml(px(8.0))
                    .text_sm()
                    .text_color(gpui::white())
                    .font_family(theme.tokens.font_family.clone())
                    .child(format!(
                        "{} / {}",
                        format_time(snapshot.current_time),
                        format_time(snapshot.duration)
                    )),
            )
    }

    fn render_skip_button(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        id: &'static str,
        icon_path: &'static str,
        delta_seconds: f64,
        icon_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_seek = self.on_seek.clone();

        div()
            .id(id)
            .size(px(32.0))
            .rounded(theme.tokens.radius_md)
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::PointingHand)
            .hover(|style| style.bg(gpui::white().opacity(0.2)))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.update_entity(&state, |state, cx| {
                    state.seek_relative(delta_seconds, cx);
                });
                if let Some(handler) = &on_seek {
                    handler(state.read(cx).current_time(), window, cx);
                }
            })
            .child(
                svg()
                    .path(icon_path)
                    .size(icon_size)
                    .text_color(gpui::white()),
            )
    }

    fn render_play_pause_button(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        is_playing: bool,
        icon_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_play = self.on_play.clone();
        let on_pause = self.on_pause.clone();

        div()
            .id("play-pause-btn")
            .size(px(40.0))
            .rounded_full()
            .bg(theme.tokens.primary)
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::PointingHand)
            .hover(|style| style.opacity(0.9))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                let is_playing_now = state.read(cx).is_playing();
                cx.update_entity(&state, |state, cx| state.toggle_play(cx));
                if is_playing_now {
                    if let Some(handler) = &on_pause {
                        handler(window, cx);
                    }
                } else if let Some(handler) = &on_play {
                    handler(window, cx);
                }
            })
            .child(
                svg()
                    .path(if is_playing {
                        "icons/pause.svg"
                    } else {
                        "icons/play.svg"
                    })
                    .size(icon_size)
                    .text_color(theme.tokens.primary_foreground),
            )
    }

    fn render_utility_controls(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(self.render_volume_button(theme, snapshot))
                    .child(self.render_volume_slider(window, snapshot.volume)),
            )
            .child(self.render_speed_control(theme, snapshot))
            .child(self.render_fullscreen_button(theme, snapshot.is_fullscreen))
    }

    fn render_volume_button(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let icon_size = self.size.icon_size();
        let volume_icon = playback::volume_icon(snapshot.volume, snapshot.is_muted);

        div()
            .id("volume-btn")
            .size(px(32.0))
            .rounded(theme.tokens.radius_md)
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::PointingHand)
            .hover(|style| style.bg(gpui::white().opacity(0.2)))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                cx.update_entity(&state, |state, cx| state.toggle_mute(cx));
            })
            .child(
                svg()
                    .path(format!("icons/{}.svg", volume_icon))
                    .size(icon_size)
                    .text_color(gpui::white()),
            )
    }

    fn render_volume_slider(&self, window: &mut Window, volume: f32) -> impl IntoElement {
        let state_bounds = self.state.clone();
        let state_drag = self.state.clone();
        let on_volume_change = self.on_volume_change.clone();

        playback::attach_slider_drag(
            div()
                .id("volume-slider")
                .relative()
                .w(px(80.0))
                .h(px(4.0))
                .bg(gpui::white().opacity(0.3))
                .rounded_full()
                .cursor(CursorStyle::PointingHand)
                .child(
                    canvas(
                        move |bounds, _, cx| {
                            state_bounds.update(cx, |state, _| {
                                state.volume_bounds = bounds;
                            });
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .h_full()
                        .w(relative(volume))
                        .bg(gpui::white())
                        .rounded_full(),
                )
                .child(
                    div()
                        .absolute()
                        .left(relative(volume))
                        .top(px(-4.0))
                        .ml(px(-6.0))
                        .size(px(12.0))
                        .rounded_full()
                        .bg(gpui::white()),
                ),
            window,
            SliderDrag {
                state: state_drag,
                on_change: on_volume_change,
                is_dragging: |s: &VideoPlayerState| s.is_volume_dragging,
                set_dragging: |s: &mut VideoPlayerState, dragging| s.is_volume_dragging = dragging,
                apply_position: |s, position, cx| s.update_volume_from_position(position, cx),
                current_value: |s: &VideoPlayerState| s.volume(),
            },
        )
    }

    fn render_speed_control(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        snapshot: &VideoControlSnapshot,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let state_items = self.state.clone();
        let on_speed_change = self.on_playback_speed_change.clone();
        let current_speed = snapshot.playback_speed;
        let show_speed_menu = snapshot.show_speed_menu;
        let theme = theme.clone();

        div()
            .id("speed-btn")
            .relative()
            .child(
                div()
                    .px(px(8.0))
                    .py(px(4.0))
                    .rounded(theme.tokens.radius_md)
                    .text_xs()
                    .text_color(gpui::white())
                    .font_family(theme.tokens.font_family.clone())
                    .cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(gpui::white().opacity(0.2)))
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.update_entity(&state, |state, cx| state.toggle_speed_menu(cx));
                    })
                    .child(current_speed.label()),
            )
            .when(show_speed_menu, move |this| {
                this.child(
                    div()
                        .absolute()
                        .bottom(px(36.0))
                        .right_0()
                        .w(px(80.0))
                        .bg(theme.tokens.popover)
                        .rounded(theme.tokens.radius_md)
                        .border_1()
                        .border_color(theme.tokens.border)
                        .shadow(smallvec::smallvec![theme.tokens.shadow_lg])
                        .py(px(4.0))
                        .children(VideoPlaybackSpeed::all().iter().map(|speed| {
                            let state_item = state_items.clone();
                            let on_speed = on_speed_change.clone();
                            let speed_val = *speed;
                            let is_selected = speed_val == current_speed;

                            div()
                                .id(ElementId::Name(
                                    format!("speed-{}", speed_val.label()).into(),
                                ))
                                .px(px(12.0))
                                .py(px(6.0))
                                .text_xs()
                                .text_color(if is_selected {
                                    theme.tokens.primary
                                } else {
                                    theme.tokens.popover_foreground
                                })
                                .font_family(theme.tokens.font_family.clone())
                                .cursor(CursorStyle::PointingHand)
                                .hover(|style| style.bg(theme.tokens.accent))
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    cx.update_entity(&state_item, |state, cx| {
                                        state.set_playback_speed(speed_val, cx);
                                    });
                                    if let Some(handler) = &on_speed {
                                        handler(speed_val, window, cx);
                                    }
                                })
                                .child(speed_val.label())
                        })),
                )
            })
    }

    fn render_fullscreen_button(
        &self,
        theme: &crate::capabilities::foundation::theme::Theme,
        is_fullscreen: bool,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_fullscreen = self.on_fullscreen.clone();
        let icon_size = self.size.icon_size();

        div()
            .id("fullscreen-btn")
            .size(px(32.0))
            .rounded(theme.tokens.radius_md)
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::PointingHand)
            .hover(|style| style.bg(gpui::white().opacity(0.2)))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.update_entity(&state, |state, cx| state.toggle_fullscreen(window, cx));
                let is_fs = state.read(cx).is_fullscreen();
                if let Some(handler) = &on_fullscreen {
                    handler(is_fs, window, cx);
                }
            })
            .child(
                svg()
                    .path(if is_fullscreen {
                        "icons/minimize.svg"
                    } else {
                        "icons/maximize.svg"
                    })
                    .size(icon_size)
                    .text_color(gpui::white()),
            )
    }
}
