pub mod app_menu;
pub mod breadcrumbs;
pub mod command_palette;
pub mod dock;
pub mod drawer_navigation;
pub mod file_tree;
pub mod floating_action_button;
pub mod keyboard_shortcuts;
pub mod list;
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
    // Command palette bindings are appended last: they are scoped to the
    // palette's own "CommandPalette" key context and never shadow the
    // context-scoped bindings installed above.
    command_palette::init_command_palette(cx);
}
