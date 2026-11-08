use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use crate::capabilities::content::code_block::CodeBlock;
use crate::capabilities::foundation::theme::use_theme;
use crate::capabilities::primitives::separator::Separator;
use crate::capabilities::primitives::text::TextVariant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableAlignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub enum RichInline {
    Text(String),
    Bold(Vec<RichInline>),
    Italic(Vec<RichInline>),
    Strikethrough(Vec<RichInline>),
    Code(String),
    Link {
        text: Vec<RichInline>,
        url: String,
    },
    Image {
        alt: String,
        url: String,
    },
    LineBreak,
    Html(String),
    Styled {
        children: Vec<RichInline>,
        color: Option<Hsla>,
        background_color: Option<Hsla>,
        bold: Option<bool>,
        italic: Option<bool>,
        font_size: Option<f32>,
    },
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub content: Vec<RichInline>,
    pub children: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub enum RichBlock {
    Paragraph(Vec<RichInline>),
    Heading {
        level: u8,
        content: Vec<RichInline>,
    },
    CodeBlock {
        language: Option<String>,
        code: String,
    },
    BlockQuote(Vec<RichBlock>),
    OrderedList {
        start: u64,
        items: Vec<ListItem>,
    },
    UnorderedList {
        items: Vec<ListItem>,
    },
    Table {
        headers: Vec<Vec<RichInline>>,
        alignments: Vec<TableAlignment>,
        rows: Vec<Vec<Vec<RichInline>>>,
    },
    HorizontalRule,
    Image {
        alt: String,
        url: String,
    },
}

#[derive(Clone)]
pub struct LinkInfo {
    pub range: std::ops::Range<usize>,
    pub url: String,
}

struct FlattenResult {
    text: String,
    runs: Vec<TextRun>,
    links: Vec<LinkInfo>,
}

/// A maximal run of inline content rendered at one effective font size.
#[derive(Debug)]
struct FontSizeSegment {
    font_size: f32,
    inlines: Vec<RichInline>,
}

/// Tolerance for comparing effective font sizes (px). Large enough to absorb
/// float arithmetic differences across `em` multiplications, small enough to
/// distinguish any visually distinct size.
const FONT_SIZE_EPSILON_PX: f32 = 0.001;

fn font_sizes_equal(a: f32, b: f32) -> bool {
    (a - b).abs() < FONT_SIZE_EPSILON_PX
}

/// Returns true when any `Styled` node in `inlines` carries an explicit font
/// size. A cheap read-only walk used to decide whether the font-size segment
/// split (which clones the tree) is needed at all.
fn has_explicit_font_size(inlines: &[RichInline]) -> bool {
    inlines.iter().any(|inline| match inline {
        RichInline::Styled {
            font_size,
            children,
            ..
        } => font_size.is_some() || has_explicit_font_size(children),
        RichInline::Bold(children)
        | RichInline::Italic(children)
        | RichInline::Strikethrough(children) => has_explicit_font_size(children),
        RichInline::Link { text, .. } => has_explicit_font_size(text),
        _ => false,
    })
}

/// Split inline content into consecutive segments of uniform effective font
/// size, preserving document order. A `Styled` subtree with an explicit font
/// size starts its own segment; group nodes (Bold, Link, ...) are rebuilt
/// around each same-size run so their styling survives the split. Content
/// without explicit font sizes yields a single segment, letting the caller
/// keep its single-element fast path in the common case.
fn split_by_font_size(inlines: &[RichInline], base_font_size: f32) -> Vec<FontSizeSegment> {
    let mut runs = Vec::new();
    split_font_size_runs(inlines, base_font_size, &mut runs);
    runs.into_iter()
        .map(|(font_size, inlines)| FontSizeSegment { font_size, inlines })
        .collect()
}

fn needs_font_size_segments(segments: &[FontSizeSegment], base_font_size: f32) -> bool {
    segments
        .iter()
        .any(|segment| !font_sizes_equal(segment.font_size, base_font_size))
}

fn segment_element_id(base_id: &ElementId, index: usize) -> ElementId {
    ElementId::NamedChild(Box::new(base_id.clone()), format!("segment-{index}").into())
}

/// Appends items to `runs` as a `(font_size, inlines)` run, merging with the
/// previous run when it targets the same size and is adjacent in document
/// order.
fn push_font_size_run(
    runs: &mut Vec<(f32, Vec<RichInline>)>,
    font_size: f32,
    inlines: Vec<RichInline>,
) {
    if inlines.is_empty() {
        return;
    }
    if let Some(last) = runs.last_mut() {
        if font_sizes_equal(last.0, font_size) {
            last.1.extend(inlines);
            return;
        }
    }
    runs.push((font_size, inlines));
}

/// Walks inline content at `font_size`, appending `(font_size, inlines)` runs
/// to `runs` in document order. Group nodes recurse into their children and
/// wrap every produced run, so both same-size and explicit-size runs keep the
/// group's styling.
fn split_font_size_runs(
    inlines: &[RichInline],
    font_size: f32,
    runs: &mut Vec<(f32, Vec<RichInline>)>,
) {
    for inline in inlines {
        match inline {
            RichInline::Styled {
                font_size: Some(next_size),
                children,
                color,
                background_color,
                bold,
                italic,
            } => {
                let wrapped = vec![RichInline::Styled {
                    children: children.clone(),
                    color: *color,
                    background_color: *background_color,
                    bold: *bold,
                    italic: *italic,
                    font_size: None,
                }];
                split_font_size_runs(&wrapped, *next_size, runs);
            }
            RichInline::Styled {
                font_size: None,
                children,
                color,
                background_color,
                bold,
                italic,
            } => {
                wrap_children_runs(children, font_size, runs, |run| RichInline::Styled {
                    children: run,
                    color: *color,
                    background_color: *background_color,
                    bold: *bold,
                    italic: *italic,
                    font_size: None,
                });
            }
            RichInline::Bold(children) => {
                wrap_children_runs(children, font_size, runs, RichInline::Bold);
            }
            RichInline::Italic(children) => {
                wrap_children_runs(children, font_size, runs, RichInline::Italic);
            }
            RichInline::Strikethrough(children) => {
                wrap_children_runs(children, font_size, runs, RichInline::Strikethrough);
            }
            RichInline::Link { text, url } => {
                wrap_children_runs(text, font_size, runs, |run| RichInline::Link {
                    text: run,
                    url: url.clone(),
                });
            }
            RichInline::Text(_)
            | RichInline::Image { .. }
            | RichInline::Code(_)
            | RichInline::LineBreak
            | RichInline::Html(_) => {
                push_font_size_run(runs, font_size, vec![inline.clone()]);
            }
        }
    }
}

/// Recurses into `children` and wraps each resulting run with `wrap`, so a
/// group node survives font-size splitting around its sized descendants.
fn wrap_children_runs(
    children: &[RichInline],
    font_size: f32,
    runs: &mut Vec<(f32, Vec<RichInline>)>,
    wrap: impl Fn(Vec<RichInline>) -> RichInline,
) {
    let mut child_runs = Vec::new();
    split_font_size_runs(children, font_size, &mut child_runs);
    for (run_size, run) in child_runs {
        push_font_size_run(runs, run_size, vec![wrap(run)]);
    }
}

/// Theme-derived text styling shared by every inline rendering path. Bundled
/// into one struct so the single-element fast path and the font-size segment
/// path stay in lockstep without threading five parameters through each call.
#[derive(Clone)]
struct InlineSegmentStyle {
    font_family: SharedString,
    font_mono: SharedString,
    text_color: Hsla,
    link_color: Hsla,
    code_bg: Hsla,
}

struct InlineFlattener {
    text: String,
    runs: Vec<TextRun>,
    links: Vec<LinkInfo>,
    style: InlineSegmentStyle,
    bold: bool,
    italic: bool,
    strikethrough: bool,
    in_link: Option<String>,
    color_override: Option<Hsla>,
    bg_override: Option<Hsla>,
}

impl InlineFlattener {
    fn new(style: InlineSegmentStyle) -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
            links: Vec::new(),
            style,
            bold: false,
            italic: false,
            strikethrough: false,
            in_link: None,
            color_override: None,
            bg_override: None,
        }
    }

    fn flatten(mut self, inlines: &[RichInline]) -> FlattenResult {
        self.walk(inlines);
        FlattenResult {
            text: self.text,
            runs: self.runs,
            links: self.links,
        }
    }

    fn walk(&mut self, inlines: &[RichInline]) {
        for inline in inlines {
            match inline {
                RichInline::Text(s) => self.push_text(s, false),
                RichInline::Bold(children) => {
                    let prev = self.bold;
                    self.bold = true;
                    self.walk(children);
                    self.bold = prev;
                }
                RichInline::Italic(children) => {
                    let prev = self.italic;
                    self.italic = true;
                    self.walk(children);
                    self.italic = prev;
                }
                RichInline::Strikethrough(children) => {
                    let prev = self.strikethrough;
                    self.strikethrough = true;
                    self.walk(children);
                    self.strikethrough = prev;
                }
                RichInline::Code(code) => self.push_text(code, true),
                RichInline::Link { text, url } => {
                    let prev = self.in_link.take();
                    self.in_link = Some(url.clone());
                    self.walk(text);
                    self.in_link = prev;
                }
                RichInline::Image { alt, .. } => {
                    if !alt.is_empty() {
                        self.push_text(alt, false);
                    }
                }
                RichInline::LineBreak => self.push_text("\n", false),
                RichInline::Html(_) => {}
                RichInline::Styled {
                    children,
                    color,
                    background_color,
                    bold,
                    italic,
                    font_size: _,
                } => {
                    let prev_bold = self.bold;
                    let prev_italic = self.italic;
                    let prev_color = self.color_override;
                    let prev_bg = self.bg_override;
                    if let Some(true) = bold {
                        self.bold = true;
                    }
                    if let Some(true) = italic {
                        self.italic = true;
                    }
                    if color.is_some() {
                        self.color_override = *color;
                    }
                    if background_color.is_some() {
                        self.bg_override = *background_color;
                    }
                    self.walk(children);
                    self.bold = prev_bold;
                    self.italic = prev_italic;
                    self.color_override = prev_color;
                    self.bg_override = prev_bg;
                }
            }
        }
    }

    fn push_text(&mut self, text: &str, is_code: bool) {
        if text.is_empty() {
            return;
        }

        let start = self.text.len();
        self.text.push_str(text);
        let len = text.len();

        let is_link = self.in_link.is_some();

        let font = if is_code {
            Font {
                family: self.style.font_mono.clone(),
                features: FontFeatures::default(),
                weight: FontWeight::default(),
                style: FontStyle::default(),
                fallbacks: None,
            }
        } else {
            Font {
                family: self.style.font_family.clone(),
                features: FontFeatures::default(),
                weight: if self.bold {
                    FontWeight::BOLD
                } else {
                    FontWeight::default()
                },
                style: if self.italic {
                    FontStyle::Italic
                } else {
                    FontStyle::Normal
                },
                fallbacks: None,
            }
        };

        let color = if is_link {
            self.style.link_color
        } else if let Some(c) = self.color_override {
            c
        } else {
            self.style.text_color
        };

        let underline = if is_link {
            Some(UnderlineStyle {
                thickness: px(1.0),
                color: Some(self.style.link_color),
                wavy: false,
            })
        } else {
            None
        };

        let strikethrough = if self.strikethrough {
            Some(StrikethroughStyle {
                thickness: px(1.0),
                color: Some(self.style.text_color),
            })
        } else {
            None
        };

        let background_color = if is_code {
            Some(self.style.code_bg)
        } else {
            self.bg_override
        };

        self.runs.push(TextRun {
            len,
            font,
            color,
            background_color,
            underline,
            strikethrough,
        });

        if let Some(ref url) = self.in_link {
            self.links.push(LinkInfo {
                range: start..(start + len),
                url: url.clone(),
            });
        }
    }
}

pub type LinkClickHandler = Rc<dyn Fn(&str, &mut Window, &mut App) + 'static>;

fn dispatch_link_click(
    handler: &Option<LinkClickHandler>,
    url: &str,
    window: &mut Window,
    cx: &mut App,
) {
    if !is_allowed_link_scheme(url) {
        return;
    }
    match handler.as_ref() {
        Some(handler) => handler(url, window, cx),
        None => cx.open_url(url),
    }
}

fn is_allowed_link_scheme(url: &str) -> bool {
    let url = url.trim_start();
    ["http://", "https://", "mailto:"].iter().any(|scheme| {
        url.get(..scheme.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(scheme))
    })
}

pub fn render_inlines(
    inlines: &[RichInline],
    base_size: Pixels,
    link_handler: Option<&LinkClickHandler>,
    element_id: Option<ElementId>,
    cx: &App,
) -> AnyElement {
    render_inline_content(
        inlines,
        base_size,
        link_handler.cloned(),
        element_id,
        true,
        cx,
    )
}

pub fn render_inlines_with_handler(
    inlines: &[RichInline],
    base_size: Pixels,
    _link_urls: &[LinkInfo],
    on_link_click: &Option<LinkClickHandler>,
    element_id: Option<ElementId>,
    cx: &App,
) -> AnyElement {
    render_inline_content(
        inlines,
        base_size,
        on_link_click.clone(),
        element_id,
        false,
        cx,
    )
}

fn render_inline_content(
    inlines: &[RichInline],
    base_size: Pixels,
    link_handler: Option<LinkClickHandler>,
    element_id: Option<ElementId>,
    debug_links: bool,
    cx: &App,
) -> AnyElement {
    let theme = use_theme(cx);
    let style = InlineSegmentStyle {
        font_family: theme.tokens.font_family.clone(),
        font_mono: theme.tokens.font_mono.clone(),
        text_color: theme.tokens.foreground,
        link_color: theme.tokens.primary,
        code_bg: theme.tokens.muted.opacity(0.3),
    };

    let base_f32 = f32::from(base_size);
    // Content without any explicit font size keeps the single-element fast
    // path; only sized content pays for the segment split.
    if !has_explicit_font_size(inlines) {
        return render_inline_segment(
            inlines,
            base_size,
            style,
            link_handler,
            element_id.unwrap_or_else(|| ElementId::Name("rich-text-inline".into())),
            debug_links,
        );
    }

    let segments = split_by_font_size(inlines, base_f32);
    if !needs_font_size_segments(&segments, base_f32) {
        // Every explicit size resolved to the base size (e.g. `font-size: 1em`),
        // so a single element renders identically and keeps link semantics.
        return render_inline_segment(
            inlines,
            base_size,
            style,
            link_handler,
            element_id.unwrap_or_else(|| ElementId::Name("rich-text-inline".into())),
            debug_links,
        );
    }

    render_font_size_segments(&segments, style, link_handler, element_id, debug_links)
}

/// Renders one run of inline content at a single font size, including the
/// empty-content, link-interactivity and debug-selector handling shared by
/// every inline rendering path.
fn render_inline_segment(
    inlines: &[RichInline],
    size: Pixels,
    style: InlineSegmentStyle,
    link_handler: Option<LinkClickHandler>,
    element_id: ElementId,
    debug_links: bool,
) -> AnyElement {
    let flattened = InlineFlattener::new(style).flatten(inlines);

    if flattened.text.is_empty() {
        return div().into_any_element();
    }

    let styled = StyledText::new(SharedString::from(flattened.text)).with_runs(flattened.runs);

    let mut container = div().text_size(size).line_height(relative(1.5));

    if debug_links && !flattened.links.is_empty() {
        container = container.debug_selector(|| "rich-text-link".into());
    }

    if !flattened.links.is_empty() {
        let click_ranges: Vec<std::ops::Range<usize>> =
            flattened.links.iter().map(|l| l.range.clone()).collect();
        let link_urls: Vec<String> = flattened.links.iter().map(|l| l.url.clone()).collect();

        return container
            .child(InteractiveText::new(element_id, styled).on_click(
                click_ranges,
                move |idx, window, cx| {
                    if let Some(url) = link_urls.get(idx) {
                        dispatch_link_click(&link_handler, url, window, cx);
                    }
                },
            ))
            .into_any_element();
    }

    container.child(styled).into_any_element()
}

/// Renders inline content containing more than one effective font size by
/// giving every segment its own text container. Baseline alignment keeps the
/// segments flowing on one line like a mixed-size text run.
fn render_font_size_segments(
    segments: &[FontSizeSegment],
    style: InlineSegmentStyle,
    link_handler: Option<LinkClickHandler>,
    element_id: Option<ElementId>,
    debug_links: bool,
) -> AnyElement {
    let base_id = element_id.unwrap_or_else(|| ElementId::Name("rich-text-inline".into()));

    let mut container = div()
        .flex()
        .flex_row()
        .flex_wrap()
        .items_baseline()
        .line_height(relative(1.5));

    for (index, segment) in segments.iter().enumerate() {
        let segment_size = px(segment.font_size);
        let segment_id = segment_element_id(&base_id, index);
        container = container.child(render_inline_segment(
            &segment.inlines,
            segment_size,
            style.clone(),
            link_handler.clone(),
            segment_id,
            debug_links,
        ));
    }

    container.into_any_element()
}

pub fn render_blocks(
    blocks: &[RichBlock],
    base_size: Pixels,
    on_link_click: &Option<LinkClickHandler>,
    id_prefix: &str,
    cx: &App,
) -> Vec<AnyElement> {
    let mut elements = Vec::new();
    let mut block_idx = 0u32;

    for block in blocks {
        let el = render_block(
            block,
            base_size,
            on_link_click,
            id_prefix,
            &mut block_idx,
            cx,
        );
        elements.push(el);
    }

    elements
}

fn render_block(
    block: &RichBlock,
    base_size: Pixels,
    on_link_click: &Option<LinkClickHandler>,
    id_prefix: &str,
    block_idx: &mut u32,
    cx: &App,
) -> AnyElement {
    let theme = use_theme(cx);
    *block_idx += 1;
    let idx = *block_idx;

    match block {
        RichBlock::Paragraph(inlines) => {
            let id = ElementId::Name(format!("{}-p-{}", id_prefix, idx).into());
            let el = render_inline_element(inlines, base_size, on_link_click, Some(id), cx);
            div().mb(px(12.0)).child(el).into_any_element()
        }

        RichBlock::Heading { level, content } => {
            let variant = match level {
                1 => TextVariant::H1,
                2 => TextVariant::H2,
                3 => TextVariant::H3,
                4 => TextVariant::H4,
                5 => TextVariant::H5,
                _ => TextVariant::H6,
            };
            let size = variant.size();
            let weight = variant.weight();
            let id = ElementId::Name(format!("{}-h{}-{}", id_prefix, level, idx).into());

            let top_margin = match level {
                1 => px(24.0),
                2 => px(20.0),
                3 => px(16.0),
                _ => px(12.0),
            };

            let el = render_inline_element(content, size, on_link_click, Some(id), cx);
            div()
                .mt(top_margin)
                .mb(px(8.0))
                .font_weight(weight)
                .child(el)
                .into_any_element()
        }

        RichBlock::CodeBlock { language, code } => {
            let mut cb = CodeBlock::new(code.clone())
                .show_line_numbers(true)
                .show_copy_button(true);
            if let Some(lang) = language {
                cb = cb.language(lang.clone());
            }
            div().mb(px(12.0)).child(cb).into_any_element()
        }

        RichBlock::BlockQuote(inner_blocks) => {
            let children = render_blocks(inner_blocks, base_size, on_link_click, id_prefix, cx);
            div()
                .mb(px(12.0))
                .pl(px(16.0))
                .border_l(px(4.0))
                .border_color(theme.tokens.border)
                .bg(theme.tokens.muted.opacity(0.15))
                .py(px(4.0))
                .text_color(theme.tokens.muted_foreground)
                .children(children)
                .into_any_element()
        }

        RichBlock::UnorderedList { items } => {
            let mut render_context = RichBlockRenderContext {
                base_size,
                on_link_click,
                id_prefix,
                block_idx,
                cx,
            };
            let children = render_list_items(items, &mut render_context, false, 1);
            div().mb(px(12.0)).children(children).into_any_element()
        }

        RichBlock::OrderedList { start, items } => {
            let mut render_context = RichBlockRenderContext {
                base_size,
                on_link_click,
                id_prefix,
                block_idx,
                cx,
            };
            let children = render_list_items(items, &mut render_context, true, *start);
            div().mb(px(12.0)).children(children).into_any_element()
        }

        RichBlock::Table {
            headers,
            alignments,
            rows,
        } => {
            let mut render_context = RichBlockRenderContext {
                base_size,
                on_link_click,
                id_prefix,
                block_idx,
                cx,
            };
            render_table(headers, alignments, rows, &mut render_context)
        }

        RichBlock::HorizontalRule => div()
            .my(px(16.0))
            .child(Separator::new())
            .into_any_element(),

        RichBlock::Image { alt: _, url } => div()
            .mb(px(12.0))
            .child(img(SharedString::from(url.clone())).max_w(px(600.0)))
            .into_any_element(),
    }
}

fn render_inline_element(
    inlines: &[RichInline],
    base_size: Pixels,
    on_link_click: &Option<LinkClickHandler>,
    element_id: Option<ElementId>,
    cx: &App,
) -> AnyElement {
    render_inline_content(
        inlines,
        base_size,
        on_link_click.clone(),
        element_id,
        true,
        cx,
    )
}

struct RichBlockRenderContext<'a> {
    base_size: Pixels,
    on_link_click: &'a Option<LinkClickHandler>,
    id_prefix: &'a str,
    block_idx: &'a mut u32,
    cx: &'a App,
}

fn render_list_items(
    items: &[ListItem],
    context: &mut RichBlockRenderContext<'_>,
    ordered: bool,
    start: u64,
) -> Vec<AnyElement> {
    let theme = use_theme(context.cx);
    let mut elements = Vec::new();

    for (i, item) in items.iter().enumerate() {
        *context.block_idx += 1;
        let idx = *context.block_idx;

        let bullet = if let Some(checked) = item.checked {
            if checked {
                SharedString::from("[x] ")
            } else {
                SharedString::from("[ ] ")
            }
        } else if ordered {
            SharedString::from(format!("{}. ", start + i as u64))
        } else {
            SharedString::from("\u{2022} ")
        };

        let id = ElementId::Name(format!("{}-li-{}", context.id_prefix, idx).into());
        let content_el = render_inline_element(
            &item.content,
            context.base_size,
            context.on_link_click,
            Some(id),
            context.cx,
        );

        let row = div()
            .flex()
            .flex_row()
            .pl(px(20.0))
            .mb(px(4.0))
            .child(
                div()
                    .flex_shrink_0()
                    .w(px(24.0))
                    .text_size(context.base_size)
                    .text_color(theme.tokens.muted_foreground)
                    .child(bullet),
            )
            .child(div().flex_1().child(content_el));

        elements.push(row.into_any_element());

        if !item.children.is_empty() {
            let children = render_list_items(&item.children, context, false, 1);
            elements.push(div().pl(px(20.0)).children(children).into_any_element());
        }
    }

    elements
}

fn render_table(
    headers: &[Vec<RichInline>],
    alignments: &[TableAlignment],
    rows: &[Vec<Vec<RichInline>>],
    context: &mut RichBlockRenderContext<'_>,
) -> AnyElement {
    let theme = use_theme(context.cx);
    let col_count = headers.len();
    if col_count == 0 {
        return div().into_any_element();
    }

    let mut table = div()
        .mb(px(12.0))
        .w_full()
        .rounded(theme.tokens.radius_sm)
        .border_1()
        .border_color(theme.tokens.border)
        .overflow_hidden();

    *context.block_idx += 1;
    let header_row = {
        let mut row = div()
            .flex()
            .flex_row()
            .bg(theme.tokens.muted.opacity(0.3))
            .border_b_1()
            .border_color(theme.tokens.border);

        for (ci, header) in headers.iter().enumerate() {
            *context.block_idx += 1;
            let id =
                ElementId::Name(format!("{}-th-{}", context.id_prefix, *context.block_idx).into());
            let el = render_inline_element(
                header,
                context.base_size,
                context.on_link_click,
                Some(id),
                context.cx,
            );
            let mut cell = div()
                .flex_1()
                .px(px(12.0))
                .py(px(8.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_size(context.base_size)
                .child(el);

            if ci < alignments.len() {
                cell = match &alignments[ci] {
                    TableAlignment::Center => cell.items_center(),
                    TableAlignment::Right => cell.items_end(),
                    TableAlignment::Left => cell,
                };
            }

            row = row.child(cell);
        }
        row
    };
    table = table.child(header_row);

    for (ri, row_data) in rows.iter().enumerate() {
        *context.block_idx += 1;
        let mut row = div()
            .flex()
            .flex_row()
            .when(ri < rows.len() - 1, |this| {
                this.border_b_1().border_color(theme.tokens.border)
            })
            .when(ri % 2 == 1, |this| this.bg(theme.tokens.muted.opacity(0.1)));

        for (ci, cell_data) in row_data.iter().enumerate() {
            *context.block_idx += 1;
            let id =
                ElementId::Name(format!("{}-td-{}", context.id_prefix, *context.block_idx).into());
            let el = render_inline_element(
                cell_data,
                context.base_size,
                context.on_link_click,
                Some(id),
                context.cx,
            );
            let mut cell = div()
                .flex_1()
                .px(px(12.0))
                .py(px(6.0))
                .text_size(context.base_size)
                .child(el);

            if ci < alignments.len() {
                cell = match &alignments[ci] {
                    TableAlignment::Center => cell.items_center(),
                    TableAlignment::Right => cell.items_end(),
                    TableAlignment::Left => cell,
                };
            }

            row = row.child(cell);
        }
        table = table.child(row);
    }

    table.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        dispatch_link_click, needs_font_size_segments, segment_element_id, LinkClickHandler,
    };
    use gpui::{ElementId, Empty, TestApp, TestAppContext};
    use std::{cell::Cell, rc::Rc};

    use super::{has_explicit_font_size, split_by_font_size};
    use crate::capabilities::content::rich_text::RichInline;

    fn text(value: &str) -> RichInline {
        RichInline::Text(value.to_string())
    }

    fn styled(children: Vec<RichInline>, font_size: Option<f32>) -> RichInline {
        RichInline::Styled {
            children,
            color: None,
            background_color: None,
            bold: None,
            italic: None,
            font_size,
        }
    }

    #[test]
    fn content_without_explicit_font_sizes_stays_in_one_segment() {
        let inlines = vec![
            text("plain "),
            RichInline::Bold(vec![text("bold")]),
            text(" plain"),
        ];
        let segments = split_by_font_size(&inlines, 14.0);
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].font_size, 14.0);
    }

    #[test]
    fn explicit_font_sizes_split_document_order_segments() {
        let inlines = vec![
            text("before "),
            styled(vec![text("big")], Some(32.0)),
            text(" after"),
        ];
        let segments = split_by_font_size(&inlines, 14.0);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].font_size, 14.0);
        assert_eq!(segments[1].font_size, 32.0);
        assert_eq!(segments[2].font_size, 14.0);
        assert!(
            matches!(&segments[1].inlines[0], RichInline::Styled { font_size: None, children, .. } if matches!(&children[0], RichInline::Text(t) if t == "big"))
        );
    }

    #[test]
    fn a_single_explicit_font_size_requires_segment_rendering() {
        let segments = split_by_font_size(&[styled(vec![text("big")], Some(28.0))], 14.0);
        assert_eq!(segments.len(), 1);
        assert!(needs_font_size_segments(&segments, 14.0));
    }

    #[test]
    fn sized_subtree_inside_a_group_preserves_the_group_in_residue() {
        let inlines = vec![RichInline::Bold(vec![
            text("a "),
            styled(vec![text("b")], Some(24.0)),
            text(" c"),
        ])];
        let segments = split_by_font_size(&inlines, 14.0);
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].font_size, 14.0);
        assert!(matches!(
            &segments[0].inlines[0],
            RichInline::Bold(children) if matches!(&children[0], RichInline::Text(t) if t == "a ")
        ));
        assert_eq!(segments[1].font_size, 24.0);
        assert_eq!(segments[2].font_size, 14.0);
    }

    #[test]
    fn only_explicit_sizes_trigger_the_segment_path() {
        let plain = vec![
            text("plain "),
            RichInline::Bold(vec![styled(vec![text("colored")], None)]),
        ];
        assert!(!has_explicit_font_size(&plain));

        let sized_deep = vec![RichInline::Bold(vec![styled(
            vec![styled(vec![text("deep")], Some(20.0))],
            None,
        )])];
        assert!(has_explicit_font_size(&sized_deep));

        let sized_in_link = vec![RichInline::Link {
            text: vec![styled(vec![text("link")], Some(20.0))],
            url: "https://example.com".to_string(),
        }];
        assert!(has_explicit_font_size(&sized_in_link));
    }

    #[test]
    fn sized_segments_preserve_arbitrary_element_ids() {
        let base_id = ElementId::Integer(42);
        let segment_id = segment_element_id(&base_id, 3);
        assert!(matches!(
            segment_id,
            ElementId::NamedChild(parent, label)
                if *parent == base_id && label == "segment-3"
        ));
    }

    #[gpui::test]
    fn render_inlines_without_an_id_uses_the_shared_fallback(cx: &mut TestAppContext) {
        use crate::capabilities::content::rich_text::render_inlines;
        use gpui::{px, Context, IntoElement, Render};

        struct FallbackIdView;

        impl Render for FallbackIdView {
            fn render(
                &mut self,
                _w: &mut gpui::Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let inlines = vec![RichInline::Link {
                    text: vec![text("link")],
                    url: "https://example.com".to_string(),
                }];
                // `None` element id on the public entry point: the link must
                // still render under the shared "rich-text-inline" fallback so
                // debug selectors stay stable regardless of the caller.
                render_inlines(&inlines, px(14.0), None, None, _cx)
            }
        }

        let (_view, cx) = cx.add_window_view(|_, _| FallbackIdView);
        cx.update(|window, cx| window.draw(cx).clear());
        assert!(
            cx.debug_bounds("rich-text-link").is_some(),
            "link should render with the shared debug selector"
        );
    }

    #[gpui::test]
    fn custom_link_handler_is_invoked() {
        let calls = Rc::new(Cell::new(0));
        let calls_for_handler = calls.clone();
        let handler: Option<LinkClickHandler> = Some(Rc::new(move |url, _, _| {
            assert_eq!(url, "https://example.com");
            calls_for_handler.set(calls_for_handler.get() + 1);
        }));
        let mut app = TestApp::new();
        let mut window = app.open_window(|_, _| Empty);

        window.update(|_, window, cx| {
            dispatch_link_click(&handler, "https://example.com", window, cx);
        });

        assert_eq!(calls.get(), 1);
    }

    #[gpui::test]
    fn disallowed_link_schemes_are_ignored() {
        let calls = Rc::new(Cell::new(0));
        let calls_for_handler = calls.clone();
        let handler: Option<LinkClickHandler> = Some(Rc::new(move |_, _, _| {
            calls_for_handler.set(calls_for_handler.get() + 1);
        }));
        let none_handler: Option<LinkClickHandler> = None;
        let mut app = TestApp::new();
        let mut window = app.open_window(|_, _| Empty);

        window.update(|_, window, cx| {
            dispatch_link_click(&handler, "javascript:alert(1)", window, cx);
            dispatch_link_click(&handler, "ftp://example.com/file", window, cx);
            dispatch_link_click(&none_handler, "file:///etc/passwd", window, cx);
            dispatch_link_click(&handler, "mailto:user@example.com", window, cx);
        });

        assert_eq!(calls.get(), 1);
    }
}
