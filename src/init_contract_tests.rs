#[cfg(feature = "editor")]
use crate::capabilities::editor::editor::{self, MoveUp};
use crate::capabilities::{
    controls::{
        combobox::{self, ComboboxCancel},
        date_picker::{self, ClosePicker},
        dropdown::{self, CloseDropdown},
        inline_edit::{self, Save},
        input::{self, Backspace as InputBackspace},
        mention_input::{self, MentionConfirm},
        otp_input::{self, OTPBackspace},
        select::{self, SelectCancel},
        textarea::{self},
        textarea_state::Backspace as TextareaBackspace,
    },
    media::{
        image_viewer::{self, ImageViewerClose},
        video_player::{self, VideoPlayerTogglePlay},
    },
    navigation::{
        sidebar::{self, ToggleSidebar},
        tabs::{self, TabNext},
    },
    overlays::{
        alert_dialog,
        dialog::{self, DialogCancel},
        popover::{self, ClosePopover},
        sheet,
    },
};
use gpui::{Action, App, TestApp};

fn binding_count(cx: &App, action: &dyn Action) -> usize {
    cx.key_bindings()
        .borrow()
        .bindings_for_action(action)
        .count()
}

fn binding_position(cx: &App, action: &dyn Action) -> usize {
    cx.key_bindings()
        .borrow()
        .bindings()
        .position(|binding| binding.action().partial_eq(action))
        .expect("representative action should be registered")
}

fn assert_representative_bindings(cx: &App) {
    let actions: &[(&str, &dyn Action, usize)] = &[
        ("input", &InputBackspace, 1),
        ("textarea", &TextareaBackspace, 1),
        ("otp", &OTPBackspace, 1),
        ("select", &SelectCancel, 1),
        ("combobox", &ComboboxCancel, 1),
        ("date-picker", &ClosePicker, 1),
        ("image-viewer", &ImageViewerClose, 1),
        ("inline-edit", &Save, 1),
        ("mention-input", &MentionConfirm, 1),
        ("video-player", &VideoPlayerTogglePlay, 1),
        // Sidebar intentionally installs no global bindings; apps bind ToggleSidebar themselves.
        ("sidebar", &ToggleSidebar, 0),
        ("dropdown", &CloseDropdown, 1),
        ("tabs", &TabNext, 1),
        ("popover", &ClosePopover, 1),
        ("dialog", &DialogCancel, 1),
    ];
    for (name, action, expected) in actions {
        assert_eq!(binding_count(cx, *action), *expected, "{name}");
    }
    #[cfg(feature = "editor")]
    assert_eq!(binding_count(cx, &MoveUp), 1);
}

fn init_individually(cx: &mut App) {
    input::init(cx);
    textarea::init(cx);
    otp_input::init(cx);
    select::init_select(cx);
    combobox::init_combobox(cx);
    date_picker::init(cx);
    dropdown::init_dropdown(cx);
    image_viewer::init_image_viewer(cx);
    inline_edit::init(cx);
    mention_input::init_mention_input(cx);
    video_player::init_video_player(cx);
    #[cfg(feature = "editor")]
    editor::init(cx);
    sidebar::init_sidebar(cx);
    tabs::init_tabs(cx);
    popover::init(cx);
    dialog::init_dialog(cx);
    sheet::init_sheet(cx);
    alert_dialog::init_alert_dialog(cx);
}

#[gpui::test]
fn root_init_preserves_historical_cross_capability_binding_order() {
    let mut app = TestApp::new();

    app.update(crate::init);

    app.read(|cx| {
        let positions = [
            binding_position(cx, &ClosePicker),
            binding_position(cx, &ImageViewerClose),
            binding_position(cx, &Save),
            binding_position(cx, &VideoPlayerTogglePlay),
        ];
        assert!(
            positions.is_sorted(),
            "expected date picker < image viewer < inline edit < video player, got {positions:?}"
        );
    });
}

#[gpui::test]
fn root_init_twice_installs_each_binding_once() {
    let mut app = TestApp::new();

    app.update(crate::init);
    app.update(crate::init);

    app.read(assert_representative_bindings);
}

#[gpui::test]
fn individual_initializers_before_root_do_not_duplicate_bindings() {
    let mut app = TestApp::new();

    app.update(init_individually);
    app.update(crate::init);

    app.read(assert_representative_bindings);
}

#[gpui::test]
fn individual_initializers_after_root_do_not_duplicate_bindings() {
    let mut app = TestApp::new();

    app.update(crate::init);
    app.update(init_individually);

    app.read(assert_representative_bindings);
}

#[gpui::test]
fn root_init_is_scoped_per_app() {
    let mut first = TestApp::new();
    let mut second = TestApp::new();

    first.update(crate::init);
    second.update(crate::init);

    first.read(assert_representative_bindings);
    second.read(assert_representative_bindings);
}
