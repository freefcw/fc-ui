use crate::capabilities::foundation::theme::{use_theme, Theme};
use crate::capabilities::primitives::icon::Icon;
use crate::capabilities::primitives::icon_source::IconSource;
use gpui::{prelude::FluentBuilder as _, *};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileNodeKind {
    File,
    Directory,
    Symlink,
}

/// Icon and icon color used to present one file-tree node.
#[derive(Clone, Debug)]
pub struct FileNodePresentation {
    pub icon: IconSource,
    pub color: Hsla,
}

type PresentationResolver =
    Arc<dyn Fn(&FileNode, bool, &Theme) -> Option<FileNodePresentation> + Send + Sync>;

fn default_file_presentation(node: &FileNode, is_expanded: bool) -> (&'static str, Option<u32>) {
    match node.kind {
        FileNodeKind::Directory => (
            if is_expanded { "folder-open" } else { "folder" },
            Some(0x60a5fa),
        ),
        FileNodeKind::Symlink => ("link", None),
        FileNodeKind::File => match node.extension() {
            Some("json" | "yaml" | "yml" | "toml" | "xml") => ("file-json", Some(0xfbbf24)),
            Some("md" | "txt" | "doc" | "docx" | "pdf") => ("file-text", Some(0xa78bfa)),
            Some("sh" | "bash" | "zsh") => ("hash", Some(0x4ade80)),
            Some("png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp") => {
                ("image", Some(0x22c55e))
            }
            Some("mp3" | "wav" | "ogg" | "flac") => ("music", Some(0xf472b6)),
            Some("mp4" | "mov" | "avi" | "webm") => ("video", Some(0xf472b6)),
            Some("zip" | "tar" | "gz" | "rar" | "7z") => ("archive", Some(0xfbbf24)),
            _ => ("file-code", Some(0x9ca3af)),
        },
    }
}

fn resolve_file_presentation(
    node: &FileNode,
    is_expanded: bool,
    theme: &Theme,
    resolver: Option<&PresentationResolver>,
) -> FileNodePresentation {
    resolver
        .and_then(|resolver| resolver(node, is_expanded, theme))
        .unwrap_or_else(|| node.presentation(is_expanded, theme))
}

#[derive(Clone, Debug)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub kind: FileNodeKind,
    pub children: Vec<FileNode>,
    pub size: Option<u64>,
    pub modified: Option<String>,
    pub is_hidden: bool,
    pub has_unloaded_children: bool,
}

impl FileNode {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            path,
            name,
            kind: FileNodeKind::File,
            children: Vec::new(),
            size: None,
            modified: None,
            is_hidden: false,
            has_unloaded_children: false,
        }
    }

    pub fn directory(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            path,
            name,
            kind: FileNodeKind::Directory,
            children: Vec::new(),
            size: None,
            modified: None,
            is_hidden: false,
            has_unloaded_children: false,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_children(mut self, children: Vec<FileNode>) -> Self {
        self.children = children;
        self
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_modified(mut self, modified: impl Into<String>) -> Self {
        self.modified = Some(modified.into());
        self
    }

    pub fn hidden(mut self, is_hidden: bool) -> Self {
        self.is_hidden = is_hidden;
        self
    }

    pub fn with_unloaded_children(mut self, has_unloaded: bool) -> Self {
        self.has_unloaded_children = has_unloaded;
        self
    }

    pub fn is_directory(&self) -> bool {
        self.kind == FileNodeKind::Directory
    }

    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    pub fn file_icon(&self, is_expanded: bool) -> IconSource {
        IconSource::Named(default_file_presentation(self, is_expanded).0.into())
    }

    pub fn file_icon_color(&self, theme: &Theme) -> Hsla {
        default_file_presentation(self, false)
            .1
            .map_or(theme.tokens.muted_foreground, |color| rgb(color).into())
    }

    /// Default presentation, also usable as the starting point for a custom
    /// [`FileTree::presentation_resolver`].
    pub fn presentation(&self, is_expanded: bool, theme: &Theme) -> FileNodePresentation {
        let (icon, color) = default_file_presentation(self, is_expanded);
        FileNodePresentation {
            icon: IconSource::Named(icon.into()),
            color: color.map_or(theme.tokens.muted_foreground, |color| rgb(color).into()),
        }
    }
}

#[derive(Clone)]
struct FlatFileNode {
    node: FileNode,
    level: usize,
}

fn sort_file_nodes(nodes: &mut [FileNode]) {
    nodes.sort_by(|a, b| match (a.is_directory(), b.is_directory()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    for node in nodes.iter_mut() {
        if !node.children.is_empty() {
            sort_file_nodes(&mut node.children);
        }
    }
}

fn flatten_file_tree(
    nodes: &[FileNode],
    expanded_paths: &HashSet<PathBuf>,
    level: usize,
    show_hidden: bool,
) -> Vec<FlatFileNode> {
    let mut flat = Vec::new();

    for node in nodes {
        if node.is_hidden && !show_hidden {
            continue;
        }

        flat.push(FlatFileNode {
            node: node.clone(),
            level,
        });

        let has_children = !node.children.is_empty() || node.has_unloaded_children;
        if has_children && expanded_paths.contains(&node.path) {
            let children =
                flatten_file_tree(&node.children, expanded_paths, level + 1, show_hidden);
            flat.extend(children);
        }
    }

    flat
}

fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

const ROW_HEIGHT: f32 = 28.0;

#[derive(IntoElement)]
pub struct FileTree {
    nodes: Vec<FileNode>,
    selected_path: Option<PathBuf>,
    expanded_paths: Vec<PathBuf>,
    show_hidden: bool,
    show_file_size: bool,
    presentation_resolver: Option<PresentationResolver>,
    on_select: Option<Arc<dyn Fn(&PathBuf, &mut Window, &mut App) + Send + Sync>>,
    on_open: Option<Arc<dyn Fn(&PathBuf, &mut Window, &mut App) + Send + Sync>>,
    on_toggle: Option<Arc<dyn Fn(&PathBuf, bool, &mut Window, &mut App) + Send + Sync>>,
    on_context_menu:
        Option<Arc<dyn Fn(&PathBuf, Point<Pixels>, &mut Window, &mut App) + Send + Sync>>,
    style: StyleRefinement,
}

impl FileTree {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            selected_path: None,
            expanded_paths: Vec::new(),
            show_hidden: false,
            show_file_size: false,
            presentation_resolver: None,
            on_select: None,
            on_open: None,
            on_toggle: None,
            on_context_menu: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn nodes(mut self, mut nodes: Vec<FileNode>) -> Self {
        sort_file_nodes(&mut nodes);
        self.nodes = nodes;
        self
    }

    pub fn selected_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.selected_path = Some(path.into());
        self
    }

    pub fn expanded_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.expanded_paths = paths;
        self
    }

    pub fn show_hidden(mut self, show: bool) -> Self {
        self.show_hidden = show;
        self
    }

    pub fn show_file_size(mut self, show: bool) -> Self {
        self.show_file_size = show;
        self
    }

    /// Override icon/color per node (for example custom extensions or Git
    /// status). Return `None` to retain the default presentation. Selected
    /// rows retain the theme's accent-foreground icon color.
    pub fn presentation_resolver<F>(mut self, resolver: F) -> Self
    where
        F: Fn(&FileNode, bool, &Theme) -> Option<FileNodePresentation> + Send + Sync + 'static,
    {
        self.presentation_resolver = Some(Arc::new(resolver));
        self
    }

    pub fn on_select<F>(mut self, handler: F) -> Self
    where
        F: Fn(&PathBuf, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_select = Some(Arc::new(handler));
        self
    }

    pub fn on_open<F>(mut self, handler: F) -> Self
    where
        F: Fn(&PathBuf, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_open = Some(Arc::new(handler));
        self
    }

    pub fn on_toggle<F>(mut self, handler: F) -> Self
    where
        F: Fn(&PathBuf, bool, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_toggle = Some(Arc::new(handler));
        self
    }

    pub fn on_context_menu<F>(mut self, handler: F) -> Self
    where
        F: Fn(&PathBuf, Point<Pixels>, &mut Window, &mut App) + Send + Sync + 'static,
    {
        self.on_context_menu = Some(Arc::new(handler));
        self
    }
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for FileTree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FileTree {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let user_style = self.style;

        let expanded_set: HashSet<PathBuf> = self.expanded_paths.into_iter().collect();
        let flat_nodes = flatten_file_tree(&self.nodes, &expanded_set, 0, self.show_hidden);

        let selected_path = self.selected_path;
        let on_select = self.on_select;
        let on_open = self.on_open;
        let on_toggle = self.on_toggle;
        let on_context_menu = self.on_context_menu;
        let show_file_size = self.show_file_size;
        let presentation_resolver = self.presentation_resolver;

        div()
            .flex()
            .flex_col()
            .w_full()
            .bg(gpui::transparent_black())
            .map(|mut this| {
                this.style().refine(&user_style);
                this
            })
            .children(flat_nodes.into_iter().map(|flat_node| {
                let is_selected = selected_path.as_ref() == Some(&flat_node.node.path);
                let is_expanded = expanded_set.contains(&flat_node.node.path);
                let has_children =
                    !flat_node.node.children.is_empty() || flat_node.node.has_unloaded_children;
                let indent = px((flat_node.level as f32) * 16.0);
                let node = flat_node.node;
                let path = node.path.clone();

                let presentation = resolve_file_presentation(
                    &node,
                    is_expanded,
                    &theme,
                    presentation_resolver.as_ref(),
                );
                let icon_color = presentation.color;
                let node_icon = presentation.icon;

                div()
                    .id(SharedString::from(path.to_string_lossy().to_string()))
                    .w_full()
                    .h(px(ROW_HEIGHT))
                    .flex()
                    .items_center()
                    .mx(px(8.0))
                    .px(px(8.0))
                    .pl(indent + px(8.0))
                    .rounded(px(8.0))
                    .cursor_pointer()
                    .bg(if is_selected {
                        theme.tokens.accent
                    } else {
                        gpui::transparent_black()
                    })
                    .text_color(if is_selected {
                        theme.tokens.accent_foreground
                    } else if node.is_hidden {
                        theme.tokens.muted_foreground
                    } else {
                        theme.tokens.foreground
                    })
                    .when(!is_selected, |d| {
                        d.hover(|s| s.bg(theme.tokens.accent.opacity(0.5)))
                    })
                    .on_click({
                        let path = path.clone();
                        let on_select = on_select.clone();
                        let on_toggle = on_toggle.clone();
                        let on_open = on_open.clone();
                        let is_dir = node.is_directory();

                        move |event, window, cx| {
                            if let Some(ref handler) = on_select {
                                handler(&path, window, cx);
                            }

                            if is_dir {
                                if let Some(ref handler) = on_toggle {
                                    handler(&path, !is_expanded, window, cx);
                                }
                            } else if event.click_count() == 2 {
                                if let Some(ref handler) = on_open {
                                    handler(&path, window, cx);
                                }
                            }
                        }
                    })
                    .on_mouse_down(MouseButton::Right, {
                        let path = path.clone();
                        let on_context_menu = on_context_menu.clone();

                        move |event, window, cx| {
                            if let Some(ref handler) = on_context_menu {
                                handler(&path, event.position, window, cx);
                            }
                        }
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .flex_1()
                            .child(
                                div()
                                    .w(px(16.0))
                                    .h(px(16.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(has_children, |d| {
                                        d.child(
                                            Icon::new(if is_expanded {
                                                "chevron-down"
                                            } else {
                                                "chevron-right"
                                            })
                                            .size(px(12.0))
                                            .color(theme.tokens.muted_foreground),
                                        )
                                    }),
                            )
                            .child(Icon::new(node_icon).size(px(16.0)).color(if is_selected {
                                theme.tokens.accent_foreground
                            } else {
                                icon_color
                            }))
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(13.0))
                                    .font_family(theme.tokens.font_family.clone())
                                    .when(node.is_hidden, |d| d.opacity(0.6))
                                    .child(node.name.clone()),
                            )
                            .when(
                                show_file_size && node.size.is_some() && !node.is_directory(),
                                |d| {
                                    d.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.tokens.muted_foreground)
                                            .child(format_size(node.size.unwrap())),
                                    )
                                },
                            ),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_named_icon(icon: &IconSource, expected: &str) {
        assert!(matches!(icon, IconSource::Named(name) if name.as_str() == expected));
    }

    #[::core::prelude::v1::test]
    fn default_file_presentation_preserves_extension_icons_and_colors() {
        let theme = Theme::dark();
        let extensions = [
            ("json", "file-json", 0xfbbf24),
            ("yaml", "file-json", 0xfbbf24),
            ("yml", "file-json", 0xfbbf24),
            ("toml", "file-json", 0xfbbf24),
            ("xml", "file-json", 0xfbbf24),
            ("md", "file-text", 0xa78bfa),
            ("txt", "file-text", 0xa78bfa),
            ("doc", "file-text", 0xa78bfa),
            ("docx", "file-text", 0xa78bfa),
            ("pdf", "file-text", 0xa78bfa),
            ("sh", "hash", 0x4ade80),
            ("bash", "hash", 0x4ade80),
            ("zsh", "hash", 0x4ade80),
            ("png", "image", 0x22c55e),
            ("jpg", "image", 0x22c55e),
            ("jpeg", "image", 0x22c55e),
            ("gif", "image", 0x22c55e),
            ("svg", "image", 0x22c55e),
            ("ico", "image", 0x22c55e),
            ("webp", "image", 0x22c55e),
            ("mp3", "music", 0xf472b6),
            ("wav", "music", 0xf472b6),
            ("ogg", "music", 0xf472b6),
            ("flac", "music", 0xf472b6),
            ("mp4", "video", 0xf472b6),
            ("mov", "video", 0xf472b6),
            ("avi", "video", 0xf472b6),
            ("webm", "video", 0xf472b6),
            ("zip", "archive", 0xfbbf24),
            ("tar", "archive", 0xfbbf24),
            ("gz", "archive", 0xfbbf24),
            ("rar", "archive", 0xfbbf24),
            ("7z", "archive", 0xfbbf24),
            ("rs", "file-code", 0x9ca3af),
            ("JSON", "file-code", 0x9ca3af),
        ];
        for (extension, icon, color) in extensions {
            let node = FileNode::file(format!("file.{extension}"));
            let presentation = node.presentation(false, &theme);
            assert_named_icon(&presentation.icon, icon);
            assert_named_icon(&node.file_icon(false), icon);
            assert_eq!(presentation.color, Hsla::from(rgb(color)));
            assert_eq!(node.file_icon_color(&theme), presentation.color);
        }
        let directory = FileNode::directory("folder");
        assert_named_icon(&directory.presentation(false, &theme).icon, "folder");
        assert_named_icon(&directory.presentation(true, &theme).icon, "folder-open");
        assert_eq!(directory.file_icon_color(&theme), Hsla::from(rgb(0x60a5fa)));
        let mut symlink = FileNode::file("link");
        symlink.kind = FileNodeKind::Symlink;
        assert_named_icon(&symlink.presentation(false, &theme).icon, "link");
        assert_eq!(
            symlink.file_icon_color(&theme),
            theme.tokens.muted_foreground
        );
    }

    #[::core::prelude::v1::test]
    fn resolver_overrides_custom_extensions_and_git_colors_with_default_fallback() {
        let theme = Theme::light();
        let tree = FileTree::new().presentation_resolver(|node, expanded, theme| {
            if node.extension() == Some("custom") {
                Some(FileNodePresentation {
                    icon: "star".into(),
                    color: rgb(0xff0000).into(),
                })
            } else if node.name == "modified.rs" {
                let mut presentation = node.presentation(expanded, theme);
                presentation.color = rgb(0x00ff00).into();
                Some(presentation)
            } else {
                None
            }
        });
        let resolve = |node: &FileNode, expanded| {
            resolve_file_presentation(node, expanded, &theme, tree.presentation_resolver.as_ref())
        };
        let custom = resolve(&FileNode::file("file.custom"), false);
        assert_named_icon(&custom.icon, "star");
        assert_eq!(custom.color, Hsla::from(rgb(0xff0000)));
        let modified = resolve(&FileNode::file("modified.rs"), false);
        assert_named_icon(&modified.icon, "file-code");
        assert_eq!(modified.color, Hsla::from(rgb(0x00ff00)));
        let fallback = resolve(&FileNode::directory("folder"), true);
        assert_named_icon(&fallback.icon, "folder-open");
        assert_eq!(fallback.color, Hsla::from(rgb(0x60a5fa)));
    }
}
