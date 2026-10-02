pub mod app_menu;
pub mod breadcrumbs;
pub mod command_palette;
pub mod dock;
pub mod drawer_navigation;
pub mod file_tree;
pub mod floating_action_button;
pub mod keyboard_shortcuts;
pub mod menu;
pub mod navigation_menu;
pub mod pagination;
pub mod segmented_nav;
pub mod sidebar;
pub mod status_bar;
pub mod tabs;
pub mod toolbar;
pub mod tree;
pub mod view_router;

pub(crate) fn init(cx: &mut gpui::App) {
    menu::init_menu_bar(cx);
    sidebar::init_sidebar(cx);
    tabs::init_tabs(cx);
}
