//! Textarea state management for multi-line text input
//!
//! Architecture mirrors InputState: Entity-based state with EntityInputHandler,
//! Focusable, EventEmitter, and a custom Element for text rendering.

use crate::capabilities::foundation::theme::use_theme;
use gpui::{prelude::*, *};
use std::ops::Range;

use crate::capabilities::controls::text_util;

pub fn init(cx: &mut App) {
    if !crate::capabilities::foundation::initialization::begin(cx, "textarea-state") {
        return;
    }
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("Textarea")),
        KeyBinding::new("delete", Delete, Some("Textarea")),
        KeyBinding::new("left", Left, Some("Textarea")),
        KeyBinding::new("right", Right, Some("Textarea")),
        KeyBinding::new("up", Up, Some("Textarea")),
        KeyBinding::new("down", Down, Some("Textarea")),
        KeyBinding::new("shift-left", SelectLeft, Some("Textarea")),
        KeyBinding::new("shift-right", SelectRight, Some("Textarea")),
        KeyBinding::new("shift-up", SelectUp, Some("Textarea")),
        KeyBinding::new("shift-down", SelectDown, Some("Textarea")),
        KeyBinding::new("home", Home, Some("Textarea")),
        KeyBinding::new("end", End, Some("Textarea")),
        KeyBinding::new("enter", Enter, Some("Textarea")),
        KeyBinding::new("shift-enter", ShiftEnter, Some("Textarea")),
        KeyBinding::new("tab", Tab, Some("Textarea")),
        KeyBinding::new("shift-tab", ShiftTab, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some("Textarea")),
        KeyBinding::new("escape", Escape, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-left", WordLeft, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-right", WordRight, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", WordLeft, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", WordRight, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-left", SelectWordLeft, Some("Textarea")),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-right", SelectWordRight, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-left", SelectWordLeft, Some("Textarea")),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-right", SelectWordRight, Some("Textarea")),
    ]);
}

actions!(
    textarea_state,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        Copy,
        Cut,
        Paste,
        Enter,
        ShiftEnter,
        Tab,
        ShiftTab,
        Escape,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
    ]
);

/// Events emitted by TextareaState
#[derive(Clone, Debug)]
pub enum TextareaEvent {
    Change,
    Focus,
    Blur,
    Enter,
    ShiftEnter,
    Tab,
    ShiftTab,
}

impl EventEmitter<TextareaEvent> for TextareaState {}

/// Multi-line text input state with cursor management
pub struct TextareaState {
    focus_handle: FocusHandle,
    pub scroll_handle: ScrollHandle,
    pub content: SharedString,
    pub placeholder: SharedString,
    pub disabled: bool,
    pub initialized: bool,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layouts: Vec<WrappedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    last_visual_line_count: usize,
    last_line_height: Pixels,
    is_selecting: bool,
    was_focused: bool,
    cursor_visible: bool,
    _blink_task: Option<Task<()>>,
    event_subscription: Option<Subscription>,
}

#[derive(Clone, Copy)]
struct VisualLineInfo {
    logical_line_idx: usize,
    logical_start: usize,
    local_start: usize,
    local_end: usize,
    origin_y: Pixels,
}

impl TextareaState {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            scroll_handle: ScrollHandle::new(),
            content: "".into(),
            placeholder: "".into(),
            disabled: false,
            initialized: false,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layouts: Vec::new(),
            last_bounds: None,
            last_visual_line_count: 1,
            last_line_height: px(20.0),
            is_selecting: false,
            was_focused: false,
            cursor_visible: true,
            _blink_task: None,
            event_subscription: None,
        }
    }

    pub(super) fn replace_event_subscription(&mut self, subscription: Option<Subscription>) {
        self.event_subscription = subscription;
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn visual_line_count_for_layout(&self) -> usize {
        self.visual_line_count()
    }

    pub fn set_value(
        &mut self,
        value: impl Into<SharedString>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = value.into();
        if self.content != value {
            self.content = value;
            let len = self.content.len();
            self.selected_range = len..len;
            self.marked_range.take();
            self.selection_reversed = false;
            self.invalidate_layout_cache();
            cx.emit(TextareaEvent::Change);
            cx.notify();
        }
    }

    // ── Line helpers ────────────────────────────────────────────────

    /// Returns byte offsets where each line starts (always includes 0).
    fn line_start_offsets(&self) -> Vec<usize> {
        let mut offsets = vec![0usize];
        for (i, ch) in self.content.char_indices() {
            if ch == '\n' {
                offsets.push(i + ch.len_utf8());
            }
        }
        offsets
    }

    fn logical_line_for_offset(&self, offset: usize) -> usize {
        let starts = self.line_start_offsets();
        let mut line = 0;
        for (i, &start) in starts.iter().enumerate() {
            if start <= offset {
                line = i;
            } else {
                break;
            }
        }
        line
    }

    /// Get the byte offset of the start of the line containing `offset`.
    fn line_start_for_offset(&self, offset: usize) -> usize {
        let starts = self.line_start_offsets();
        let line = self.logical_line_for_offset(offset);
        starts[line]
    }

    /// Get the byte offset of the end of the line containing `offset` (before \n).
    fn line_end_for_offset(&self, offset: usize) -> usize {
        let starts = self.line_start_offsets();
        let line = self.logical_line_for_offset(offset);
        if line + 1 < starts.len() {
            starts[line + 1] - 1
        } else {
            self.content.len()
        }
    }

    fn visual_line_count(&self) -> usize {
        self.last_visual_line_count.max(1)
    }

    /// Split content into lines (the text of each line, without trailing \n).
    fn lines(&self) -> Vec<&str> {
        if self.content.is_empty() {
            return vec![""];
        }
        let s: &str = &self.content;
        let mut result: Vec<&str> = s.split('\n').collect();
        // If content ends with \n, split produces an extra empty string which is correct
        if result.is_empty() {
            result.push("");
        }
        result
    }

    fn visual_lines_for_layouts(
        &self,
        layouts: &[WrappedLine],
        line_height: Pixels,
    ) -> Vec<VisualLineInfo> {
        let starts = self.line_start_offsets();
        let mut visual_lines = Vec::new();
        let mut current_y = Pixels::ZERO;

        for (logical_line_idx, wrapped_line) in layouts.iter().enumerate() {
            let logical_start = starts.get(logical_line_idx).copied().unwrap_or(0);
            let mut local_start = 0;
            let mut visual_line_idx = 0;

            for wrap_boundary in wrapped_line.wrap_boundaries() {
                let glyph =
                    &wrapped_line.runs()[wrap_boundary.run_ix].glyphs[wrap_boundary.glyph_ix];
                let local_end = glyph.index;
                visual_lines.push(VisualLineInfo {
                    logical_line_idx,
                    logical_start,
                    local_start,
                    local_end,
                    origin_y: current_y + line_height * visual_line_idx as f32,
                });
                local_start = local_end;
                visual_line_idx += 1;
            }

            visual_lines.push(VisualLineInfo {
                logical_line_idx,
                logical_start,
                local_start,
                local_end: wrapped_line.text.len(),
                origin_y: current_y + line_height * visual_line_idx as f32,
            });

            current_y += line_height * (visual_line_idx + 1) as f32;
        }

        if visual_lines.is_empty() {
            visual_lines.push(VisualLineInfo {
                logical_line_idx: 0,
                logical_start: 0,
                local_start: 0,
                local_end: 0,
                origin_y: Pixels::ZERO,
            });
        }

        visual_lines
    }

    fn visual_lines(&self, line_height: Pixels) -> Vec<VisualLineInfo> {
        self.visual_lines_for_layouts(&self.last_layouts, line_height)
    }

    fn point_for_offset_in_layouts(
        &self,
        layouts: &[WrappedLine],
        offset: usize,
        line_height: Pixels,
    ) -> Option<(usize, Point<Pixels>)> {
        let logical_line_idx = self.logical_line_for_offset(offset);
        let logical_start = *self.line_start_offsets().get(logical_line_idx)?;
        let wrapped_line = layouts.get(logical_line_idx)?;
        let local_offset = offset
            .saturating_sub(logical_start)
            .min(wrapped_line.text.len());
        wrapped_line
            .position_for_index(local_offset, line_height)
            .map(|point| (logical_line_idx, point))
    }

    fn point_for_offset(
        &self,
        offset: usize,
        line_height: Pixels,
    ) -> Option<(usize, Point<Pixels>)> {
        self.point_for_offset_in_layouts(&self.last_layouts, offset, line_height)
    }

    fn vertical_navigation_target(
        &self,
        offset: usize,
        delta: isize,
        line_height: Pixels,
    ) -> Option<usize> {
        if self.last_layouts.is_empty() {
            return None;
        }

        let (logical_line_idx, cursor_point) = self.point_for_offset(offset, line_height)?;
        let visual_lines = self.visual_lines(line_height);
        let logical_line_origin_y = visual_lines
            .iter()
            .find(|line| line.logical_line_idx == logical_line_idx)?
            .origin_y;
        let cursor_visual_y = logical_line_origin_y + cursor_point.y;
        let current_visual_line_idx = visual_lines.iter().position(|line| {
            cursor_visual_y >= line.origin_y && cursor_visual_y < line.origin_y + line_height
        })?;

        let target_visual_line_idx = current_visual_line_idx as isize + delta;
        if target_visual_line_idx < 0 || target_visual_line_idx >= visual_lines.len() as isize {
            return None;
        }

        let target_line = visual_lines[target_visual_line_idx as usize];
        let target_wrapped_line = self.last_layouts.get(target_line.logical_line_idx)?;
        let logical_line_origin_y = visual_lines
            .iter()
            .find(|line| line.logical_line_idx == target_line.logical_line_idx)?
            .origin_y;
        let target_local_y = target_line.origin_y - logical_line_origin_y;
        let target_local_x = cursor_point.x;
        let target_local_offset = target_wrapped_line
            .closest_index_for_position(gpui::point(target_local_x, target_local_y), line_height)
            .unwrap_or_else(|boundary| boundary);
        Some(target_line.logical_start + target_local_offset)
    }

    // ── Cursor / Selection ──────────────────────────────────────────

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify();
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = self.clamp_offset(offset);
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.reset_blink(cx);
        cx.notify();
    }

    fn clamp_offset(&self, offset: usize) -> usize {
        text_util::clamp_offset(&self.content, offset)
    }

    // ── Grapheme boundaries ─────────────────────────────────────────

    fn previous_boundary(&self, offset: usize) -> usize {
        text_util::previous_grapheme_boundary(&self.content, offset)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        text_util::next_grapheme_boundary(&self.content, offset)
    }

    fn previous_word_boundary(&self, offset: usize) -> usize {
        text_util::previous_word_boundary(&self.content, offset)
    }

    fn next_word_boundary(&self, offset: usize) -> usize {
        text_util::next_word_boundary(&self.content, offset)
    }

    // ── UTF-16 conversion (for EntityInputHandler) ──────────────────

    fn offset_to_utf16(&self, offset: usize) -> usize {
        crate::capabilities::controls::text_util::offset_to_utf16(&self.content, offset)
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        crate::capabilities::controls::text_util::range_to_utf16(&self.content, range)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        crate::capabilities::controls::text_util::range_from_utf16(&self.content, range_utf16)
    }

    // ── Layout cache ────────────────────────────────────────────────

    fn invalidate_layout_cache(&mut self) {
        self.last_layouts.clear();
        self.last_bounds = None;
        self.last_visual_line_count = 1;
    }

    // ── Blink ───────────────────────────────────────────────────────

    fn start_blink(&mut self, cx: &mut Context<Self>) {
        self._blink_task = None;
        self.cursor_visible = true;
        self._blink_task = Some(cx.spawn(async |this, cx| loop {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(500))
                .await;
            let ok = this.update(cx, |this: &mut Self, cx: &mut Context<Self>| {
                this.cursor_visible = !this.cursor_visible;
                cx.notify();
            });
            if ok.is_err() {
                break;
            }
        }));
    }

    fn stop_blink(&mut self) {
        self._blink_task = None;
        self.cursor_visible = true;
    }

    fn reset_blink(&mut self, cx: &mut Context<Self>) {
        self.stop_blink();
        self.start_blink(cx);
    }

    fn sync_focus_state(&mut self, is_focused: bool, _window: &mut Window, cx: &mut Context<Self>) {
        if self.was_focused == is_focused {
            return;
        }

        self.was_focused = is_focused;
        if is_focused {
            self.start_blink(cx);
            cx.emit(TextareaEvent::Focus);
        } else {
            self.stop_blink();
            self.is_selecting = false;
            cx.emit(TextareaEvent::Blur);
        }
        cx.notify();
    }

    // ── Mouse ───────────────────────────────────────────────────────

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        let idx = self.index_for_mouse_position(event.position);
        if event.click_count == 2 {
            let start = self.previous_word_boundary(idx);
            let end = self.next_word_boundary(idx);
            self.selected_range = start..end;
            self.selection_reversed = false;
            self.reset_blink(cx);
            cx.notify();
        } else if event.modifiers.shift {
            self.select_to(idx, cx);
        } else {
            self.move_to(idx, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _cx: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.is_selecting {
            let idx = self.index_for_mouse_position(event.position);
            self.select_to(idx, cx);
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if self.last_layouts.is_empty() {
            return 0;
        }

        let relative_y = position.y - bounds.top();
        if relative_y <= Pixels::ZERO {
            return 0;
        }
        if position.y >= bounds.bottom() {
            return self.content.len();
        }

        let line_height = self.last_line_height;
        let visual_lines = self.visual_lines(line_height);
        let relative_x = (position.x - bounds.left()).max(Pixels::ZERO);

        for visual_line in &visual_lines {
            if relative_y >= visual_line.origin_y && relative_y < visual_line.origin_y + line_height
            {
                let Some(wrapped_line) = self.last_layouts.get(visual_line.logical_line_idx) else {
                    return self.content.len();
                };
                let logical_line_origin_y = visual_lines
                    .iter()
                    .find(|line| line.logical_line_idx == visual_line.logical_line_idx)
                    .map(|line| line.origin_y)
                    .unwrap_or(Pixels::ZERO);
                let local_y = relative_y - logical_line_origin_y;
                let local_offset = wrapped_line
                    .closest_index_for_position(gpui::point(relative_x, local_y), line_height)
                    .unwrap_or_else(|boundary| boundary);
                return visual_line.logical_start + local_offset;
            }
        }

        self.content.len()
    }

    // ── Action handlers ─────────────────────────────────────────────

    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    pub fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    pub fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.start, cx);
            return;
        }
        let line_height = self.last_line_height;
        if let Some(new_offset) =
            self.vertical_navigation_target(self.cursor_offset(), -1, line_height)
        {
            self.move_to(new_offset, cx);
        }
    }

    pub fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.end, cx);
            return;
        }
        let line_height = self.last_line_height;
        if let Some(new_offset) =
            self.vertical_navigation_target(self.cursor_offset(), 1, line_height)
        {
            self.move_to(new_offset, cx);
        }
    }

    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    pub fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let line_height = self.last_line_height;
        if let Some(new_offset) =
            self.vertical_navigation_target(self.cursor_offset(), -1, line_height)
        {
            self.select_to(new_offset, cx);
        }
    }

    pub fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let line_height = self.last_line_height;
        if let Some(new_offset) =
            self.vertical_navigation_target(self.cursor_offset(), 1, line_height)
        {
            self.select_to(new_offset, cx);
        }
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.content.len();
        self.selection_reversed = false;
        self.reset_blink(cx);
        cx.notify();
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        let line_start = self.line_start_for_offset(self.cursor_offset());
        self.move_to(line_start, cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        let line_end = self.line_end_for_offset(self.cursor_offset());
        self.move_to(line_end, cx);
    }

    pub fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.previous_word_boundary(self.cursor_offset());
        if self.selected_range.is_empty() {
            self.move_to(offset, cx);
        } else {
            self.move_to(self.selected_range.start.min(offset), cx);
        }
    }

    pub fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.next_word_boundary(self.cursor_offset());
        if self.selected_range.is_empty() {
            self.move_to(offset, cx);
        } else {
            self.move_to(self.selected_range.end.max(offset), cx);
        }
    }

    pub fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_word_boundary(self.cursor_offset()), cx);
    }

    pub fn select_word_right(
        &mut self,
        _: &SelectWordRight,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_to(self.next_word_boundary(self.cursor_offset()), cx);
    }

    pub fn enter(&mut self, _: &Enter, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
        cx.emit(TextareaEvent::Enter);
    }

    pub fn shift_enter(&mut self, _: &ShiftEnter, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextareaEvent::ShiftEnter);
    }

    pub fn tab(&mut self, _: &Tab, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "    ", window, cx);
        cx.emit(TextareaEvent::Tab);
    }

    pub fn shift_tab(&mut self, _: &ShiftTab, _window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextareaEvent::ShiftTab);
    }

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx);
        }
    }

    pub fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    pub fn escape(&mut self, _: &Escape, _window: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = self.content.len()..self.content.len();
        cx.notify();
    }
}

// ── EntityInputHandler ──────────────────────────────────────────────

impl EntityInputHandler for TextareaState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
        self.invalidate_layout_cache();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();
        self.selection_reversed = false;
        if self.focus_handle.is_focused(window) {
            self.reset_blink(cx);
        }
        self.invalidate_layout_cache();
        cx.emit(TextareaEvent::Change);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|r| text_util::range_from_utf16(new_text, r))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        self.selection_reversed = false;
        if self.focus_handle.is_focused(window) {
            self.reset_blink(cx);
        }
        self.invalidate_layout_cache();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.last_layouts.is_empty() {
            return None;
        }

        let range = self.range_from_utf16(&range_utf16);
        let line_height = bounds.size.height / self.visual_line_count() as f32;
        let visual_lines = self.visual_lines(line_height);

        let (start_logical_line_idx, start_point) =
            self.point_for_offset(range.start, line_height)?;
        let (end_logical_line_idx, end_point) = self.point_for_offset(range.end, line_height)?;
        let start_logical_origin_y = visual_lines
            .iter()
            .find(|line| line.logical_line_idx == start_logical_line_idx)?
            .origin_y;
        let end_logical_origin_y = visual_lines
            .iter()
            .find(|line| line.logical_line_idx == end_logical_line_idx)?
            .origin_y;

        Some(Bounds::from_corners(
            point(
                bounds.left() + start_point.x,
                bounds.top() + start_logical_origin_y + start_point.y,
            ),
            point(
                bounds.left() + end_point.x,
                bounds.top() + end_logical_origin_y + end_point.y + line_height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.last_bounds.as_ref()?;
        bounds.localize(&point)?;
        if self.last_layouts.is_empty() {
            return None;
        }

        let byte_offset = self.index_for_mouse_position(point);
        Some(self.offset_to_utf16(byte_offset))
    }
}

// ── Render ───────────────────────────────────────────────────────────

impl Render for TextareaState {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        div()
            .w_full()
            .on_mouse_down(MouseButton::Left, {
                let entity = entity.clone();
                move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |state, cx| {
                        state.on_mouse_down(event, window, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, {
                let entity = entity.clone();
                move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |state, cx| {
                        state.on_mouse_up(event, window, cx);
                    });
                }
            })
            .on_mouse_up_out(MouseButton::Left, {
                let entity = entity.clone();
                move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |state, cx| {
                        state.on_mouse_up(event, window, cx);
                    });
                }
            })
            .on_mouse_move({
                let entity = entity.clone();
                move |event: &MouseMoveEvent, window: &mut Window, cx: &mut App| {
                    entity.update(cx, |state, cx| {
                        state.on_mouse_move(event, window, cx);
                    });
                }
            })
            .child(TextareaTextElement {
                state: entity.clone(),
            })
    }
}

impl Focusable for TextareaState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

// ── Custom Element for multi-line text rendering ─────────────────────

struct TextareaTextElement {
    state: Entity<TextareaState>,
}

struct TextareaPrepaintState {
    lines: Vec<WrappedLine>,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
}

impl IntoElement for TextareaTextElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextareaTextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextareaPrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.state.read(cx);
        let line_count = state.visual_line_count();
        let line_height = window.line_height();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (line_height * line_count as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.state.read(cx);
        let text_style = window.text_style();
        let theme = use_theme(cx);
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();

        let content_lines = input.lines();
        let starts = input.line_start_offsets();
        let selected_range = input.selected_range.clone();
        let cursor_offset = input.cursor_offset();
        let is_empty = input.content.is_empty();
        let placeholder = input.placeholder.clone();
        let marked_range = input.marked_range.clone();
        let wrap_width = Some(bounds.size.width.max(px(1.0)));

        let mut wrapped_lines = Vec::with_capacity(content_lines.len().max(1));
        for (line_idx, line_text) in content_lines.iter().enumerate() {
            let (display_text, text_color) = if is_empty && line_idx == 0 {
                (placeholder.clone(), theme.tokens.muted_foreground)
            } else if is_empty {
                (SharedString::from(""), text_style.color)
            } else {
                (SharedString::from(line_text.to_string()), text_style.color)
            };

            let base_run = TextRun {
                len: display_text.len(),
                font: text_style.font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            // Handle IME marked range within this line
            let runs = if !is_empty {
                if let Some(ref marked) = marked_range {
                    let line_start = starts[line_idx];
                    let line_end = line_start + line_text.len();
                    let mark_start = marked.start.max(line_start).saturating_sub(line_start);
                    let mark_end = marked.end.min(line_end).saturating_sub(line_start);
                    if mark_start < mark_end && mark_start < display_text.len() {
                        vec![
                            TextRun {
                                len: mark_start,
                                ..base_run.clone()
                            },
                            TextRun {
                                len: mark_end - mark_start,
                                underline: Some(UnderlineStyle {
                                    color: Some(base_run.color),
                                    thickness: px(1.0),
                                    wavy: false,
                                }),
                                ..base_run.clone()
                            },
                            TextRun {
                                len: display_text.len() - mark_end,
                                ..base_run.clone()
                            },
                        ]
                        .into_iter()
                        .filter(|r| r.len > 0)
                        .collect()
                    } else {
                        vec![base_run]
                    }
                } else {
                    vec![base_run]
                }
            } else {
                vec![base_run]
            };

            let mut shaped = window
                .text_system()
                .shape_text(display_text, font_size, &runs, wrap_width, None)
                .unwrap_or_default();
            wrapped_lines.push(shaped.pop().unwrap_or_default());
        }

        let visual_lines = input.visual_lines_for_layouts(&wrapped_lines, line_height);
        let mut cursor_quad = None;
        let mut selection_quads = Vec::new();

        if selected_range.is_empty() {
            if let Some((logical_line_idx, cursor_point)) =
                input.point_for_offset_in_layouts(&wrapped_lines, cursor_offset, line_height)
            {
                let logical_line_origin_y = visual_lines
                    .iter()
                    .find(|line| line.logical_line_idx == logical_line_idx)
                    .map(|line| line.origin_y)
                    .unwrap_or(Pixels::ZERO);
                cursor_quad = Some(fill(
                    Bounds::new(
                        point(
                            bounds.left() + cursor_point.x,
                            bounds.top() + logical_line_origin_y + cursor_point.y,
                        ),
                        size(px(2.), line_height),
                    ),
                    rgb(0x0066ff),
                ));
            }
        } else if !is_empty {
            for visual_line in &visual_lines {
                let visual_start = visual_line.logical_start + visual_line.local_start;
                let visual_end = visual_line.logical_start + visual_line.local_end;
                let overlaps_text =
                    selected_range.start < visual_end && selected_range.end > visual_start;
                let spans_newline = selected_range.end > visual_end
                    && visual_line.local_end
                        == wrapped_lines
                            .get(visual_line.logical_line_idx)
                            .map(|line| line.text.len())
                            .unwrap_or(0);

                if !overlaps_text && !spans_newline {
                    continue;
                }

                let Some(wrapped_line) = wrapped_lines.get(visual_line.logical_line_idx) else {
                    continue;
                };
                let local_start = selected_range
                    .start
                    .max(visual_start)
                    .saturating_sub(visual_line.logical_start)
                    .min(wrapped_line.text.len());
                let local_end = selected_range
                    .end
                    .min(visual_end)
                    .saturating_sub(visual_line.logical_start)
                    .min(wrapped_line.text.len());
                let Some(start_point) = wrapped_line.position_for_index(local_start, line_height)
                else {
                    continue;
                };
                let end_point = wrapped_line
                    .position_for_index(local_end, line_height)
                    .unwrap_or(start_point);
                let logical_line_origin_y = visual_lines
                    .iter()
                    .find(|line| line.logical_line_idx == visual_line.logical_line_idx)
                    .map(|line| line.origin_y)
                    .unwrap_or(Pixels::ZERO);
                let x_end = if spans_newline {
                    wrapped_line.width().max(end_point.x + px(4.0))
                } else {
                    end_point.x
                };

                selection_quads.push(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + start_point.x,
                            bounds.top() + logical_line_origin_y + start_point.y,
                        ),
                        point(
                            bounds.left() + x_end,
                            bounds.top() + logical_line_origin_y + end_point.y + line_height,
                        ),
                    ),
                    rgba(0x3311ff30),
                ));
            }
        }

        TextareaPrepaintState {
            lines: wrapped_lines,
            cursor: cursor_quad,
            selections: selection_quads,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let disabled = self.state.read(cx).disabled;
        let line_height = window.line_height();

        // Register input handler — this is THE KEY to receiving keyboard input
        if !disabled {
            window.handle_input(
                &focus_handle,
                ElementInputHandler::new(bounds, self.state.clone()),
                cx,
            );
        }

        // Paint selections
        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }

        // Paint each line
        let lines = std::mem::take(&mut prepaint.lines);
        let mut painted_lines = Vec::with_capacity(lines.len());
        let mut current_y = Pixels::ZERO;
        for line in lines.into_iter() {
            let origin = point(bounds.left(), bounds.top() + current_y);
            let _ = line.paint(
                origin,
                line_height,
                TextAlign::default(),
                Some(bounds),
                window,
                cx,
            );
            current_y += line.size(line_height).height;
            painted_lines.push(line);
        }

        // Paint cursor
        let is_focused = focus_handle.is_focused(window);
        if is_focused {
            let cursor_visible = self.state.read(cx).cursor_visible;
            if cursor_visible {
                if let Some(cursor) = prepaint.cursor.take() {
                    window.paint_quad(cursor);
                }
            }
        }

        // Store layouts for hit-testing
        self.state.update(cx, |state, cx| {
            let visual_line_count = painted_lines
                .iter()
                .map(|line| line.wrap_boundaries().len() + 1)
                .sum::<usize>()
                .max(1);
            state.last_layouts = painted_lines;
            state.last_bounds = Some(bounds);
            state.last_line_height = line_height;
            if state.last_visual_line_count != visual_line_count {
                state.last_visual_line_count = visual_line_count;
                cx.notify();
            }
            state.sync_focus_state(is_focused, window, cx);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::foundation::theme::{install_theme, Theme};

    struct TestView {
        state: Entity<TextareaState>,
    }

    impl Render for TestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    #[gpui::test]
    fn word_left_moves_cursor_to_previous_word_boundary() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx)));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        let text = "foo bar baz";
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value(text, window, cx);
                // set_value places cursor at end (11)
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 8..8, "should stop before baz");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 4..4, "should stop before bar");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 0..0, "should reach start");
            });
        });
    }

    #[gpui::test]
    fn word_right_moves_cursor_to_next_word_boundary() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx)));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value("foo bar baz", window, cx);
                state.selected_range = 0..0;
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 4..4, "should stop at bar start");
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 8..8, "should stop at baz start");
                state.word_right(&WordRight, window, cx);
                assert_eq!(state.selected_range, 11..11, "should reach end");
            });
        });
    }

    #[gpui::test]
    fn word_movement_handles_unicode() {
        let mut app = TestApp::new();
        app.update(|cx| install_theme(cx, Theme::light()));
        let state = app.update(|cx| cx.new(|cx| TextareaState::new(cx)));
        let mut window = app.open_window(|_, _| TestView {
            state: state.clone(),
        });

        let text = "中文 test";
        window.update(|view, window, cx| {
            view.state.update(cx, |state, cx| {
                state.set_value(text, window, cx);
                // set_value places cursor at end (11)
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 7..7, "should stop before test");
                state.word_left(&WordLeft, window, cx);
                assert_eq!(state.selected_range, 0..0, "should reach start");
            });
        });
    }
}
