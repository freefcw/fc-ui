pub mod avatar_group;
pub mod calendar;
pub mod carousel;
pub mod checkbox;
pub mod collapsible;
pub mod color_picker;
pub mod combobox;
pub mod copy_button;
pub mod date_picker;
pub mod drag_drop;
pub mod dropdown;
pub mod form;
pub mod hotkey_input;
pub mod inline_edit;
pub mod input;
pub(crate) mod input_format;
pub mod input_state;
pub mod mention_input;
pub mod number_input;
pub mod otp_input;
pub mod radio;
pub mod range_slider;
pub mod rating;
pub mod search_input;
pub mod select;
pub mod slider;
pub mod sortable_list;
pub mod stepper;
pub mod tag_input;
pub mod text_field;
pub(crate) mod text_util;
pub mod textarea;
pub mod textarea_state;
pub mod time_picker;
pub mod toggle;
pub mod toggle_group;

/// Register controls that historically precede image viewer keybindings.
pub(crate) fn init_before_image_viewer(cx: &mut gpui::App) {
    input::init(cx);
    textarea::init(cx);
    otp_input::init(cx);
    select::init_select(cx);
    combobox::init_combobox(cx);
    date_picker::init(cx);
    dropdown::init_dropdown(cx);
}

/// Register controls that historically follow image viewer keybindings.
pub(crate) fn init_after_image_viewer(cx: &mut gpui::App) {
    inline_edit::init(cx);
    mention_input::init_mention_input(cx);
}
