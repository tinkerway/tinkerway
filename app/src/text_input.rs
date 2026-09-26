//! Text field for compose + note body.
//! Adapted from the GPUI 0.2.2 `input` example (Apache-2.0), without
//! `unicode-segmentation` — char boundaries via std only.
//! Supports single-line (compose submit) and multi-line (Enter inserts newline).

use std::ops::Range;

use gpui::{
    App, Bounds, Context, CursorStyle, ElementId, ElementInputHandler, Entity, EntityInputHandler,
    FocusHandle, Focusable, GlobalElementId, LayoutId, MouseButton, MouseDownEvent, PaintQuad,
    Pixels, Point, ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle,
    Window, actions, div, fill, hsla, point, prelude::*, px, relative, rgb, rgba, size, white,
};

actions!(
    text_input,
    [Backspace, Delete, Left, Right, Home, End, Submit, Newline]
);

pub struct TextInput {
    focus_handle: FocusHandle,
    content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    last_lines: Vec<(Range<usize>, ShapedLine)>,
    multiline: bool,
    min_lines: usize,
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
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            last_lines: Vec::new(),
            multiline: false,
            min_lines: 1,
            on_submit: None,
            on_change: None,
        }
    }

    pub fn multiline(mut self, min_lines: usize) -> Self {
        self.multiline = true;
        self.min_lines = min_lines.max(1);
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
        self.marked_range = None;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
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
        if self.multiline {
            let cursor = self.cursor_offset();
            let line_start = self.content[..cursor]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            self.move_to(line_start, cx);
        } else {
            self.move_to(0, cx);
        }
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        if self.multiline {
            let cursor = self.cursor_offset();
            let line_end = self.content[cursor..]
                .find('\n')
                .map(|i| cursor + i)
                .unwrap_or(self.content.len());
            self.move_to(line_end, cx);
        } else {
            self.move_to(self.content.len(), cx);
        }
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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_to(self.index_for_mouse_position(event.position), cx);
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
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
            let n = self.last_lines.len().max(1) as f32;
            let line_h = f32::from(bounds.size.height) / n;
            let rel_y = f32::from((position.y - bounds.top()).max(px(0.)));
            let line_idx = (rel_y / line_h).floor().max(0.0) as usize;
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
        if self.selection_reversed {
            self.selected_range.start = offset;
        } else {
            self.selected_range.end = offset;
        }
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify();
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
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start.min(last_layout.len())),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end.min(last_layout.len())),
                bounds.bottom(),
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let layout = self.last_layout.as_ref()?;
        Some(self.offset_to_utf16(layout.closest_index_for_x(line_point.x)))
    }
}

struct FieldElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    lines: Vec<(Range<usize>, ShapedLine)>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
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
                let at = shaped_lines
                    .last()
                    .map(|(r, _)| r.end)
                    .unwrap_or(0);
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
        let mut selection_quad = None;
        if !is_empty {
            if selected_range.is_empty() {
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
            } else if let Some((_, first_line)) = shaped_lines.first() {
                // Simple single-band selection for first line overlap (good enough for demo).
                let start = selected_range.start.min(first_line.len());
                let end = selected_range.end.min(first_line.len());
                selection_quad = Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + first_line.x_for_index(start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + first_line.x_for_index(end),
                            bounds.top() + line_height,
                        ),
                    ),
                    rgba(0x3366ff40),
                ));
            }
        } else {
            cursor_quad = Some(fill(
                Bounds::new(point(bounds.left(), bounds.top()), size(px(2.), line_height)),
                rgb(0x1a1a1a),
            ));
        }

        PrepaintState {
            lines: shaped_lines,
            cursor: cursor_quad,
            selection: selection_quad,
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
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection);
        }
        let lines = std::mem::take(&mut prepaint.lines);
        for (line_idx, (_range, line)) in lines.iter().enumerate() {
            let origin = point(
                bounds.left(),
                bounds.top() + prepaint.line_height * line_idx as f32,
            );
            line.paint(origin, prepaint.line_height, window, cx).unwrap();
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
        div()
            .flex()
            .key_context(context)
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::newline))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .bg(white())
            .border_1()
            .border_color(rgb(0xcccccc))
            .rounded_md()
            .px_3()
            .py_2()
            .w_full()
            .line_height(px(22.))
            .text_size(px(15.))
            .text_color(rgb(0x1a1a1a))
            .child(FieldElement {
                input: cx.entity(),
            })
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
