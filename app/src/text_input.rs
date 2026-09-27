//! Text field for compose + note body.
//! Adapted from the GPUI 0.2.2 `input` example (Apache-2.0), without
//! `unicode-segmentation` — char boundaries via std only.
//! Supports single-line (compose submit) and multi-line (Enter inserts newline).

use std::ops::Range;

use gpui::{
    actions, div, fill, hsla, point, prelude::*, px, relative, rgb, rgba, size, white, App, Bounds,
    ClipboardItem, Context, CursorStyle, ElementId, ElementInputHandler, Entity,
    EntityInputHandler, FocusHandle, Focusable, GlobalElementId, LayoutId, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine,
    SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window,
};

actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        Home,
        End,
        Submit,
        Newline,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectHome,
        SelectEnd,
        SelectAll,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteWordLeft,
        DeleteToLineStart,
        Copy,
        Cut,
        Paste,
    ]
);

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    /// Horizontal aim for Up and Down, so the caret stays in the same column.
    preferred_x: Option<Pixels>,
    /// True while the pointer button is down and a drag should extend the selection.
    is_selecting: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    last_lines: Vec<(Range<usize>, ShapedLine)>,
    last_line_height: Pixels,
    multiline: bool,
    min_lines: usize,
    /// Drop the field chrome so a parent surface can be the writing area.
    bare: bool,
    text_px: f32,
    line_px: f32,
    /// Returns `true` when the parent accepted the text (e.g. note created).
    /// `TextInput::submit` clears on `self` after success — do not nest
    /// `entity.update` on this input from inside the callback.
    pub on_submit: Option<Box<dyn Fn(&str, &mut Window, &mut Context<Self>) -> bool>>,
    /// Fired after content changes (typing, paste, clear). Parent may debounce save.
    pub on_change: Option<Box<dyn Fn(&str, &mut Window, &mut Context<Self>)>>,
}

impl TextInput {
    pub fn new(cx: &mut Context<Self>, placeholder: impl Into<SharedString>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: placeholder.into(),
            selected_range: 0..0,
            selection_reversed: false,
            preferred_x: None,
            is_selecting: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            last_lines: Vec::new(),
            last_line_height: px(22.),
            multiline: false,
            min_lines: 1,
            bare: false,
            text_px: 15.,
            line_px: 22.,
            on_submit: None,
            on_change: None,
        }
    }

    pub fn multiline(mut self, min_lines: usize) -> Self {
        self.multiline = true;
        self.min_lines = min_lines.max(1);
        self
    }

    /// Render text only. The parent draws the surface.
    pub fn bare(mut self) -> Self {
        self.bare = true;
        self
    }

    /// Set the text size and line height in pixels.
    pub fn text_px(mut self, size: f32, line: f32) -> Self {
        self.text_px = size;
        self.line_px = line;
        self
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.content = text.into();
        let len = self.content.len();
        self.selected_range = len..len;
        self.selection_reversed = false;
        self.preferred_x = None;
        self.marked_range = None;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.preferred_x = None;
        self.marked_range = None;
        cx.notify();
    }

    fn emit_change(&self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(callback) = self.on_change.as_ref() {
            callback(&self.content, window, cx);
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_start(self.cursor_offset()), cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.line_end(self.cursor_offset()), cx);
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, false, cx);
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(1, false, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(-1, true, cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.move_vertical(1, true, cx);
    }

    fn select_home(&mut self, _: &SelectHome, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_start(self.cursor_offset()), cx);
    }

    fn select_end(&mut self, _: &SelectEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.line_end(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx);
    }

    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.previous_word(self.cursor_offset()), cx);
    }

    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.next_word(self.cursor_offset()), cx);
    }

    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_word(self.cursor_offset()), cx);
    }

    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_word(self.cursor_offset()), cx);
    }

    fn delete_word_left(
        &mut self,
        _: &DeleteWordLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_word(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete_to_line_start(
        &mut self,
        _: &DeleteToLineStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            self.select_to(self.line_start(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.content[self.selected_range.clone()].to_string(),
        ));
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.content[self.selected_range.clone()].to_string(),
        ));
        self.replace_text_in_range(None, "", window, cx);
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        self.replace_text_in_range(None, &text, window, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    pub(crate) fn submit(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        let success = self
            .on_submit
            .as_ref()
            .map(|callback| callback(&self.content, window, cx))
            .unwrap_or(false);
        if success {
            // Clear on `self` while already borrowed — never `entity.update(self)`.
            self.clear(cx);
        }
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            self.replace_text_in_range(None, "\n", window, cx);
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus_handle);
        let offset = self.index_for_mouse_position(event.position);
        if event.click_count >= 3 {
            self.is_selecting = false;
            self.select_line_at(offset, cx);
        } else if event.click_count == 2 {
            self.is_selecting = false;
            self.select_word_at(offset, cx);
        } else if event.modifiers.shift {
            self.is_selecting = true;
            self.select_to(offset, cx);
        } else {
            self.is_selecting = true;
            self.move_to(offset, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if !event.dragging() {
            self.is_selecting = false;
            return;
        }
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.preferred_x = None;
        self.selection_reversed = false;
        self.selected_range = offset..offset;
        cx.notify();
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };
        if self.multiline && !self.last_lines.is_empty() {
            if position.y < bounds.top() {
                return 0;
            }
            let line_h = f32::from(self.last_line_height).max(1.);
            let rel_y = f32::from((position.y - bounds.top()).max(px(0.)));
            let line_idx = (rel_y / line_h).floor().max(0.0) as usize;
            let text_lines = self.content.split('\n').count().max(1);
            // A click under the last written line stays on that line.
            if position.y > bounds.bottom() || line_idx >= text_lines {
                return self.content.len();
            }
            let line_idx = line_idx.min(self.last_lines.len() - 1);
            let (range, line) = &self.last_lines[line_idx];
            let local_x = position.x - bounds.left();
            let local = line.closest_index_for_x(local_x);
            return (range.start + local).min(range.end);
        }
        let Some(line) = self.last_layout.as_ref() else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.preferred_x = None;
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        if self.selected_range.start == self.selected_range.end {
            self.selection_reversed = false;
        }
        cx.notify();
    }

    fn move_vertical(&mut self, delta: isize, selecting: bool, cx: &mut Context<Self>) {
        if self.last_lines.is_empty() {
            let target = if delta < 0 { 0 } else { self.content.len() };
            if selecting {
                self.select_to(target, cx);
            } else {
                self.move_to(target, cx);
            }
            return;
        }
        let cursor = self.cursor_offset();
        let line_idx = self
            .last_lines
            .iter()
            .position(|(range, _)| cursor <= range.end)
            .unwrap_or(self.last_lines.len() - 1);
        let (range, line) = &self.last_lines[line_idx];
        let local = cursor.saturating_sub(range.start).min(line.len());
        let x = self.preferred_x.unwrap_or_else(|| line.x_for_index(local));
        let next =
            (line_idx as isize + delta).clamp(0, self.last_lines.len() as isize - 1) as usize;
        if next == line_idx {
            self.preferred_x = Some(x);
            return;
        }
        let (next_range, next_line) = &self.last_lines[next];
        let local_next = next_line.closest_index_for_x(x);
        let offset = (next_range.start + local_next).min(next_range.end);
        if selecting {
            self.select_to(offset, cx);
        } else {
            self.move_to(offset, cx);
        }
        self.preferred_x = Some(x);
    }

    fn line_start(&self, offset: usize) -> usize {
        let offset = offset.min(self.content.len());
        if self.multiline {
            self.content[..offset]
                .rfind('\n')
                .map(|index| index + 1)
                .unwrap_or(0)
        } else {
            0
        }
    }

    fn line_end(&self, offset: usize) -> usize {
        let offset = offset.min(self.content.len());
        if self.multiline {
            self.content[offset..]
                .find('\n')
                .map(|index| offset + index)
                .unwrap_or(self.content.len())
        } else {
            self.content.len()
        }
    }

    fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let len = self.content.len();
        if len == 0 {
            return;
        }
        let mut probe = offset.min(len);
        if probe == len {
            probe = self.previous_boundary(probe);
        }
        let Some(ch) = self.content[probe..].chars().next() else {
            return;
        };
        let kind = word_kind(ch);
        let mut start = probe;
        while start > 0 {
            let prev = self.previous_boundary(start);
            let prev_ch = self.content[prev..start].chars().next().unwrap();
            if word_kind(prev_ch) != kind {
                break;
            }
            start = prev;
        }
        let mut end = self.next_boundary(probe);
        while end < len {
            let next_ch = self.content[end..].chars().next().unwrap();
            if word_kind(next_ch) != kind {
                break;
            }
            end = self.next_boundary(end);
        }
        self.preferred_x = None;
        self.selection_reversed = false;
        self.selected_range = start..end;
        cx.notify();
    }

    fn select_line_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let offset = offset.min(self.content.len());
        let start = self.line_start(offset);
        let mut end = self.line_end(offset);
        if self.content.as_bytes().get(end) == Some(&b'\n') {
            end += 1;
        }
        self.preferred_x = None;
        self.selection_reversed = false;
        self.selected_range = start..end;
        cx.notify();
    }

    fn previous_word(&self, offset: usize) -> usize {
        let mut index = offset.min(self.content.len());
        while index > 0 {
            let prev = self.previous_boundary(index);
            let ch = self.content[prev..index].chars().next().unwrap();
            if is_word_char(ch) {
                break;
            }
            index = prev;
        }
        while index > 0 {
            let prev = self.previous_boundary(index);
            let ch = self.content[prev..index].chars().next().unwrap();
            if !is_word_char(ch) {
                break;
            }
            index = prev;
        }
        index
    }

    fn next_word(&self, offset: usize) -> usize {
        let len = self.content.len();
        let mut index = offset.min(len);
        while index < len {
            let next = self.next_boundary(index);
            let ch = self.content[index..next].chars().next().unwrap();
            if is_word_char(ch) {
                break;
            }
            index = next;
        }
        while index < len {
            let next = self.next_boundary(index);
            let ch = self.content[index..next].chars().next().unwrap();
            if !is_word_char(ch) {
                break;
            }
            index = next;
        }
        index
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf16_count = 0;
        let mut utf8_offset = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .char_indices()
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .char_indices()
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn word_kind(ch: char) -> u8 {
    if is_word_char(ch) {
        0
    } else if ch.is_whitespace() {
        1
    } else {
        2
    }
}

impl EntityInputHandler for TextInput {
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
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        let new_text = if self.multiline {
            new_text.replace('\r', "")
        } else {
            new_text.replace(['\n', '\r'], "")
        };

        self.content =
            (self.content[0..range.start].to_owned() + &new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.selection_reversed = false;
        self.preferred_x = None;
        self.marked_range.take();
        self.emit_change(window, cx);
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
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or_else(|| self.marked_range.clone())
            .unwrap_or_else(|| self.selected_range.clone());

        let new_text = if self.multiline {
            new_text.replace('\r', "")
        } else {
            new_text.replace(['\n', '\r'], "")
        };

        self.content =
            (self.content[0..range.start].to_owned() + &new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.start)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        self.emit_change(window, cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);
        let line_h = self.last_line_height;
        let (line_idx, span, line) =
            self.last_lines
                .iter()
                .enumerate()
                .find_map(|(idx, (span, line))| {
                    (range.start >= span.start && range.start <= span.end)
                        .then_some((idx, span, line))
                })?;
        let local_start = range.start.saturating_sub(span.start).min(line.len());
        let local_end = range
            .end
            .min(span.end)
            .saturating_sub(span.start)
            .min(line.len());
        let x0 = line.x_for_index(local_start);
        let x1 = line.x_for_index(local_end);
        let y = bounds.top() + line_h * line_idx as f32;
        Some(Bounds::from_corners(
            point(bounds.left() + x0, y),
            point(bounds.left() + x1.max(x0 + px(1.)), y + line_h),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.offset_to_utf16(self.index_for_mouse_position(point)))
    }
}

struct FieldElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    lines: Vec<(Range<usize>, ShapedLine)>,
    cursor: Option<PaintQuad>,
    selection: Vec<PaintQuad>,
    line_height: Pixels,
}

impl IntoElement for FieldElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FieldElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let line_height = window.line_height();
        let line_count = if input.multiline {
            let content_lines = input.content.lines().count().max(1);
            content_lines.max(input.min_lines)
        } else {
            1
        };
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (line_height * line_count as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.content.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();
        let line_height = window.line_height();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let is_empty = content.is_empty();

        let display = if is_empty {
            input.placeholder.clone()
        } else {
            content.clone()
        };
        let text_color = if is_empty {
            hsla(0., 0., 0., 0.4)
        } else {
            style.color
        };

        let mut shaped_lines: Vec<(Range<usize>, ShapedLine)> = Vec::new();
        if input.multiline {
            let mut byte_offset = 0usize;
            let segments: Vec<String> = if display.is_empty() {
                vec![String::new()]
            } else {
                display.split('\n').map(str::to_owned).collect()
            };
            for (i, segment) in segments.iter().enumerate() {
                let start = byte_offset;
                let end = start + segment.len();
                let run = TextRun {
                    len: segment.len(),
                    font: style.font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let line = window.text_system().shape_line(
                    SharedString::from(segment.clone()),
                    font_size,
                    &[run],
                    None,
                );
                shaped_lines.push((start..end, line));
                byte_offset = end;
                if i + 1 < segments.len() {
                    // account for the '\n' separator in the source string
                    if !is_empty {
                        byte_offset += 1;
                    }
                }
            }
            while shaped_lines.len() < input.min_lines {
                let run = TextRun {
                    len: 0,
                    font: style.font(),
                    color: text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let line = window.text_system().shape_line(
                    SharedString::from(""),
                    font_size,
                    &[run],
                    None,
                );
                let at = shaped_lines.last().map(|(r, _)| r.end).unwrap_or(0);
                shaped_lines.push((at..at, line));
            }
        } else {
            let run = TextRun {
                len: display.len(),
                font: style.font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let runs = if let Some(marked_range) = input.marked_range.as_ref() {
                vec![
                    TextRun {
                        len: marked_range.start,
                        ..run.clone()
                    },
                    TextRun {
                        len: marked_range.end - marked_range.start,
                        underline: Some(UnderlineStyle {
                            color: Some(run.color),
                            thickness: px(1.0),
                            wavy: false,
                        }),
                        ..run.clone()
                    },
                    TextRun {
                        len: display.len() - marked_range.end,
                        ..run
                    },
                ]
                .into_iter()
                .filter(|run| run.len > 0)
                .collect()
            } else {
                vec![run]
            };
            let line = window
                .text_system()
                .shape_line(display, font_size, &runs, None);
            shaped_lines.push((0..content.len(), line));
        }

        let mut cursor_quad = None;
        let mut selection = Vec::new();
        if !is_empty {
            if !selected_range.is_empty() {
                let field_width = bounds.size.width;
                for (line_idx, (range, line)) in shaped_lines.iter().enumerate() {
                    let has_newline = content.as_bytes().get(range.end) == Some(&b'\n');
                    let covered_end = if has_newline {
                        range.end + 1
                    } else {
                        range.end
                    };
                    if selected_range.end <= range.start || selected_range.start >= covered_end {
                        continue;
                    }
                    let local_start = selected_range
                        .start
                        .saturating_sub(range.start)
                        .min(line.len());
                    let local_end = selected_range
                        .end
                        .min(range.end)
                        .saturating_sub(range.start)
                        .min(line.len());
                    let x0 = line.x_for_index(local_start);
                    let extend = has_newline && selected_range.end > range.end;
                    let x1 = if extend {
                        field_width
                    } else {
                        line.x_for_index(local_end)
                    };
                    let right = if x1 > x0 { x1 } else { x0 + px(8.) };
                    let y = bounds.top() + line_height * line_idx as f32;
                    selection.push(fill(
                        Bounds::from_corners(
                            point(bounds.left() + x0, y),
                            point(bounds.left() + right, y + line_height),
                        ),
                        rgba(0x4a61704d),
                    ));
                }
            }
            for (line_idx, (range, line)) in shaped_lines.iter().enumerate() {
                if cursor >= range.start && cursor <= range.end {
                    let local = cursor - range.start;
                    let x = line.x_for_index(local);
                    cursor_quad = Some(fill(
                        Bounds::new(
                            point(
                                bounds.left() + x,
                                bounds.top() + line_height * line_idx as f32,
                            ),
                            size(px(2.), line_height),
                        ),
                        rgb(0x1a1a1a),
                    ));
                    break;
                }
                // cursor on the newline between lines
                if cursor == range.end + 1
                    && line_idx + 1 < shaped_lines.len()
                    && content.as_bytes().get(range.end) == Some(&b'\n')
                {
                    cursor_quad = Some(fill(
                        Bounds::new(
                            point(
                                bounds.left(),
                                bounds.top() + line_height * (line_idx + 1) as f32,
                            ),
                            size(px(2.), line_height),
                        ),
                        rgb(0x1a1a1a),
                    ));
                    break;
                }
            }
        } else {
            cursor_quad = Some(fill(
                Bounds::new(
                    point(bounds.left(), bounds.top()),
                    size(px(2.), line_height),
                ),
                rgb(0x1a1a1a),
            ));
        }

        PrepaintState {
            lines: shaped_lines,
            cursor: cursor_quad,
            selection,
            line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for quad in prepaint.selection.drain(..) {
            window.paint_quad(quad);
        }
        let lines = std::mem::take(&mut prepaint.lines);
        for (line_idx, (_range, line)) in lines.iter().enumerate() {
            let origin = point(
                bounds.left(),
                bounds.top() + prepaint.line_height * line_idx as f32,
            );
            line.paint(origin, prepaint.line_height, window, cx)
                .unwrap();
        }
        if focus_handle.is_focused(window) {
            if let Some(cursor) = prepaint.cursor.take() {
                window.paint_quad(cursor);
            }
        }
        let primary = lines.first().map(|(_, l)| l.clone());
        self.input.update(cx, |input, _cx| {
            input.last_layout = primary;
            input.last_bounds = Some(bounds);
            input.last_lines = lines;
            input.last_line_height = prepaint.line_height;
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let context = if self.multiline {
            "MultilineInput"
        } else {
            "LineInput"
        };
        let text_px = self.text_px;
        let line_px = self.line_px;
        let bare = self.bare;
        div()
            .flex()
            .key_context(context)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_home))
            .on_action(cx.listener(Self::select_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::newline))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .when(!bare, |this| {
                this.bg(white())
                    .border_1()
                    .border_color(rgb(0xcccccc))
                    .rounded_md()
                    .px_3()
                    .py_2()
            })
            .w_full()
            .when(bare, |this| this.flex_col().min_h_full())
            .line_height(px(line_px))
            .text_size(px(text_px))
            .text_color(rgb(0x142a3a))
            .child(FieldElement { input: cx.entity() })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
