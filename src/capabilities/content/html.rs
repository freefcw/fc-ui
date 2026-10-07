#[cfg(feature = "html-render")]
use html5ever::parse_document;
#[cfg(feature = "html-render")]
use html5ever::tendril::TendrilSink;
#[cfg(feature = "html-render")]
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use gpui::*;

use std::rc::Rc;

use crate::capabilities::content::rich_text::LinkClickHandler;
#[cfg(feature = "html-render")]
use crate::capabilities::content::rich_text::{
    render_blocks, ListItem, RichBlock, RichInline, TableAlignment,
};
#[cfg(feature = "html-render")]
use crate::capabilities::foundation::theme::use_theme;

#[cfg(feature = "html-render")]
static ALLOWED_TAGS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "br",
    "hr",
    "strong",
    "b",
    "em",
    "i",
    "s",
    "strike",
    "del",
    "code",
    "pre",
    "a",
    "img",
    "ul",
    "ol",
    "li",
    "blockquote",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
    "span",
    "div",
];

/// Renders trusted-or-untrusted HTML source as rich text blocks, limited to
/// the whitelisted tags in `ALLOWED_TAGS`.
///
/// Input is rejected before parsing when it exceeds 4 MiB or nests deeper
/// than 4096 elements: html5ever has no depth cap and degrades quadratically
/// (servo/html5ever#788). Rejected input renders as empty and logs to stderr.
#[derive(IntoElement)]
pub struct Html {
    base: Div,
    source: SharedString,
    base_font_size: Option<Pixels>,
    on_link_click: Option<LinkClickHandler>,
}

impl Html {
    pub fn new(source: impl Into<SharedString>) -> Self {
        Self {
            base: div(),
            source: source.into(),
            base_font_size: None,
            on_link_click: None,
        }
    }

    pub fn base_font_size(mut self, size: Pixels) -> Self {
        self.base_font_size = Some(size);
        self
    }

    pub fn on_link_click(
        mut self,
        handler: impl Fn(&str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_link_click = Some(Rc::new(handler));
        self
    }
}

/// Upper bound on element nesting depth. html5ever's tree builder grows
/// quadratically with nesting depth and imposes no cap of its own, so a few
/// hundred KB of nested markup can occupy the UI thread for seconds to minutes
/// (servo/html5ever#788). Chromium and WebKit stop nesting at 512; this is
/// deliberately more permissive while still bounding the work.
#[cfg(feature = "html-render")]
const MAX_NESTING_DEPTH: usize = 4096;

/// Upper bound on input length. Depth is the dominant cost, but bounding length
/// keeps a single pathological document from monopolizing the parse.
#[cfg(feature = "html-render")]
const MAX_SOURCE_LEN: usize = 4 * 1024 * 1024;

/// Scan `source` without building a tree and report the deepest element nesting
/// it reaches. Only tags are counted, so text containing `<` is not charged as
/// markup. Unbalanced closing tags are ignored rather than treated as errors —
/// html5ever tolerates them and the caller only needs a safe upper bound.
/// Self-closing slashes count as opening tags: HTML5 ignores them outside
/// foreign content, so `<div/>` realises as nesting in the parsed tree.
#[cfg(feature = "html-render")]
fn max_nesting_depth(source: &str) -> usize {
    const VOID_ELEMENTS: &[&str] = &[
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
        "source", "track", "wbr",
    ];

    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }

        // Skip comments and declarations wholesale.
        if source[i..].starts_with("<!--") {
            match source[i + 4..].find("-->") {
                Some(offset) => {
                    i += 4 + offset + 3;
                    continue;
                }
                None => break,
            }
        }
        if source[i..].starts_with("<!") || source[i..].starts_with("<?") {
            match source[i..].find('>') {
                Some(offset) => {
                    i += offset + 1;
                    continue;
                }
                None => break,
            }
        }

        let closing = bytes.get(i + 1) == Some(&b'/');
        let name_start = if closing { i + 2 } else { i + 1 };
        let mut name_end = name_start;
        while name_end < bytes.len()
            && (bytes[name_end].is_ascii_alphanumeric() || bytes[name_end] == b'-')
        {
            name_end += 1;
        }
        if name_end == name_start {
            // Not a tag after all (e.g. a bare `<`); move past it.
            i += 1;
            continue;
        }
        let name = source[name_start..name_end].to_ascii_lowercase();

        let tag_end = match source[name_end..].find('>') {
            Some(offset) => name_end + offset,
            None => break,
        };
        if closing {
            depth = depth.saturating_sub(1);
        } else if !VOID_ELEMENTS.contains(&name.as_str()) {
            // A trailing `/` does not exempt the tag: per HTML5, html5ever
            // ignores the self-closing slash on non-void elements, so `<div/>`
            // nests just like `<div>` and must be charged a level. Foreign
            // content (SVG/MathML) self-closing tags are over-counted here,
            // which fails safe.
            depth += 1;
            if depth > max_depth {
                max_depth = depth;
            }
        }

        i = tag_end + 1;
    }

    max_depth
}

#[cfg(feature = "html-render")]
fn parse_html(source: &str) -> Vec<RichBlock> {
    if source.len() > MAX_SOURCE_LEN || max_nesting_depth(source) > MAX_NESTING_DEPTH {
        eprintln!(
            "adabraka-ui: html-render refused to parse {} bytes with nesting deeper than {} \
             (html5ever has no depth cap and degrades quadratically)",
            source.len(),
            MAX_NESTING_DEPTH
        );
        return Vec::new();
    }

    let wrapped = format!("<html><body>{}</body></html>", source);
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut wrapped.as_bytes())
        .expect("html parse");

    let body = find_body(&dom.document);
    match body {
        Some(node) => walk_children_blocks(&node),
        None => Vec::new(),
    }
}

#[cfg(feature = "html-render")]
fn find_body(node: &Handle) -> Option<Handle> {
    match &node.data {
        NodeData::Element { name, .. } if name.local.as_ref() == "body" => {
            return Some(node.clone());
        }
        _ => {}
    }
    for child in node.children.borrow().iter() {
        if let Some(found) = find_body(child) {
            return Some(found);
        }
    }
    None
}

#[cfg(feature = "html-render")]
fn flush_inlines(pending: &mut Vec<RichInline>, blocks: &mut Vec<RichBlock>) {
    if !pending.is_empty() {
        let mut inlines = std::mem::take(pending);
        collapse_boundary_spaces(&mut inlines);
        trim_inlines(&mut inlines);
        if !inlines.is_empty() {
            blocks.push(RichBlock::Paragraph(inlines));
        }
    }
}

#[cfg(feature = "html-render")]
fn walk_children_blocks(node: &Handle) -> Vec<RichBlock> {
    let mut blocks = Vec::new();
    let mut pending_inlines: Vec<RichInline> = Vec::new();

    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Element { name, .. } => {
                let tag = name.local.as_ref();
                if !is_allowed_tag(tag) {
                    continue;
                }
                match tag {
                    "p" | "div" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let inlines = walk_block_inlines(child);
                        if !inlines.is_empty() {
                            blocks.push(RichBlock::Paragraph(inlines));
                        }
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let level = tag.as_bytes()[1] - b'0';
                        let inlines = walk_block_inlines(child);
                        blocks.push(RichBlock::Heading {
                            level,
                            content: inlines,
                        });
                    }
                    "pre" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let (lang, code) = extract_code_block(child);
                        blocks.push(RichBlock::CodeBlock {
                            language: lang,
                            code,
                        });
                    }
                    "blockquote" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let inner = walk_children_blocks(child);
                        blocks.push(RichBlock::BlockQuote(inner));
                    }
                    "ul" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let items = walk_list_items(child);
                        blocks.push(RichBlock::UnorderedList { items });
                    }
                    "ol" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let start = get_attr(child, "start")
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(1);
                        let items = walk_list_items(child);
                        blocks.push(RichBlock::OrderedList { start, items });
                    }
                    "hr" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        blocks.push(RichBlock::HorizontalRule);
                    }
                    "table" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let table = walk_table(child);
                        blocks.push(table);
                    }
                    "img" => {
                        flush_inlines(&mut pending_inlines, &mut blocks);
                        let src = get_attr(child, "src").unwrap_or_default();
                        let alt = get_attr(child, "alt").unwrap_or_default();
                        blocks.push(RichBlock::Image { alt, url: src });
                    }
                    _ => {
                        let inlines = walk_children_inlines(child);
                        pending_inlines.extend(inlines);
                    }
                }
            }
            NodeData::Text { contents } => {
                let text = collapse_html_whitespace(&contents.borrow());
                if !text.trim().is_empty() {
                    pending_inlines.push(RichInline::Text(text));
                }
            }
            _ => {}
        }
    }

    flush_inlines(&mut pending_inlines, &mut blocks);
    blocks
}

#[cfg(feature = "html-render")]
fn walk_children_inlines(node: &Handle) -> Vec<RichInline> {
    let mut inlines = Vec::new();

    for child in node.children.borrow().iter() {
        match &child.data {
            NodeData::Text { contents } => {
                let text = collapse_html_whitespace(&contents.borrow());
                if !text.is_empty() {
                    inlines.push(RichInline::Text(text));
                }
            }
            NodeData::Element { name, attrs: _, .. } => {
                let tag = name.local.as_ref();
                if !is_allowed_tag(tag) {
                    continue;
                }
                match tag {
                    "strong" | "b" => {
                        let children = walk_children_inlines(child);
                        inlines.push(RichInline::Bold(children));
                    }
                    "em" | "i" => {
                        let children = walk_children_inlines(child);
                        inlines.push(RichInline::Italic(children));
                    }
                    "s" | "strike" | "del" => {
                        let children = walk_children_inlines(child);
                        inlines.push(RichInline::Strikethrough(children));
                    }
                    "code" => {
                        let text = collect_text(child);
                        inlines.push(RichInline::Code(text));
                    }
                    "a" => {
                        let href = get_attr(child, "href").unwrap_or_default();
                        let children = walk_children_inlines(child);
                        inlines.push(RichInline::Link {
                            text: children,
                            url: href,
                        });
                    }
                    "br" => {
                        inlines.push(RichInline::LineBreak);
                    }
                    "img" => {
                        let src = get_attr(child, "src").unwrap_or_default();
                        let alt = get_attr(child, "alt").unwrap_or_default();
                        inlines.push(RichInline::Image { alt, url: src });
                    }
                    "span" | "div" => {
                        let children = walk_children_inlines(child);
                        let style_attr = get_attr(child, "style");
                        if let Some(style_str) = style_attr {
                            let parsed = parse_inline_style(&style_str);
                            if parsed.has_any() {
                                inlines.push(RichInline::Styled {
                                    children,
                                    color: parsed.color,
                                    background_color: parsed.background_color,
                                    bold: parsed.bold,
                                    italic: parsed.italic,
                                    font_size: parsed.font_size,
                                });
                            } else {
                                inlines.extend(children);
                            }
                        } else {
                            inlines.extend(children);
                        }
                    }
                    "ul" | "ol" => {}
                    _ => {
                        let children = walk_children_inlines(child);
                        inlines.extend(children);
                    }
                }
            }
            _ => {}
        }
    }

    collapse_boundary_spaces(&mut inlines);
    inlines
}

#[cfg(feature = "html-render")]
fn walk_block_inlines(node: &Handle) -> Vec<RichInline> {
    let mut inlines = walk_children_inlines(node);
    trim_inlines(&mut inlines);
    inlines
}

#[cfg(feature = "html-render")]
fn collapse_html_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_whitespace = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !in_whitespace {
                out.push(' ');
                in_whitespace = true;
            }
        } else {
            out.push(ch);
            in_whitespace = false;
        }
    }
    out
}

#[cfg(feature = "html-render")]
fn starts_with_space(inline: &RichInline) -> bool {
    match inline {
        RichInline::Text(s) => s.starts_with(' '),
        RichInline::Bold(children)
        | RichInline::Italic(children)
        | RichInline::Strikethrough(children) => children.first().is_some_and(starts_with_space),
        RichInline::Link { text, .. } => text.first().is_some_and(starts_with_space),
        RichInline::Styled { children, .. } => children.first().is_some_and(starts_with_space),
        _ => false,
    }
}

#[cfg(feature = "html-render")]
fn ends_with_space(inline: &RichInline) -> bool {
    match inline {
        RichInline::Text(s) => s.ends_with(' '),
        RichInline::Bold(children)
        | RichInline::Italic(children)
        | RichInline::Strikethrough(children) => children.last().is_some_and(ends_with_space),
        RichInline::Link { text, .. } => text.last().is_some_and(ends_with_space),
        RichInline::Styled { children, .. } => children.last().is_some_and(ends_with_space),
        _ => false,
    }
}

#[cfg(feature = "html-render")]
fn strip_leading_space(inline: &mut RichInline) {
    match inline {
        RichInline::Text(s) => {
            *s = s.trim_start().to_string();
        }
        RichInline::Bold(children)
        | RichInline::Italic(children)
        | RichInline::Strikethrough(children) => {
            if let Some(first) = children.first_mut() {
                strip_leading_space(first);
            }
        }
        RichInline::Link { text, .. } => {
            if let Some(first) = text.first_mut() {
                strip_leading_space(first);
            }
        }
        RichInline::Styled { children, .. } => {
            if let Some(first) = children.first_mut() {
                strip_leading_space(first);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "html-render")]
fn strip_trailing_space(inline: &mut RichInline) {
    match inline {
        RichInline::Text(s) => {
            *s = s.trim_end().to_string();
        }
        RichInline::Bold(children)
        | RichInline::Italic(children)
        | RichInline::Strikethrough(children) => {
            if let Some(last) = children.last_mut() {
                strip_trailing_space(last);
            }
        }
        RichInline::Link { text, .. } => {
            if let Some(last) = text.last_mut() {
                strip_trailing_space(last);
            }
        }
        RichInline::Styled { children, .. } => {
            if let Some(last) = children.last_mut() {
                strip_trailing_space(last);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "html-render")]
fn is_empty_text(inline: &RichInline) -> bool {
    matches!(inline, RichInline::Text(s) if s.is_empty())
}

#[cfg(feature = "html-render")]
fn collapse_boundary_spaces(inlines: &mut Vec<RichInline>) {
    let mut after_space = false;
    let mut i = 0;
    while i < inlines.len() {
        if after_space && starts_with_space(&inlines[i]) {
            strip_leading_space(&mut inlines[i]);
            if is_empty_text(&inlines[i]) {
                inlines.remove(i);
                continue;
            }
        }
        after_space = ends_with_space(&inlines[i]);
        i += 1;
    }
}

#[cfg(feature = "html-render")]
fn trim_inlines(inlines: &mut Vec<RichInline>) {
    loop {
        match inlines.first() {
            Some(first) if starts_with_space(first) => {
                strip_leading_space(&mut inlines[0]);
                if is_empty_text(&inlines[0]) {
                    inlines.remove(0);
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
    loop {
        match inlines.last() {
            Some(last) if ends_with_space(last) => {
                let idx = inlines.len() - 1;
                strip_trailing_space(&mut inlines[idx]);
                if is_empty_text(&inlines[idx]) {
                    inlines.pop();
                } else {
                    break;
                }
            }
            _ => break,
        }
    }
}

#[cfg(feature = "html-render")]
fn extract_code_block(pre_node: &Handle) -> (Option<String>, String) {
    for child in pre_node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            if name.local.as_ref() == "code" {
                let lang = get_attr(child, "class").and_then(|cls| {
                    cls.strip_prefix("language-")
                        .map(|l| l.to_string())
                        .or_else(|| cls.strip_prefix("lang-").map(|l| l.to_string()))
                });
                let code = collect_text(child);
                return (lang, code);
            }
        }
    }
    (None, collect_text(pre_node))
}

#[cfg(feature = "html-render")]
fn walk_list_items(node: &Handle) -> Vec<ListItem> {
    let mut items = Vec::new();

    for child in node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            if name.local.as_ref() == "li" {
                let inlines = walk_block_inlines(child);
                let mut sub_items = Vec::new();

                for sub in child.children.borrow().iter() {
                    if let NodeData::Element { name: sub_name, .. } = &sub.data {
                        let sub_tag = sub_name.local.as_ref();
                        if sub_tag == "ul" || sub_tag == "ol" {
                            sub_items = walk_list_items(sub);
                        }
                    }
                }

                items.push(ListItem {
                    checked: None,
                    content: inlines,
                    children: sub_items,
                });
            }
        }
    }

    items
}

#[cfg(feature = "html-render")]
fn walk_table(node: &Handle) -> RichBlock {
    let mut headers: Vec<Vec<RichInline>> = Vec::new();
    let mut rows: Vec<Vec<Vec<RichInline>>> = Vec::new();
    let mut alignments: Vec<TableAlignment> = Vec::new();

    for child in node.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            match name.local.as_ref() {
                "thead" => {
                    for tr in child.children.borrow().iter() {
                        if let NodeData::Element { name: tr_name, .. } = &tr.data {
                            if tr_name.local.as_ref() == "tr" {
                                headers = walk_table_cells(tr, &mut alignments, true);
                            }
                        }
                    }
                }
                "tbody" => {
                    for tr in child.children.borrow().iter() {
                        if let NodeData::Element { name: tr_name, .. } = &tr.data {
                            if tr_name.local.as_ref() == "tr" {
                                let mut dummy = Vec::new();
                                let row = walk_table_cells(tr, &mut dummy, false);
                                rows.push(row);
                            }
                        }
                    }
                }
                "tr" => {
                    if headers.is_empty() {
                        headers = walk_table_cells(child, &mut alignments, true);
                    } else {
                        let mut dummy = Vec::new();
                        let row = walk_table_cells(child, &mut dummy, false);
                        rows.push(row);
                    }
                }
                _ => {}
            }
        }
    }

    RichBlock::Table {
        headers,
        alignments,
        rows,
    }
}

#[cfg(feature = "html-render")]
fn walk_table_cells(
    tr: &Handle,
    alignments: &mut Vec<TableAlignment>,
    is_header: bool,
) -> Vec<Vec<RichInline>> {
    let mut cells = Vec::new();

    for child in tr.children.borrow().iter() {
        if let NodeData::Element { name, .. } = &child.data {
            let tag = name.local.as_ref();
            if tag == "th" || tag == "td" {
                let inlines = walk_block_inlines(child);
                cells.push(inlines);

                if is_header {
                    let align = get_attr(child, "align")
                        .map(|a| match a.as_str() {
                            "center" => TableAlignment::Center,
                            "right" => TableAlignment::Right,
                            _ => TableAlignment::Left,
                        })
                        .unwrap_or(TableAlignment::Left);
                    alignments.push(align);
                }
            }
        }
    }

    cells
}

#[cfg(feature = "html-render")]
fn collect_text(node: &Handle) -> String {
    let mut out = String::new();
    collect_text_recursive(node, &mut out);
    out
}

#[cfg(feature = "html-render")]
fn collect_text_recursive(node: &Handle, out: &mut String) {
    match &node.data {
        NodeData::Text { contents } => {
            out.push_str(&contents.borrow());
        }
        _ => {
            for child in node.children.borrow().iter() {
                collect_text_recursive(child, out);
            }
        }
    }
}

#[cfg(feature = "html-render")]
fn get_attr(node: &Handle, attr_name: &str) -> Option<String> {
    if let NodeData::Element { attrs, .. } = &node.data {
        for attr in attrs.borrow().iter() {
            if attr.name.local.as_ref() == attr_name {
                return Some(attr.value.to_string());
            }
        }
    }
    None
}

#[cfg(feature = "html-render")]
fn is_allowed_tag(tag: &str) -> bool {
    ALLOWED_TAGS.contains(&tag)
}

#[cfg(feature = "html-render")]
struct InlineStyle {
    color: Option<Hsla>,
    background_color: Option<Hsla>,
    bold: Option<bool>,
    italic: Option<bool>,
    font_size: Option<f32>,
}

#[cfg(feature = "html-render")]
impl InlineStyle {
    fn has_any(&self) -> bool {
        self.color.is_some()
            || self.background_color.is_some()
            || self.bold.is_some()
            || self.italic.is_some()
            || self.font_size.is_some()
    }
}

#[cfg(feature = "html-render")]
fn parse_inline_style(style: &str) -> InlineStyle {
    let mut result = InlineStyle {
        color: None,
        background_color: None,
        bold: None,
        italic: None,
        font_size: None,
    };

    for declaration in style.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }
        if let Some((prop, value)) = declaration.split_once(':') {
            let prop = prop.trim().to_lowercase();
            let value = value.trim();

            match prop.as_str() {
                "color" => {
                    result.color = parse_css_color(value);
                }
                "background-color" | "background" => {
                    result.background_color = parse_css_color(value);
                }
                "font-weight" => match value {
                    "bold" | "700" | "800" | "900" => result.bold = Some(true),
                    "normal" | "400" => result.bold = Some(false),
                    _ => {}
                },
                "font-style" => match value {
                    "italic" | "oblique" => result.italic = Some(true),
                    "normal" => result.italic = Some(false),
                    _ => {}
                },
                "font-size" => {
                    if let Some(stripped) = value.strip_suffix("px") {
                        if let Ok(size) = stripped.trim().parse::<f32>() {
                            result.font_size = Some(size);
                        }
                    } else if let Some(stripped) = value.strip_suffix("em") {
                        if let Ok(size) = stripped.trim().parse::<f32>() {
                            result.font_size = Some(size * 14.0);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    result
}

#[cfg(feature = "html-render")]
fn parse_css_color(value: &str) -> Option<Hsla> {
    let value = value.trim();

    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    if let Some(inner) = value
        .strip_prefix("rgb(")
        .or_else(|| value.strip_prefix("RGB("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(rgb_to_hsla(r, g, b, 1.0));
        }
    }

    if let Some(inner) = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("RGBA("))
        .and_then(|s| s.strip_suffix(')'))
    {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a = parts[3].trim().parse::<f32>().ok()?;
            return Some(rgb_to_hsla(r, g, b, a));
        }
    }

    match value.to_lowercase().as_str() {
        "red" => Some(rgb_to_hsla(255, 0, 0, 1.0)),
        "green" => Some(rgb_to_hsla(0, 128, 0, 1.0)),
        "blue" => Some(rgb_to_hsla(0, 0, 255, 1.0)),
        "yellow" => Some(rgb_to_hsla(255, 255, 0, 1.0)),
        "orange" => Some(rgb_to_hsla(255, 165, 0, 1.0)),
        "purple" => Some(rgb_to_hsla(128, 0, 128, 1.0)),
        "white" => Some(rgb_to_hsla(255, 255, 255, 1.0)),
        "black" => Some(rgb_to_hsla(0, 0, 0, 1.0)),
        "gray" | "grey" => Some(rgb_to_hsla(128, 128, 128, 1.0)),
        "cyan" => Some(rgb_to_hsla(0, 255, 255, 1.0)),
        "magenta" => Some(rgb_to_hsla(255, 0, 255, 1.0)),
        "pink" => Some(rgb_to_hsla(255, 192, 203, 1.0)),
        "brown" => Some(rgb_to_hsla(165, 42, 42, 1.0)),
        "coral" => Some(rgb_to_hsla(255, 127, 80, 1.0)),
        "teal" => Some(rgb_to_hsla(0, 128, 128, 1.0)),
        "navy" => Some(rgb_to_hsla(0, 0, 128, 1.0)),
        "maroon" => Some(rgb_to_hsla(128, 0, 0, 1.0)),
        "lime" => Some(rgb_to_hsla(0, 255, 0, 1.0)),
        "olive" => Some(rgb_to_hsla(128, 128, 0, 1.0)),
        "silver" => Some(rgb_to_hsla(192, 192, 192, 1.0)),
        "gold" => Some(rgb_to_hsla(255, 215, 0, 1.0)),
        _ => None,
    }
}

#[cfg(feature = "html-render")]
fn parse_hex_color(hex: &str) -> Option<Hsla> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(rgb_to_hsla(r, g, b, 1.0))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(rgb_to_hsla(r, g, b, 1.0))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(rgb_to_hsla(r, g, b, a as f32 / 255.0))
        }
        _ => None,
    }
}

#[cfg(feature = "html-render")]
fn rgb_to_hsla(r: u8, g: u8, b: u8, a: f32) -> Hsla {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let lightness = (max + min) / 2.0;

    if delta < f32::EPSILON {
        return hsla(0.0, 0.0, lightness, a);
    }

    let saturation = if lightness <= 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let hue = if (max - rf).abs() < f32::EPSILON {
        ((gf - bf) / delta + if gf < bf { 6.0 } else { 0.0 }) / 6.0
    } else if (max - gf).abs() < f32::EPSILON {
        ((bf - rf) / delta + 2.0) / 6.0
    } else {
        ((rf - gf) / delta + 4.0) / 6.0
    };

    hsla(hue, saturation, lightness, a)
}

#[cfg(feature = "html-render")]
impl RenderOnce for Html {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let base_size = self.base_font_size.unwrap_or(px(14.0));

        let blocks = parse_html(&self.source);
        let elements = render_blocks(&blocks, base_size, &self.on_link_click, "html", cx);

        self.base
            .flex()
            .flex_col()
            .font_family(theme.tokens.font_family.clone())
            .text_color(theme.tokens.foreground)
            .children(elements)
    }
}

#[cfg(not(feature = "html-render"))]
impl RenderOnce for Html {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let _ = &self.source;
        self.base
            .child("Enable the 'html-render' feature to render HTML content.")
    }
}

impl Styled for Html {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

#[cfg(all(test, feature = "html-render"))]
mod tests {
    use super::{max_nesting_depth, parse_html, Html, MAX_NESTING_DEPTH};
    use gpui::{
        point, px, Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window,
    };
    use std::{cell::Cell, rc::Rc};

    struct HtmlLinkTestView {
        calls: Rc<Cell<usize>>,
    }

    impl Render for HtmlLinkTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            Html::new("<p><a href=\"https://example.com\">link</a></p>").on_link_click(
                move |url, _, _| {
                    assert_eq!(url, "https://example.com");
                    calls.set(calls.get() + 1);
                },
            )
        }
    }

    #[gpui::test]
    fn clicking_an_html_link_invokes_the_custom_handler(cx: &mut TestAppContext) {
        let calls = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view(|_, _| HtmlLinkTestView {
            calls: calls.clone(),
        });

        cx.update(|window, cx| window.draw(cx).clear());
        let link_bounds = cx
            .debug_bounds("rich-text-link")
            .expect("HTML link should have rendered bounds");
        let link_position = point(
            link_bounds.origin.x + px(2.0),
            link_bounds.origin.y + px(8.0),
        );
        cx.simulate_mouse_move(link_position, None, Modifiers::none());
        cx.simulate_mouse_down(link_position, MouseButton::Left, Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear());
        cx.simulate_mouse_up(link_position, MouseButton::Left, Modifiers::none());

        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn nested_list_text_is_not_duplicated_in_parent_item() {
        use super::parse_html;
        use crate::capabilities::content::rich_text::{RichBlock, RichInline};

        let blocks = parse_html("<ul><li>a\n  <ul><li>b</li></ul>\n</li></ul>");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RichBlock::UnorderedList { items } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].content.len(), 1);
                assert!(matches!(&items[0].content[0], RichInline::Text(t) if t == "a"));
                assert_eq!(items[0].children.len(), 1);
                assert!(
                    matches!(&items[0].children[0].content[0], RichInline::Text(t) if t == "b")
                );
            }
            other => panic!("expected unordered list, got {:?}", other),
        }
    }

    #[test]
    fn inline_whitespace_collapses_and_trims_at_block_edges() {
        use super::parse_html;
        use crate::capabilities::content::rich_text::{RichBlock, RichInline};

        let blocks = parse_html("<p>\n  hello\n   <strong>world</strong>\n</p>");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RichBlock::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 2);
                assert!(matches!(&inlines[0], RichInline::Text(t) if t == "hello "));
                assert!(
                    matches!(&inlines[1], RichInline::Bold(children) if children.len() == 1 && matches!(&children[0], RichInline::Text(t) if t == "world"))
                );
            }
            other => panic!("expected paragraph, got {:?}", other),
        }
    }

    #[test]
    fn nesting_depth_counts_only_open_elements() {
        assert_eq!(max_nesting_depth("plain text"), 0);
        assert_eq!(max_nesting_depth("<p>hello</p>"), 1);
        assert_eq!(max_nesting_depth("<div><p><strong>x</strong></p></div>"), 3);
        // Void elements do not open a level.
        assert_eq!(max_nesting_depth("<p><br><hr><img src=\"x\"></p>"), 1);
        // Self-closing slashes on non-void elements still open a level:
        // html5ever ignores the slash (HTML5), so these nest in the real tree.
        assert_eq!(max_nesting_depth("<p><span/></p>"), 2);
        assert_eq!(max_nesting_depth(&"<div/>".repeat(10)), 10);
        // Comments and declarations are skipped, including markup inside them.
        assert_eq!(max_nesting_depth("<!-- <div><div><div> --><p>x</p>"), 1);
        assert_eq!(max_nesting_depth("<!DOCTYPE html><p>x</p>"), 1);
        // Sibling elements do not accumulate depth.
        assert_eq!(max_nesting_depth("<p>a</p><p>b</p><p>c</p>"), 1);
        // A bare `<` in text is not a tag.
        assert_eq!(max_nesting_depth("1 < 2 and 3 < 4"), 0);
        // Unbalanced closing tags saturate rather than underflow.
        assert_eq!(max_nesting_depth("</div></div><p>x</p>"), 1);
    }

    #[test]
    fn deeply_nested_input_is_refused_instead_of_parsed() {
        let depth = MAX_NESTING_DEPTH + 1;
        let deep = format!("{}x{}", "<div>".repeat(depth), "</div>".repeat(depth));
        assert!(max_nesting_depth(&deep) > MAX_NESTING_DEPTH);
        assert!(parse_html(&deep).is_empty());
    }

    #[test]
    fn self_closing_chains_are_refused_like_explicit_nesting() {
        // html5ever ignores the slash on non-void elements, so `<div/>` nests.
        let deep = "<div/>".repeat(MAX_NESTING_DEPTH + 1);
        assert!(max_nesting_depth(&deep) > MAX_NESTING_DEPTH);
        assert!(parse_html(&deep).is_empty());
    }

    #[test]
    fn nesting_within_the_limit_still_parses() {
        let depth = 64;
        let nested = format!("{}x{}", "<div>".repeat(depth), "</div>".repeat(depth));
        assert_eq!(max_nesting_depth(&nested), depth);
        assert!(!parse_html(&nested).is_empty());
    }
}
