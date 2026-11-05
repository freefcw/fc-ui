pub mod audio_player;
pub mod canvas_component;
pub mod crop_area;
pub mod file_upload;
pub mod image_viewer;
pub(crate) mod playback;
#[cfg(feature = "qrcode")]
pub mod qr_code;
pub mod svg_renderer;
pub mod video_player;
pub mod waveform;

/// Format a media timestamp as `H:MM:SS` when hours are present, otherwise
/// zero-padded `MM:SS`. Shared by the audio and video players so both render
/// identical durations.
pub(crate) fn format_time(seconds: f64) -> String {
    let total_seconds = seconds.max(0.0).floor() as u64;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let secs = total_seconds % 60;

    if hours > 0 {
        format!("{}:{:02}:{:02}", hours, minutes, secs)
    } else {
        format!("{:02}:{:02}", minutes, secs)
    }
}

pub(crate) fn init_image(cx: &mut gpui::App) {
    image_viewer::init_image_viewer(cx);
}

pub(crate) fn init_video(cx: &mut gpui::App) {
    video_player::init_video_player(cx);
}

#[cfg(test)]
mod tests {
    use super::format_time;

    #[test]
    fn formats_minutes_and_seconds_zero_padded() {
        assert_eq!(format_time(5.0), "00:05");
        assert_eq!(format_time(65.4), "01:05");
    }

    #[test]
    fn formats_hours_without_leading_zero() {
        assert_eq!(format_time(3661.0), "1:01:01");
    }

    #[test]
    fn clamps_negative_input_to_zero() {
        assert_eq!(format_time(-3.0), "00:00");
    }
}
