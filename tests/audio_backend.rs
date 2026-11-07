#![cfg(feature = "audio")]

use fc_ui::components::audio_player::{AudioPlayback, AudioPlayerState, PlaybackSpeed};
use gpui::{AppContext, TestApp};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Default)]
struct Output {
    volume: f32,
    speed: f32,
    playing: bool,
    stopped: bool,
    finished: bool,
    paths: Vec<String>,
}

struct FakePlayback(Arc<Mutex<Output>>);

impl AudioPlayback for FakePlayback {
    fn load(&mut self, path: &str) -> Result<Duration, String> {
        let mut output = self.0.lock().unwrap();
        output.paths.push(path.into());
        // A new source resets the device-side settings, just as rodio does.
        output.volume = 1.;
        output.speed = 1.;
        output.playing = false;
        output.finished = false;
        Ok(Duration::from_secs(60))
    }
    fn play(&self) {
        self.0.lock().unwrap().playing = true;
    }
    fn pause(&self) {
        self.0.lock().unwrap().playing = false;
    }
    fn stop(&self) {
        let mut output = self.0.lock().unwrap();
        output.stopped = true;
        output.playing = false;
    }
    fn seek(&self, _: Duration) {}
    fn set_volume(&self, volume: f32) {
        self.0.lock().unwrap().volume = volume;
    }
    fn set_speed(&self, speed: f32) {
        self.0.lock().unwrap().speed = speed;
    }
    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().finished
    }
}

#[gpui::test]
fn loading_and_reloading_keep_muted_output_and_playback_speed() {
    let mut app = TestApp::new();
    let output = Arc::new(Mutex::new(Output {
        playing: true,
        ..Default::default()
    }));
    let state = app.update(|cx| {
        cx.new(|cx| AudioPlayerState::new_with_backend(cx, FakePlayback(output.clone())))
    });
    assert!(!output.lock().unwrap().playing, "new controls start paused");
    app.update(|cx| {
        state.update(cx, |state, cx| {
            state.set_volume(0.4, cx);
            state.set_playback_speed(PlaybackSpeed::Double, cx);
            state.set_muted(true, cx);
            for path in ["first.mp3", "second.mp3"] {
                assert!(state.load_file(path, cx));
                state.set_playing(true, cx);
                assert!(state.is_muted());
                assert_eq!(output.lock().unwrap().volume, 0.);
                assert_eq!(output.lock().unwrap().speed, 2.);
                assert!(output.lock().unwrap().playing);
            }
            state.set_muted(false, cx);
            assert_eq!(output.lock().unwrap().volume, 0.4);
            state.stop(cx);
            assert!(!state.is_playing());
            assert!(output.lock().unwrap().stopped);
            assert!(!output.lock().unwrap().playing);
            assert!(!state.is_finished());
            output.lock().unwrap().finished = true;
            assert!(state.is_finished());
        })
    });
    assert_eq!(output.lock().unwrap().paths, ["first.mp3", "second.mp3"]);
}

#[test]
fn audio_state_preserves_its_thread_traits_with_an_injected_backend() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AudioPlayerState>();
}
