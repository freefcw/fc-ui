use crate::capabilities::foundation::icon_config::resolve_icon_path;
use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::media::playback::{self, PlaybackCore, SliderDrag};
use gpui::{prelude::*, *};
use std::rc::Rc;
#[cfg(feature = "audio")]
use std::sync::{Arc, Mutex};

type WindowCallback = Rc<dyn Fn(&mut Window, &mut App) + 'static>;
type ValueCallback<T> = Rc<dyn Fn(T, &mut Window, &mut App) + 'static>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum AudioPlayerSize {
    Compact,
    #[default]
    Full,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum PlaybackSpeed {
    Half,
    #[default]
    Normal,
    OneAndHalf,
    Double,
}

impl PlaybackSpeed {
    pub fn value(&self) -> f32 {
        match self {
            PlaybackSpeed::Half => 0.5,
            PlaybackSpeed::Normal => 1.0,
            PlaybackSpeed::OneAndHalf => 1.5,
            PlaybackSpeed::Double => 2.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            PlaybackSpeed::Half => "0.5x",
            PlaybackSpeed::Normal => "1x",
            PlaybackSpeed::OneAndHalf => "1.5x",
            PlaybackSpeed::Double => "2x",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PlaybackSpeed::Half => PlaybackSpeed::Normal,
            PlaybackSpeed::Normal => PlaybackSpeed::OneAndHalf,
            PlaybackSpeed::OneAndHalf => PlaybackSpeed::Double,
            PlaybackSpeed::Double => PlaybackSpeed::Half,
        }
    }
}

#[cfg(feature = "audio")]
pub struct AudioBackend {
    // Fields drop in declaration order: `player` is dropped before
    // `device_sink`, honouring the sink's rule that it must outlive every
    // `Player` connected to its mixer.
    player: rodio::Player,
    /// Owns the output device. Dropping it stops playback.
    device_sink: rodio::MixerDeviceSink,
    file_path: Option<String>,
}

#[cfg(feature = "audio")]
impl AudioBackend {
    pub fn new() -> Option<Self> {
        let mut device_sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        // Playback ends silently when the sink is dropped; the library should
        // not print rodio's own notice to stderr.
        device_sink.log_on_drop(false);
        let player = rodio::Player::connect_new(device_sink.mixer());
        player.pause();
        Some(Self {
            device_sink,
            player,
            file_path: None,
        })
    }

    pub fn load(&mut self, path: &str) -> Result<std::time::Duration, String> {
        use rodio::Source;

        let source = open_decoder(path)?;
        let duration = source.total_duration().unwrap_or(std::time::Duration::ZERO);

        // Reconnecting a fresh player drops the previous source and resets
        // volume/speed state along with it, so reapply nothing here.
        self.player = rodio::Player::connect_new(self.device_sink.mixer());
        self.player.append(source);
        self.player.pause();
        self.file_path = Some(path.to_string());

        Ok(duration)
    }

    pub fn play(&self) {
        self.player.play();
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn stop(&self) {
        self.player.pause();
        let _ = self.player.try_seek(std::time::Duration::ZERO);
    }

    pub fn seek(&self, position: std::time::Duration) {
        let _ = self.player.try_seek(position);
    }

    pub fn set_volume(&self, volume: f32) {
        self.player.set_volume(volume);
    }

    pub fn set_speed(&self, speed: f32) {
        self.player.set_speed(speed);
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }
}

/// Decodes `path` into a seekable source.
///
/// Converts the `File` itself rather than a `BufReader`: only that `TryFrom`
/// impl records the byte length, which both `total_duration` and `try_seek`
/// rely on. Wrapping the file in a `BufReader` first produces the same
/// `Decoder<BufReader<File>>` type but silently reports no duration.
#[cfg(feature = "audio")]
fn open_decoder(path: &str) -> Result<rodio::Decoder<std::io::BufReader<std::fs::File>>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    rodio::Decoder::try_from(file).map_err(|e| e.to_string())
}

pub struct AudioPlayerState {
    is_playing: bool,
    core: PlaybackCore<PlaybackSpeed>,
    focus_handle: FocusHandle,
    progress_dragging: bool,
    volume_dragging: bool,
    progress_bounds: Bounds<Pixels>,
    volume_bounds: Bounds<Pixels>,
    source_path: Option<String>,
    #[cfg(feature = "audio")]
    backend: Option<Arc<Mutex<AudioBackend>>>,
}

impl AudioPlayerState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            is_playing: false,
            core: PlaybackCore::new(0.8, PlaybackSpeed::Normal),
            focus_handle: cx.focus_handle(),
            progress_dragging: false,
            volume_dragging: false,
            progress_bounds: Bounds::default(),
            volume_bounds: Bounds::default(),
            source_path: None,
            #[cfg(feature = "audio")]
            backend: AudioBackend::new().map(|b| Arc::new(Mutex::new(b))),
        }
    }

    #[cfg(feature = "audio")]
    pub fn load_file(&mut self, path: impl Into<String>, cx: &mut Context<Self>) -> bool {
        let path_str = path.into();

        if let Some(ref backend) = self.backend {
            if let Ok(mut backend) = backend.lock() {
                match backend.load(&path_str) {
                    Ok(duration) => {
                        self.core.set_duration(f64::from(duration.as_secs_f32()));
                        self.core.set_current_time(0.0);
                        self.is_playing = false;
                        backend.set_volume(self.core.volume());
                        backend.set_speed(self.core.speed().value());
                        self.source_path = Some(path_str);
                        cx.notify();
                        return true;
                    }
                    Err(e) => {
                        eprintln!("Failed to load audio: {}", e);
                    }
                }
            }
        }
        self.source_path = None;
        cx.notify();
        false
    }

    #[cfg(not(feature = "audio"))]
    pub fn load_file(&mut self, path: impl Into<String>, cx: &mut Context<Self>) -> bool {
        self.source_path = Some(path.into());
        cx.notify();
        false
    }

    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    pub fn set_playing(&mut self, playing: bool, cx: &mut Context<Self>) {
        self.is_playing = playing;
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                if playing {
                    backend.play();
                } else {
                    backend.pause();
                }
            }
        }
        cx.notify();
    }

    pub fn toggle_playing(&mut self, cx: &mut Context<Self>) {
        self.is_playing = !self.is_playing;
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                if self.is_playing {
                    backend.play();
                } else {
                    backend.pause();
                }
            }
        }
        cx.notify();
    }

    pub fn is_muted(&self) -> bool {
        self.core.muted()
    }

    pub fn set_muted(&mut self, muted: bool, cx: &mut Context<Self>) {
        self.core.set_muted(muted);
        #[cfg(feature = "audio")]
        self.apply_volume();
        cx.notify();
    }

    pub fn toggle_muted(&mut self, cx: &mut Context<Self>) {
        self.core.set_muted(!self.core.muted());
        #[cfg(feature = "audio")]
        self.apply_volume();
        cx.notify();
    }

    pub fn current_time(&self) -> f32 {
        self.core.current_time() as f32
    }

    pub fn set_current_time(&mut self, time: f32, cx: &mut Context<Self>) {
        self.core.set_current_time(f64::from(time));
        cx.notify();
    }

    pub fn duration(&self) -> f32 {
        self.core.duration() as f32
    }

    pub fn set_duration(&mut self, duration: f32, cx: &mut Context<Self>) {
        if self.core.try_set_duration(f64::from(duration)) {
            cx.notify();
        }
    }

    pub fn volume(&self) -> f32 {
        self.core.volume()
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.core.set_volume(volume);
        #[cfg(feature = "audio")]
        self.apply_volume();
        cx.notify();
    }

    #[cfg(feature = "audio")]
    fn apply_volume(&self) {
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                backend.set_volume(self.core.effective_volume());
            }
        }
    }

    pub fn effective_volume(&self) -> f32 {
        self.core.effective_volume()
    }

    pub fn playback_speed(&self) -> PlaybackSpeed {
        *self.core.speed()
    }

    pub fn set_playback_speed(&mut self, speed: PlaybackSpeed, cx: &mut Context<Self>) {
        self.core.set_speed(speed);
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                backend.set_speed(speed.value());
            }
        }
        cx.notify();
    }

    pub fn cycle_playback_speed(&mut self, cx: &mut Context<Self>) {
        let next = self.core.speed().next();
        self.core.set_speed(next);
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                backend.set_speed(next.value());
            }
        }
        cx.notify();
    }

    pub fn stop(&mut self, cx: &mut Context<Self>) {
        self.is_playing = false;
        self.core.set_current_time(0.0);
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                backend.stop();
            }
        }
        cx.notify();
    }

    #[cfg(feature = "audio")]
    pub fn is_finished(&self) -> bool {
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                return backend.is_empty();
            }
        }
        false
    }

    #[cfg(not(feature = "audio"))]
    pub fn is_finished(&self) -> bool {
        self.current_time() >= self.duration()
    }

    fn progress_percentage(&self) -> f32 {
        self.core.progress() as f32
    }

    fn update_progress_from_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(percentage) = playback::track_fraction(position, self.progress_bounds) else {
            return;
        };
        self.core
            .set_current_time(f64::from(percentage) * self.core.duration());
        #[cfg(feature = "audio")]
        if let Some(ref backend) = self.backend {
            if let Ok(backend) = backend.lock() {
                backend.seek(std::time::Duration::from_secs_f32(self.current_time()));
            }
        }
        cx.notify();
    }

    fn update_volume_from_position(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if let Some(volume) = playback::track_fraction(position, self.volume_bounds) {
            self.set_volume(volume, cx);
        }
    }

    #[cfg(feature = "audio")]
    pub fn is_audio_loaded(&self) -> bool {
        self.backend.is_some() && self.source_path.is_some()
    }

    #[cfg(not(feature = "audio"))]
    pub fn is_audio_loaded(&self) -> bool {
        false
    }
}

impl Focusable for AudioPlayerState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AudioPlayerState {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

use crate::capabilities::media::format_time;

#[derive(IntoElement)]
pub struct AudioPlayer {
    state: Entity<AudioPlayerState>,
    size: AudioPlayerSize,
    disabled: bool,
    title: Option<SharedString>,
    on_play: Option<WindowCallback>,
    on_pause: Option<WindowCallback>,
    on_seek: Option<ValueCallback<f32>>,
    on_volume_change: Option<ValueCallback<f32>>,
    on_speed_change: Option<ValueCallback<PlaybackSpeed>>,
    style: StyleRefinement,
}

impl AudioPlayer {
    pub fn new(state: Entity<AudioPlayerState>) -> Self {
        Self {
            state,
            size: AudioPlayerSize::Full,
            disabled: false,
            title: None,
            on_play: None,
            on_pause: None,
            on_seek: None,
            on_volume_change: None,
            on_speed_change: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn size(mut self, size: AudioPlayerSize) -> Self {
        self.size = size;
        self
    }

    pub fn compact(mut self) -> Self {
        self.size = AudioPlayerSize::Compact;
        self
    }

    pub fn full(mut self) -> Self {
        self.size = AudioPlayerSize::Full;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
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

    pub fn on_seek(mut self, handler: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
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

    pub fn on_speed_change(
        mut self,
        handler: impl Fn(PlaybackSpeed, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_speed_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for AudioPlayer {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AudioPlayer {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let state = self.state.read(cx);
        let is_playing = state.is_playing;
        let is_muted = state.is_muted();
        let current_time = state.current_time();
        let duration = state.duration();
        let volume = state.volume();
        let progress_percentage = state.progress_percentage();
        let playback_speed = state.playback_speed();
        let user_style = self.style.clone();

        let (padding, gap, button_size, icon_size, track_height, thumb_size) = match self.size {
            AudioPlayerSize::Compact => (px(8.0), px(8.0), px(32.0), px(16.0), px(4.0), px(12.0)),
            AudioPlayerSize::Full => (px(16.0), px(12.0), px(40.0), px(20.0), px(6.0), px(16.0)),
        };

        let play_icon = if is_playing { "pause" } else { "play" };
        let volume_icon = playback::volume_icon(volume, is_muted);

        let base = div()
            .flex()
            .items_center()
            .gap(gap)
            .p(padding)
            .bg(theme.tokens.card)
            .border_1()
            .border_color(theme.tokens.border)
            .rounded(theme.tokens.radius_lg)
            .when(self.disabled, |this| this.opacity(0.5));

        match self.size {
            AudioPlayerSize::Compact => base
                .map(|this| {
                    let mut div = this;
                    div.style().refine(&user_style);
                    div
                })
                .child(self.render_play_button(
                    window,
                    &theme,
                    play_icon,
                    is_playing,
                    button_size,
                    icon_size,
                ))
                .child(self.render_progress_bar(
                    window,
                    &theme,
                    progress_percentage,
                    track_height,
                    thumb_size,
                ))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(theme.tokens.muted_foreground)
                        .font_family(theme.tokens.font_mono.clone())
                        .child(format!(
                            "{} / {}",
                            format_time(current_time as f64),
                            format_time(duration as f64)
                        )),
                ),

            AudioPlayerSize::Full => base
                .flex_col()
                .gap(px(12.0))
                .map(|this| {
                    let mut div = this;
                    div.style().refine(&user_style);
                    div
                })
                .when_some(self.title.clone(), |this, title| {
                    this.child(
                        div()
                            .w_full()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.tokens.foreground)
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(title),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .w_full()
                        .child(self.render_progress_bar(
                            window,
                            &theme,
                            progress_percentage,
                            track_height,
                            thumb_size,
                        ))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .w_full()
                                .text_size(px(12.0))
                                .font_family(theme.tokens.font_mono.clone())
                                .text_color(theme.tokens.muted_foreground)
                                .child(format_time(current_time as f64))
                                .child(format_time(duration as f64)),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(div().flex().items_center().gap(px(8.0)).child(
                            self.render_play_button(
                                window,
                                &theme,
                                play_icon,
                                is_playing,
                                button_size,
                                icon_size,
                            ),
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(self.render_speed_button(window, &theme, playback_speed))
                                .child(self.render_mute_button(
                                    window,
                                    &theme,
                                    volume_icon,
                                    px(28.0),
                                    px(14.0),
                                ))
                                .child(self.render_volume_slider(
                                    window,
                                    &theme,
                                    volume,
                                    px(80.0),
                                    px(4.0),
                                    px(12.0),
                                )),
                        ),
                ),
        }
    }
}

impl AudioPlayer {
    fn render_play_button(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        icon_name: &str,
        _is_playing: bool,
        button_size: Pixels,
        icon_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_play = self.on_play.clone();
        let on_pause = self.on_pause.clone();
        let disabled = self.disabled;

        div()
            .id("audio-play-button")
            .flex()
            .items_center()
            .justify_center()
            .size(button_size)
            .rounded_full()
            .bg(theme.tokens.primary)
            .when(!disabled, |this| {
                this.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme.tokens.primary.opacity(0.9)))
            })
            .child(
                svg()
                    .path(resolve_icon_path(icon_name))
                    .size(icon_size)
                    .text_color(theme.tokens.primary_foreground),
            )
            .when(!disabled, |this| {
                this.on_click(window.listener_for(&state, move |state, _, window, cx| {
                    state.toggle_playing(cx);
                    if state.is_playing {
                        if let Some(ref handler) = on_play {
                            handler(window, cx);
                        }
                    } else if let Some(ref handler) = on_pause {
                        handler(window, cx);
                    }
                }))
            })
    }

    fn render_progress_bar(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        percentage: f32,
        track_height: Pixels,
        thumb_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_seek = self.on_seek.clone();
        let disabled = self.disabled;

        div()
            .flex_1()
            .h(thumb_size)
            .flex()
            .items_center()
            .relative()
            .child(
                canvas(
                    {
                        let state = state.clone();
                        move |bounds, _, cx| {
                            state.update(cx, |state, _| {
                                state.progress_bounds = bounds;
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .w_full()
                    .h(track_height)
                    .rounded_full()
                    .bg(theme.tokens.muted)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(percentage))
                            .bg(theme.tokens.primary),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(percentage))
                    .top(px(0.0))
                    .ml(-(thumb_size / 2.0))
                    .size(thumb_size)
                    .rounded_full()
                    .bg(theme.tokens.primary)
                    .border_2()
                    .border_color(theme.tokens.background)
                    .shadow_sm()
                    .when(!disabled, |this| this.cursor(CursorStyle::PointingHand)),
            )
            .when(!disabled, |this| {
                playback::attach_slider_drag(
                    this,
                    window,
                    SliderDrag {
                        state: state.clone(),
                        on_change: on_seek.clone(),
                        is_dragging: |s: &AudioPlayerState| s.progress_dragging,
                        set_dragging: |s: &mut AudioPlayerState, dragging| {
                            s.progress_dragging = dragging
                        },
                        apply_position: |s, position, cx| {
                            s.update_progress_from_position(position, cx)
                        },
                        current_value: |s: &AudioPlayerState| s.current_time(),
                    },
                )
            })
    }

    fn render_mute_button(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        icon_name: &str,
        button_size: Pixels,
        icon_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let disabled = self.disabled;

        div()
            .id("audio-mute-button")
            .flex()
            .items_center()
            .justify_center()
            .size(button_size)
            .rounded(theme.tokens.radius_md)
            .when(!disabled, |this| {
                this.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme.tokens.accent))
            })
            .child(
                svg()
                    .path(resolve_icon_path(icon_name))
                    .size(icon_size)
                    .text_color(theme.tokens.muted_foreground),
            )
            .when(!disabled, |this| {
                this.on_click(window.listener_for(&state, move |state, _, _, cx| {
                    state.toggle_muted(cx);
                }))
            })
    }

    fn render_volume_slider(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        volume: f32,
        width: Pixels,
        track_height: Pixels,
        thumb_size: Pixels,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_volume_change = self.on_volume_change.clone();
        let disabled = self.disabled;

        div()
            .w(width)
            .h(thumb_size)
            .flex()
            .items_center()
            .relative()
            .child(
                canvas(
                    {
                        let state = state.clone();
                        move |bounds, _, cx| {
                            state.update(cx, |state, _| {
                                state.volume_bounds = bounds;
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div()
                    .w_full()
                    .h(track_height)
                    .rounded_full()
                    .bg(theme.tokens.muted)
                    .overflow_hidden()
                    .child(
                        div()
                            .h_full()
                            .w(relative(volume))
                            .bg(theme.tokens.muted_foreground),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left(relative(volume))
                    .top(px(0.0))
                    .ml(-(thumb_size / 2.0))
                    .size(thumb_size)
                    .rounded_full()
                    .bg(theme.tokens.foreground)
                    .border_2()
                    .border_color(theme.tokens.background)
                    .shadow_sm()
                    .when(!disabled, |this| this.cursor(CursorStyle::PointingHand)),
            )
            .when(!disabled, |this| {
                playback::attach_slider_drag(
                    this,
                    window,
                    SliderDrag {
                        state: state.clone(),
                        on_change: on_volume_change.clone(),
                        is_dragging: |s: &AudioPlayerState| s.volume_dragging,
                        set_dragging: |s: &mut AudioPlayerState, dragging| {
                            s.volume_dragging = dragging
                        },
                        apply_position: |s, position, cx| {
                            s.update_volume_from_position(position, cx)
                        },
                        current_value: |s: &AudioPlayerState| s.volume(),
                    },
                )
            })
    }

    fn render_speed_button(
        &self,
        window: &mut Window,
        theme: &crate::capabilities::foundation::theme::Theme,
        speed: PlaybackSpeed,
    ) -> impl IntoElement {
        let state = self.state.clone();
        let on_speed_change = self.on_speed_change.clone();
        let disabled = self.disabled;

        div()
            .id("audio-speed-button")
            .flex()
            .items_center()
            .justify_center()
            .px(px(8.0))
            .h(px(28.0))
            .rounded(theme.tokens.radius_md)
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.tokens.muted_foreground)
            .when(!disabled, |this| {
                this.cursor(CursorStyle::PointingHand)
                    .hover(|style| style.bg(theme.tokens.accent))
            })
            .child(speed.label())
            .when(!disabled, |this| {
                this.on_click(window.listener_for(&state, move |state, _, window, cx| {
                    state.cycle_playback_speed(cx);
                    if let Some(ref handler) = on_speed_change {
                        handler(state.playback_speed(), window, cx);
                    }
                }))
            })
    }
}

#[cfg(all(test, feature = "audio"))]
mod tests {
    use super::open_decoder;
    use rodio::Source;

    const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/audio/sample.mp3");

    /// The `File` decoder path must report the real duration. Decoding through
    /// a `BufReader` yields the same type but no duration, which would leave
    /// the progress bar and seek target pinned at zero.
    #[test]
    fn decoding_a_file_reports_its_duration() {
        let source = open_decoder(SAMPLE).expect("decode sample.mp3");
        let duration = source.total_duration().expect("duration is known");

        // `afinfo` reports 372.7s for this asset; allow a wide margin so the
        // test tracks the decoder path, not the exact encoder settings.
        assert!(
            duration.as_secs_f64() > 300.0,
            "expected a multi-minute duration, got {duration:?}"
        );
    }

    /// Seeking must work on the decoded source. `rodio::Player::try_seek`
    /// delegates here, so a regression shows up as a dead progress bar.
    #[test]
    fn a_decoded_file_can_be_seeked() {
        let mut source = open_decoder(SAMPLE).expect("decode sample.mp3");
        let _: Vec<_> = source.by_ref().take(44_100).collect();

        assert!(
            source.try_seek(std::time::Duration::from_secs(30)).is_ok(),
            "seeking a decoded file failed"
        );
    }

    #[test]
    fn a_missing_file_is_an_error_rather_than_a_panic() {
        assert!(open_decoder("/nonexistent/fc-ui.mp3").is_err());
    }
}
