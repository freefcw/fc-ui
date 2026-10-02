#[cfg(feature = "markdown")]
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use gpui::*;

use std::rc::Rc;

use crate::capabilities::content::rich_text::LinkClickHandler;
#[cfg(feature = "markdown")]
use crate::capabilities::content::rich_text::{
    render_blocks, ListItem, RichBlock, RichInline, TableAlignment,
};
#[cfg(feature = "markdown")]
use crate::capabilities::foundation::theme::use_theme;

#[derive(IntoElement)]
pub struct Markdown {
    base: Div,
    source: SharedString,
    base_font_size: Option<Pixels>,
    on_link_click: Option<LinkClickHandler>,
}

impl Markdown {
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

#[cfg(feature = "markdown")]
fn heading_level_to_u8(level: &HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(feature = "markdown")]
fn inlines_to_plain_text(inlines: &[RichInline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            RichInline::Text(s) => out.push_str(s),
            RichInline::Bold(children)
            | RichInline::Italic(children)
            | RichInline::Strikethrough(children) => {
                out.push_str(&inlines_to_plain_text(children));
            }
            RichInline::Code(s) => out.push_str(s),
            RichInline::Link { text, .. } => {
                out.push_str(&inlines_to_plain_text(text));
            }
            RichInline::Image { alt, .. } => out.push_str(alt),
            RichInline::LineBreak => out.push('\n'),
            RichInline::Html(_) => {}
            RichInline::Styled { children, .. } => {
                out.push_str(&inlines_to_plain_text(children));
            }
        }
    }
    out
}

#[cfg(feature = "markdown")]
impl RenderOnce for Markdown {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = use_theme(cx);
        let base_size = self.base_font_size.unwrap_or(px(14.0));

        let blocks = parse_markdown_with_urls(&self.source);
        let elements = render_blocks(&blocks, base_size, &self.on_link_click, "md", cx);

        self.base
            .flex()
            .flex_col()
            .font_family(theme.tokens.font_family.clone())
            .text_color(theme.tokens.foreground)
            .children(elements)
    }
}

#[cfg(not(feature = "markdown"))]
impl RenderOnce for Markdown {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let _ = &self.source;
        self.base
            .child("Enable the 'markdown' feature to render markdown content.")
    }
}

impl Styled for Markdown {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

#[cfg(feature = "markdown")]
struct ListState {
    ordered: bool,
    start: u64,
    items: Vec<ListItem>,
    current_item_inlines: Vec<RichInline>,
    current_item_checked: Option<bool>,
    current_item_children: Vec<ListItem>,
}

#[cfg(feature = "markdown")]
struct TableState {
    headers: Vec<Vec<RichInline>>,
    alignments: Vec<TableAlignment>,
    rows: Vec<Vec<Vec<RichInline>>>,
    current_row: Vec<Vec<RichInline>>,
    in_head: bool,
}

#[cfg(feature = "markdown")]
fn parse_markdown_with_urls(source: &str) -> Vec<RichBlock> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(source, options);
    let events: Vec<Event> = parser.collect();

    let mut builder = UrlTrackingBlockBuilder::new();
    builder.build(&events);
    builder.blocks
}

#[cfg(feature = "markdown")]
struct UrlTrackingBlockBuilder {
    blocks: Vec<RichBlock>,
    inline_stack: Vec<Vec<RichInline>>,
    list_stack: Vec<ListState>,
    blockquote_depth: usize,
    blockquote_blocks: Vec<Vec<RichBlock>>,
    table_state: Option<TableState>,
    current_heading_level: Option<u8>,
    in_code_block: bool,
    code_block_lang: Option<String>,
    code_block_content: String,
    url_stack: Vec<String>,
}

#[cfg(feature = "markdown")]
impl UrlTrackingBlockBuilder {
    fn new() -> Self {
        Self {
            blocks: Vec::new(),
            inline_stack: Vec::new(),
            list_stack: Vec::new(),
            blockquote_depth: 0,
            blockquote_blocks: Vec::new(),
            table_state: None,
            current_heading_level: None,
            in_code_block: false,
            code_block_lang: None,
            code_block_content: String::new(),
            url_stack: Vec::new(),
        }
    }

    fn build(&mut self, events: &[Event]) {
        for event in events {
            self.process_event(event);
        }
    }

    fn process_event(&mut self, event: &Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag) => self.end_tag(tag),
            Event::Text(text) => self.text(text),
            Event::Code(code) => self.push_inline(RichInline::Code(code.to_string())),
            Event::SoftBreak => self.push_inline(RichInline::Text(" ".to_string())),
            Event::HardBreak => self.push_inline(RichInline::LineBreak),
            Event::Rule => self.push_block(RichBlock::HorizontalRule),
            Event::Html(html) => self.push_inline(RichInline::Html(html.to_string())),
            Event::TaskListMarker(checked) => {
                if let Some(list) = self.list_stack.last_mut() {
                    list.current_item_checked = Some(*checked);
                }
            }
            _ => {}
        }
    }

    fn start_tag(&mut self, tag: &Tag) {
        match tag {
            Tag::Paragraph => {
                self.inline_stack.push(Vec::new());
            }
            Tag::Heading { level, .. } => {
                self.current_heading_level = Some(heading_level_to_u8(level));
                self.inline_stack.push(Vec::new());
            }
            Tag::BlockQuote(_) => {
                self.blockquote_depth += 1;
                self.blockquote_blocks.push(Vec::new());
            }
            Tag::CodeBlock(kind) => {
                self.in_code_block = true;
                self.code_block_content.clear();
                match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        self.code_block_lang = if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str)
                        };
                    }
                    pulldown_cmark::CodeBlockKind::Indented => {
                        self.code_block_lang = None;
                    }
                }
            }
            Tag::List(start) => {
                self.list_stack.push(ListState {
                    ordered: start.is_some(),
                    start: start.unwrap_or(1),
                    items: Vec::new(),
                    current_item_inlines: Vec::new(),
                    current_item_checked: None,
                    current_item_children: Vec::new(),
                });
            }
            Tag::Item => {
                if let Some(list) = self.list_stack.last_mut() {
                    list.current_item_inlines.clear();
                    list.current_item_checked = None;
                    list.current_item_children.clear();
                }
                self.inline_stack.push(Vec::new());
            }
            Tag::Strong => {
                self.inline_stack.push(Vec::new());
            }
            Tag::Emphasis => {
                self.inline_stack.push(Vec::new());
            }
            Tag::Strikethrough => {
                self.inline_stack.push(Vec::new());
            }
            Tag::Link { dest_url, .. } => {
                self.url_stack.push(dest_url.to_string());
                self.inline_stack.push(Vec::new());
            }
            Tag::Image { dest_url, .. } => {
                self.url_stack.push(dest_url.to_string());
                self.inline_stack.push(Vec::new());
            }
            Tag::Table(alignments) => {
                self.table_state = Some(TableState {
                    headers: Vec::new(),
                    alignments: alignments
                        .iter()
                        .map(|a| match a {
                            pulldown_cmark::Alignment::Left => TableAlignment::Left,
                            pulldown_cmark::Alignment::Center => TableAlignment::Center,
                            pulldown_cmark::Alignment::Right => TableAlignment::Right,
                            pulldown_cmark::Alignment::None => TableAlignment::Left,
                        })
                        .collect(),
                    rows: Vec::new(),
                    current_row: Vec::new(),
                    in_head: false,
                });
            }
            Tag::TableHead => {
                if let Some(ref mut ts) = self.table_state {
                    ts.in_head = true;
                    ts.current_row.clear();
                }
            }
            Tag::TableRow => {
                if let Some(ref mut ts) = self.table_state {
                    ts.current_row.clear();
                }
            }
            Tag::TableCell => {
                self.inline_stack.push(Vec::new());
            }
            _ => {}
        }
    }

    fn end_tag(&mut self, tag: &TagEnd) {
        match tag {
            TagEnd::Paragraph => {
                let inlines = self.inline_stack.pop().unwrap_or_default();
                self.push_block(RichBlock::Paragraph(inlines));
            }
            TagEnd::Heading(_level) => {
                let inlines = self.inline_stack.pop().unwrap_or_default();
                let lvl = self.current_heading_level.take().unwrap_or(1);
                self.push_block(RichBlock::Heading {
                    level: lvl,
                    content: inlines,
                });
            }
            TagEnd::BlockQuote(_) => {
                self.blockquote_depth -= 1;
                let inner = self.blockquote_blocks.pop().unwrap_or_default();
                self.push_block(RichBlock::BlockQuote(inner));
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                let code = std::mem::take(&mut self.code_block_content);
                let lang = self.code_block_lang.take();
                self.push_block(RichBlock::CodeBlock {
                    language: lang,
                    code,
                });
            }
            TagEnd::List(_ordered) => {
                if let Some(list) = self.list_stack.pop() {
                    let block = if list.ordered {
                        RichBlock::OrderedList {
                            start: list.start,
                            items: list.items,
                        }
                    } else {
                        RichBlock::UnorderedList { items: list.items }
                    };
                    self.push_block(block);
                }
            }
            TagEnd::Item => {
                let inlines = self.inline_stack.pop().unwrap_or_default();
                if let Some(list) = self.list_stack.last_mut() {
                    list.items.push(ListItem {
                        checked: list.current_item_checked,
                        content: inlines,
                        children: std::mem::take(&mut list.current_item_children),
                    });
                }
            }
            TagEnd::Strong => {
                let children = self.inline_stack.pop().unwrap_or_default();
                self.push_inline(RichInline::Bold(children));
            }
            TagEnd::Emphasis => {
                let children = self.inline_stack.pop().unwrap_or_default();
                self.push_inline(RichInline::Italic(children));
            }
            TagEnd::Strikethrough => {
                let children = self.inline_stack.pop().unwrap_or_default();
                self.push_inline(RichInline::Strikethrough(children));
            }
            TagEnd::Link => {
                let children = self.inline_stack.pop().unwrap_or_default();
                let url = self.url_stack.pop().unwrap_or_default();
                self.push_inline(RichInline::Link {
                    text: children,
                    url,
                });
            }
            TagEnd::Image => {
                let alt_inlines = self.inline_stack.pop().unwrap_or_default();
                let alt = inlines_to_plain_text(&alt_inlines);
                let url = self.url_stack.pop().unwrap_or_default();
                self.push_inline(RichInline::Image { alt, url });
            }
            TagEnd::Table => {
                if let Some(ts) = self.table_state.take() {
                    self.push_block(RichBlock::Table {
                        headers: ts.headers,
                        alignments: ts.alignments,
                        rows: ts.rows,
                    });
                }
            }
            TagEnd::TableHead => {
                if let Some(ref mut ts) = self.table_state {
                    ts.headers = std::mem::take(&mut ts.current_row);
                    ts.in_head = false;
                }
            }
            TagEnd::TableRow => {
                if let Some(ref mut ts) = self.table_state {
                    if !ts.in_head {
                        let row = std::mem::take(&mut ts.current_row);
                        ts.rows.push(row);
                    }
                }
            }
            TagEnd::TableCell => {
                let inlines = self.inline_stack.pop().unwrap_or_default();
                if let Some(ref mut ts) = self.table_state {
                    ts.current_row.push(inlines);
                }
            }
            _ => {}
        }
    }

    fn text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_block_content.push_str(text);
            return;
        }
        self.push_inline(RichInline::Text(text.to_string()));
    }

    fn push_inline(&mut self, inline: RichInline) {
        if let Some(stack) = self.inline_stack.last_mut() {
            stack.push(inline);
        }
    }

    fn push_block(&mut self, block: RichBlock) {
        if let Some(list) = self.list_stack.last_mut() {
            match block {
                RichBlock::OrderedList { items, .. } | RichBlock::UnorderedList { items, .. } => {
                    list.current_item_children.extend(items);
                    return;
                }
                RichBlock::Paragraph(inlines) => {
                    if let Some(current) = self.inline_stack.last_mut() {
                        current.extend(inlines);
                    }
                    return;
                }
                _ => {}
            }
        }
        if self.blockquote_depth > 0 {
            if let Some(blocks) = self.blockquote_blocks.last_mut() {
                blocks.push(block);
                return;
            }
        }
        self.blocks.push(block);
    }
}

#[cfg(all(test, feature = "markdown"))]
mod tests {
    use super::Markdown;
    use gpui::{
        point, px, Context, IntoElement, Modifiers, MouseButton, Render, TestAppContext, Window,
    };
    use std::{cell::Cell, rc::Rc};

    struct MarkdownLinkTestView {
        calls: Rc<Cell<usize>>,
    }

    impl Render for MarkdownLinkTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            Markdown::new("[link](https://example.com)").on_link_click(move |url, _, _| {
                assert_eq!(url, "https://example.com");
                calls.set(calls.get() + 1);
            })
        }
    }

    #[gpui::test]
    fn clicking_a_markdown_link_invokes_the_custom_handler(cx: &mut TestAppContext) {
        let calls = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view(|_, _| MarkdownLinkTestView {
            calls: calls.clone(),
        });

        cx.update(|window, cx| window.draw(cx).clear());
        let link_bounds = cx
            .debug_bounds("rich-text-link")
            .expect("Markdown link should have rendered bounds");
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
    fn nested_list_attaches_to_parent_item() {
        use super::parse_markdown_with_urls;
        use crate::capabilities::content::rich_text::{RichBlock, RichInline};

        let blocks = parse_markdown_with_urls("- a\n  - b");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RichBlock::UnorderedList { items } => {
                assert_eq!(items.len(), 1);
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
    fn loose_list_item_content_stays_in_item() {
        use super::parse_markdown_with_urls;
        use crate::capabilities::content::rich_text::{RichBlock, RichInline};

        let blocks = parse_markdown_with_urls("- a\n\n- b");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RichBlock::UnorderedList { items } => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0].content[0], RichInline::Text(t) if t == "a"));
                assert!(matches!(&items[1].content[0], RichInline::Text(t) if t == "b"));
            }
            other => panic!("expected unordered list, got {:?}", other),
        }
    }

    #[test]
    fn inline_image_stays_in_paragraph_flow() {
        use super::parse_markdown_with_urls;
        use crate::capabilities::content::rich_text::{RichBlock, RichInline};

        let blocks = parse_markdown_with_urls("文字 ![x](u) 后缀");
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            RichBlock::Paragraph(inlines) => {
                assert_eq!(inlines.len(), 3);
                assert!(matches!(&inlines[0], RichInline::Text(t) if t == "文字 "));
                assert!(
                    matches!(&inlines[1], RichInline::Image { alt, url } if alt == "x" && url == "u")
                );
                assert!(matches!(&inlines[2], RichInline::Text(t) if t == " 后缀"));
            }
            other => panic!("expected paragraph, got {:?}", other),
        }
    }
}
