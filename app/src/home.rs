//! Home view: line input → markdown note → notes list.

use std::path::PathBuf;

use gpui::{
    App, Context, Entity, FocusHandle, Focusable, MouseButton, SharedString, Window, div,
    prelude::*, rgb, white,
};

use crate::text_input::LineInput;
use crate::workspace_files::{list_notes, write_note};

/// Bind LineInput key chords on the application keymap.
pub fn bind_line_input_keys(cx: &mut App) {
    use crate::text_input::{Backspace, Delete, End, Home, Left, Right, Submit};
    use gpui::KeyBinding;

    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("LineInput")),
        KeyBinding::new("delete", Delete, Some("LineInput")),
        KeyBinding::new("left", Left, Some("LineInput")),
        KeyBinding::new("right", Right, Some("LineInput")),
        KeyBinding::new("home", Home, Some("LineInput")),
        KeyBinding::new("end", End, Some("LineInput")),
        KeyBinding::new("enter", Submit, Some("LineInput")),
    ]);
}

pub struct TinkerwayApp {
    line_input: Entity<LineInput>,
    notes: Vec<SharedString>,
    status: SharedString,
    workspace: PathBuf,
    focus_handle: FocusHandle,
}

impl TinkerwayApp {
    pub fn new(
        line_input: Entity<LineInput>,
        workspace: PathBuf,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut app = Self {
            line_input,
            notes: Vec::new(),
            status: format!("workspace: {}", workspace.display()).into(),
            workspace,
            focus_handle: cx.focus_handle(),
        };
        app.refresh_notes(cx);
        app
    }

    /// Wire Enter on `line_input` to write a note via this app entity.
    ///
    /// The callback returns success so `LineInput::submit` can clear on `self`
    /// — never nest `line_input.update` from inside LineInput's own update.
    pub fn connect_submit(app: &Entity<Self>, line_input: &Entity<LineInput>, cx: &mut App) {
        let app_entity = app.clone();
        line_input.update(cx, |input, _cx| {
            input.on_submit = Some(Box::new(move |line, _window, cx| {
                app_entity.update(cx, |app, cx| app.submit_line(line, cx))
            }));
        });
    }

    pub fn line_input(&self) -> &Entity<LineInput> {
        &self.line_input
    }

    pub fn notes(&self) -> &[SharedString] {
        &self.notes
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    fn refresh_notes(&mut self, cx: &mut Context<Self>) {
        match list_notes(&self.workspace) {
            Ok(names) => {
                self.notes = names.into_iter().map(SharedString::from).collect();
            }
            Err(err) => {
                self.status = format!("could not list notes: {err}").into();
            }
        }
        cx.notify();
    }

    /// Write `line` as a note and refresh the list. Returns whether the write
    /// succeeded. Does **not** clear the line input — Enter clears inside
    /// `LineInput::submit`; the Add button clears via a parent-owned update.
    pub fn submit_line(&mut self, line: &str, cx: &mut Context<Self>) -> bool {
        match write_note(&self.workspace, line) {
            Ok(name) => {
                self.status = format!("wrote {name}").into();
                self.refresh_notes(cx);
                true
            }
            Err(err) => {
                self.status = format!("write failed: {err}").into();
                cx.notify();
                false
            }
        }
    }

    fn on_submit_click(
        &mut self,
        _: &gpui::MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line = self.line_input.read(cx).text().to_string();
        if self.submit_line(&line, cx) {
            // Parent-first path: not nested inside LineInput's update.
            self.line_input.update(cx, |input, cx| input.clear(cx));
        }
    }
}

impl Focusable for TinkerwayApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TinkerwayApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notes = self.notes.clone();
        let status = self.status.clone();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xf7f7f5))
            .text_color(rgb(0x1a1a1a))
            .font_family(".SystemUIFont")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_6()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("tinkerway"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x555555))
                            .child("Type a line. Enter writes a markdown note in .tinkerway-workspace/."),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .items_center()
                            .child(self.line_input.clone())
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(0x1a1a1a))
                                    .text_color(white())
                                    .text_sm()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(rgb(0x333333)))
                                    .child("Add")
                                    .on_mouse_up(
                                        MouseButton::Left,
                                        cx.listener(Self::on_submit_click),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x666666))
                            .child(status),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child("Notes"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .children(if notes.is_empty() {
                                vec![div()
                                    .text_sm()
                                    .text_color(rgb(0x888888))
                                    .child("No notes yet.")
                                    .into_any_element()]
                            } else {
                                notes
                                    .into_iter()
                                    .map(|name| {
                                        div()
                                            .text_sm()
                                            .py_1()
                                            .border_b_1()
                                            .border_color(rgb(0xe5e5e5))
                                            .child(name)
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_input::Submit;
    use crate::workspace_files::{ensure_workspace, list_notes};
    use gpui::{Focusable, Keystroke, TestAppContext};
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_workspace() -> PathBuf {
        let mut dir = env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("tinkerway-home-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn submit_line_writes_and_lists_without_clearing_input() {
        let dir = temp_workspace();
        ensure_workspace(&dir).unwrap();

        // Contract: submit_line returns success and refreshes notes; clearing
        // is the caller's job (LineInput::submit on Enter, or Add button).
        let mut cx = TestAppContext::single();
        let (app, line_input) = cx.update(|cx| {
            let line_input = cx.new(|cx| LineInput::new(cx, "placeholder"));
            line_input.update(cx, |input, cx| input.set_text("from unit", cx));
            let app = cx.new(|cx| TinkerwayApp::new(line_input.clone(), dir.clone(), cx));
            (app, line_input)
        });

        let wrote = app.update(&mut cx, |app, cx| app.submit_line("from unit", cx));
        assert!(wrote);
        app.read_with(&cx, |app, _| {
            assert_eq!(app.notes().len(), 1);
            assert!(app.status().starts_with("wrote "));
        });
        // Parent did not clear — text still present (simulates nested-safe path).
        line_input.read_with(&cx, |input, _| {
            assert_eq!(input.text(), "from unit");
        });

        let _ = fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn submit_clears_on_self_without_nested_update(cx: &mut TestAppContext) {
        // Drive Submit while LineInput is the updating entity; clear must happen
        // on `self`, not via entity.update (which would panic).
        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |_, cx| {
                    cx.new(|cx| {
                        let mut input = LineInput::new(cx, "placeholder");
                        input.on_submit = Some(Box::new(|_line, _window, _cx| true));
                        input
                    })
                })
            })
            .unwrap();

        window
            .update(cx, |input, window, cx| {
                input.set_text("clear me", cx);
                input.submit(&Submit, window, cx);
            })
            .unwrap();

        window
            .update(cx, |input, _, _| {
                assert!(input.text().is_empty(), "submit should clear on self");
            })
            .unwrap();
    }

    #[gpui::test]
    fn enter_writes_note_and_updates_list(cx: &mut TestAppContext) {
        let dir = temp_workspace();
        ensure_workspace(&dir).unwrap();

        cx.update(bind_line_input_keys);

        let window = cx
            .update(|cx| {
                let workspace = dir.clone();
                cx.open_window(Default::default(), |_, cx| {
                    let line_input = cx.new(|cx| LineInput::new(cx, "What's on your mind?"));
                    let app = cx.new(|cx| TinkerwayApp::new(line_input.clone(), workspace, cx));
                    TinkerwayApp::connect_submit(&app, &line_input, cx);
                    app
                })
            })
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                app.line_input.update(cx, |input, cx| {
                    input.set_text("hello from enter", cx);
                });
                window.focus(&app.line_input.focus_handle(cx));
            })
            .unwrap();

        cx.dispatch_keystroke(*window, Keystroke::parse("enter").unwrap());

        window
            .update(cx, |app, _, cx| {
                assert_eq!(app.notes().len(), 1, "note list should refresh");
                assert!(
                    app.status().starts_with("wrote "),
                    "status={}",
                    app.status()
                );
                assert!(
                    app.line_input.read(cx).text().is_empty(),
                    "Enter should clear via LineInput::submit on self"
                );
            })
            .unwrap();

        let listed = list_notes(&dir).unwrap();
        assert_eq!(listed.len(), 1);
        let body = fs::read_to_string(dir.join(&listed[0])).unwrap();
        assert_eq!(body, "hello from enter\n");

        let _ = fs::remove_dir_all(&dir);
    }
}
