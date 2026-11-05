pub mod alert_dialog;
pub mod bottom_sheet;
pub mod confirm_dialog;
pub mod context_menu;
pub mod dialog;
pub mod hover_card;
pub mod notification_center;
pub(crate) mod overlay_shell;
pub mod popover;
pub mod popover_menu;
pub mod sheet;
pub mod toast;
pub mod tooltip;

pub(crate) fn init(cx: &mut gpui::App) {
    popover::init(cx);
    dialog::init_dialog(cx);
    sheet::init_sheet(cx);
    alert_dialog::init_alert_dialog(cx);
    bottom_sheet::init_bottom_sheet(cx);
}
