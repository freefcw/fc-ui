pub(crate) mod edit;
pub mod fonts;
pub mod gpui_ext;
pub mod http;
pub mod icon_config;
pub mod initialization;
pub mod responsive;
pub mod styled_ext;
pub mod theme;
pub mod util;

pub(crate) fn init(cx: &mut gpui::App) {
    fonts::register_fonts(cx);
}
