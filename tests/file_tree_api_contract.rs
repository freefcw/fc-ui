use fc_ui::{
    components::IconSource,
    navigation::file_tree::{FileNode, FileNodeKind, FileNodePresentation, FileTree},
};
use std::path::PathBuf;

#[test]
fn file_node_literals_and_presentation_resolvers_remain_accessible() {
    let node = FileNode {
        path: PathBuf::from("example.proto"),
        name: "example.proto".into(),
        kind: FileNodeKind::File,
        children: vec![],
        size: None,
        modified: None,
        is_hidden: false,
        has_unloaded_children: false,
    };
    let _ = FileTree::new()
        .nodes(vec![node])
        .presentation_resolver(|node, expanded, theme| {
            (node.extension() == Some("proto")).then(|| FileNodePresentation {
                icon: IconSource::Named("file-proto".into()),
                ..node.presentation(expanded, theme)
            })
        });
}
