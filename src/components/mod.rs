//! Compatibility facade for the legacy component taxonomy.

pub use crate::capabilities::content::code_block;
pub use crate::capabilities::controls::{
    avatar_group, calendar, carousel, checkbox, collapsible, color_picker, combobox, copy_button,
    date_picker, drag_drop, dropdown, form, hotkey_input, inline_edit, input, input_state,
    mention_input, number_input, otp_input, radio, range_slider, rating, search_input, select,
    slider, sortable_list, stepper, tag_input, textarea, textarea_state, time_picker, toggle,
    toggle_group,
};
pub use crate::capabilities::data::{countdown, sparkline, timeline};
#[cfg(feature = "editor")]
pub use crate::capabilities::editor::editor;
pub use crate::capabilities::effects::{
    animated_collapsible, animated_counter, animated_list, animated_presence, animated_progress,
    animated_switch, animated_text, aurora, confetti, dot_pattern, expandable_card, glass_morphism,
    gradient_border, gradient_text, layout_transition, magnetic_button, marquee, meteors, noise,
    number_ticker, particle_emitter, pulse_indicator, shared_element_transition, shimmer,
    skeleton_loader, spotlight, text_highlight, text_reveal, tilt_card, type_writer,
};
#[cfg(feature = "qrcode")]
pub use crate::capabilities::media::qr_code;
pub use crate::capabilities::media::{
    audio_player, canvas_component, crop_area, file_upload, image_viewer, svg_renderer,
    video_player, waveform,
};
pub use crate::capabilities::navigation::{
    dock, drawer_navigation, floating_action_button, keyboard_shortcuts, navigation_menu,
    pagination, segmented_nav, view_router,
};
pub use crate::capabilities::overlays::{confirm_dialog, notification_center, tooltip};
pub use crate::capabilities::primitives::{
    alert, avatar, button, empty_state, icon, icon_button, icon_source, kbd, label, progress,
    ripple, separator, skeleton, spinner, text,
};
pub use crate::capabilities::scroll::{
    infinite_scroll, resizable, scrollable, scrollbar, split_pane,
};

pub use crate::capabilities::controls::slider::SliderAxis;
pub use crate::capabilities::primitives::display::badge;
pub use crate::capabilities::primitives::icon::{IconSize, IconVariant};
pub use crate::capabilities::primitives::icon_source::IconSource;
