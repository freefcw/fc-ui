//! Compatibility facade for legacy navigation paths.

pub use crate::capabilities::navigation::{
    app_menu, breadcrumbs, file_tree, menu, sidebar, status_bar, tabs, toolbar, tree,
};
// `virtual_list` used to alias the deprecated `scroll::variable_virtual_list` module.
// That module has been removed in favour of the live `uniform_virtual_list`, so the
// legacy path now resolves to the recommended implementation.
pub use crate::capabilities::scroll::uniform_virtual_list as virtual_list;
