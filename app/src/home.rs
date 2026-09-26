//! Home view: multi-line compose → vault note → list → open/edit → debounced autosave.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    App, Context, Entity, FocusHandle, Focusable, MouseButton, SharedString, Task, Timer, Window,
    div, img, prelude::*, px, rgb, white,
};
use tinkerway_vault::{
    MasterKey, NoteId, NoteMeta, Vault, migrate_legacy_workspace, title_from_body,
};

use crate::text_input::TextInput;

const AUTOSAVE_DELAY_MS: u64 = 400;

/// Bind TextInput key chords on the application keymap.
pub fn bind_text_input_keys(cx: &mut App) {
    use crate::text_input::{Backspace, Delete, End, Home, Left, Newline, Right, Submit};
    use gpui::KeyBinding;

    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("LineInput")),
        KeyBinding::new("delete", Delete, Some("LineInput")),
        KeyBinding::new("left", Left, Some("LineInput")),
        KeyBinding::new("right", Right, Some("LineInput")),
        KeyBinding::new("home", Home, Some("LineInput")),
        KeyBinding::new("end", End, Some("LineInput")),
        KeyBinding::new("enter", Submit, Some("LineInput")),
        // Multiline compose / body
        KeyBinding::new("backspace", Backspace, Some("MultilineInput")),
        KeyBinding::new("delete", Delete, Some("MultilineInput")),
        KeyBinding::new("left", Left, Some("MultilineInput")),
        KeyBinding::new("right", Right, Some("MultilineInput")),
        KeyBinding::new("home", Home, Some("MultilineInput")),
        KeyBinding::new("end", End, Some("MultilineInput")),
        KeyBinding::new("enter", Newline, Some("MultilineInput")),
        KeyBinding::new("cmd-enter", Submit, Some("MultilineInput")),
        KeyBinding::new("ctrl-enter", Submit, Some("MultilineInput")),
    ]);
}

pub struct TinkerwayApp {
    compose_input: Entity<TextInput>,
    body_input: Entity<TextInput>,
    notes: Vec<NoteMeta>,
    selected_id: Option<NoteId>,
    status: SharedString,
    vault: Vault,
    focus_handle: FocusHandle,
    save_generation: u64,
    _save_task: Option<Task<()>>,
    dirty: bool,
}

impl TinkerwayApp {
    pub fn new(
        compose_input: Entity<TextInput>,
        body_input: Entity<TextInput>,
        vault: Vault,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut app = Self {
            compose_input,
            body_input,
            notes: Vec::new(),
            selected_id: None,
            status: format!("vault: {}", vault.root().display()).into(),
            vault,
            focus_handle: cx.focus_handle(),
            save_generation: 0,
            _save_task: None,
            dirty: false,
        };
        app.refresh_notes(cx);
        app
    }

    /// Open vault with an explicit key (tests / Linux CI). Skips Keychain.
    pub fn new_with_test_vault(
        compose_input: Entity<TextInput>,
        body_input: Entity<TextInput>,
        root: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<Self, String> {
        let vault = Vault::open_with_key(root, MasterKey::generate()).map_err(|e| e.to_string())?;
        Ok(Self::new(compose_input, body_input, vault, cx))
    }

    /// Wire compose Submit (cmd/ctrl-enter) to create a note.
    pub fn connect_compose(app: &Entity<Self>, compose: &Entity<TextInput>, cx: &mut App) {
        let app_entity = app.clone();
        compose.update(cx, |input, _cx| {
            input.on_submit = Some(Box::new(move |text, _window, cx| {
                app_entity.update(cx, |app, cx| app.submit_compose(text, cx))
            }));
        });
    }

    /// Wire body edits to debounced autosave. Parent-owned update path.
    pub fn connect_body(app: &Entity<Self>, body: &Entity<TextInput>, cx: &mut App) {
        let app_entity = app.clone();
        body.update(cx, |input, _cx| {
            input.on_change = Some(Box::new(move |_text, _window, cx| {
                app_entity.update(cx, |app, cx| app.on_body_changed(cx));
            }));
        });
    }

    pub fn compose_input(&self) -> &Entity<TextInput> {
        &self.compose_input
    }

    pub fn body_input(&self) -> &Entity<TextInput> {
        &self.body_input
    }

    pub fn notes(&self) -> &[NoteMeta] {
        &self.notes
    }

    pub fn selected_id(&self) -> Option<&NoteId> {
        self.selected_id.as_ref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn vault_root(&self) -> &std::path::Path {
        self.vault.root()
    }

    /// Migrate legacy plaintext `.tinkerway-workspace/` if present.
    pub fn migrate_legacy_if_needed(&mut self, cx: &mut Context<Self>) {
        match migrate_legacy_workspace(&mut self.vault, None) {
            Ok(report) if report.migrated > 0 => {
                self.status = format!(
                    "migrated {} note(s) from plaintext workspace",
                    report.migrated
                )
                .into();
                self.refresh_notes(cx);
            }
            Ok(_) => {}
            Err(err) => {
                self.status = format!("migrate failed: {err}").into();
                cx.notify();
            }
        }
    }

    fn refresh_notes(&mut self, cx: &mut Context<Self>) {
        match self.vault.list_notes() {
            Ok(notes) => {
                self.notes = notes;
                if let Some(selected) = self.selected_id.clone() {
                    if !self.notes.iter().any(|n| n.id == selected) {
                        self.selected_id = None;
                        self.body_input
                            .update(cx, |input, cx| input.set_text("", cx));
                        self.dirty = false;
                    }
                }
            }
            Err(err) => {
                self.status = format!("could not list notes: {err}").into();
            }
        }
        cx.notify();
    }

    /// Load note into the body editor. Parent-owned path — safe from list clicks.
    pub fn open_note(&mut self, id: &NoteId, cx: &mut Context<Self>) {
        // Flush pending edits on the previous note first.
        self.flush_autosave(cx);

        match self.vault.read_note(id) {
            Ok(note) => {
                self.selected_id = Some(note.id.clone());
                self.body_input
                    .update(cx, |input, cx| input.set_text(note.body, cx));
                self.dirty = false;
                self.status = format!("opened {}", note.title).into();
            }
            Err(err) => {
                self.status = format!("could not open note: {err}").into();
            }
        }
        cx.notify();
    }

    /// Create a note from compose text. Returns whether create succeeded.
    /// Does **not** clear compose — Submit clears inside TextInput::submit;
    /// the Add button clears via a parent-owned update.
    pub fn submit_compose(&mut self, text: &str, cx: &mut Context<Self>) -> bool {
        self.flush_autosave(cx);
        match self.vault.create_note(text) {
            Ok(id) => {
                let title = title_from_body(text);
                self.refresh_notes(cx);
                self.open_note(&id, cx);
                self.status = format!("saved “{title}”").into();
                cx.notify();
                true
            }
            Err(err) => {
                self.status = format!("save failed: {err}").into();
                cx.notify();
                false
            }
        }
    }

    fn on_body_changed(&mut self, cx: &mut Context<Self>) {
        if self.selected_id.is_none() {
            return;
        }
        self.dirty = true;
        self.schedule_autosave(cx);
    }

    fn schedule_autosave(&mut self, cx: &mut Context<Self>) {
        self.save_generation = self.save_generation.wrapping_add(1);
        let gen = self.save_generation;
        self._save_task = Some(cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(AUTOSAVE_DELAY_MS)).await;
            this.update(cx, |app, cx| {
                if app.save_generation == gen {
                    app.flush_autosave(cx);
                }
            })
            .ok();
        }));
    }

    fn flush_autosave(&mut self, cx: &mut Context<Self>) {
        if !self.dirty {
            return;
        }
        let Some(id) = self.selected_id.clone() else {
            return;
        };
        let body = self.body_input.read(cx).text().to_string();
        match self.vault.update_note(&id, &body) {
            Ok(()) => {
                self.dirty = false;
                let title = title_from_body(&body);
                self.status = format!("autosaved “{title}”").into();
                self.refresh_notes(cx);
            }
            Err(err) => {
                self.status = format!("autosave failed: {err}").into();
                cx.notify();
            }
        }
    }

    fn on_add_click(
        &mut self,
        _: &gpui::MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = self.compose_input.read(cx).text().to_string();
        if self.submit_compose(&text, cx) {
            // Parent-first path: not nested inside TextInput's update.
            self.compose_input.update(cx, |input, cx| input.clear(cx));
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
        let selected_id = self.selected_id.clone();
        let has_selection = selected_id.is_some();

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
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .child(
                                img("brand/tinkerway-icon.png")
                                    .size(px(36.))
                                    .rounded_md(),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("tinkerway"),
                            ),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x555555))
                            .child(
                                "Write privately. Notes stay encrypted on disk; the OS keystore holds the key.",
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(self.compose_input.clone())
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .items_center()
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
                                            .child("Add note")
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(Self::on_add_click),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x888888))
                                            .child("⌘↩ / Ctrl+Enter also saves"),
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
                                    .map(|meta| {
                                        let is_selected = selected_id
                                            .as_ref()
                                            .is_some_and(|s| s == &meta.id);
                                        let open_id = meta.id.clone();
                                        let title = SharedString::from(meta.title);
                                        div()
                                            .text_sm()
                                            .py_1()
                                            .px_1()
                                            .rounded_sm()
                                            .border_b_1()
                                            .border_color(rgb(0xe5e5e5))
                                            .cursor_pointer()
                                            .when(is_selected, |s| s.bg(rgb(0xe8e8e4)))
                                            .hover(|s| s.bg(rgb(0xeeeeea)))
                                            .child(title)
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.open_note(&open_id, cx);
                                                }),
                                            )
                                            .into_any_element()
                                    })
                                    .collect()
                            }),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(if has_selection {
                                "Note (markdown · autosaves)"
                            } else {
                                "Note"
                            }),
                    )
                    .child(if has_selection {
                        self.body_input.clone().into_any_element()
                    } else {
                        div()
                            .mt_1()
                            .p_3()
                            .rounded_md()
                            .bg(white())
                            .border_1()
                            .border_color(rgb(0xe0e0e0))
                            .min_h(px(120.))
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child("Click a note to edit it.")
                            .into_any_element()
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_input::Submit;
    use gpui::{Focusable, Keystroke, TestAppContext};
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_vault_root() -> PathBuf {
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
    fn submit_compose_writes_and_lists_without_clearing_input() {
        let dir = temp_vault_root();
        let mut cx = TestAppContext::single();
        let (app, compose) = cx.update(|cx| {
            let compose = cx.new(|cx| TextInput::new(cx, "placeholder").multiline(3));
            let body = cx.new(|cx| TextInput::new(cx, "body").multiline(4));
            compose.update(cx, |input, cx| input.set_text("from unit", cx));
            let app = cx
                .new(|cx| {
                    TinkerwayApp::new_with_test_vault(compose.clone(), body, dir.clone(), cx)
                        .unwrap()
                });
            (app, compose)
        });

        let wrote = app.update(&mut cx, |app, cx| app.submit_compose("from unit", cx));
        assert!(wrote);
        app.read_with(&cx, |app, _| {
            assert_eq!(app.notes().len(), 1);
            assert!(app.status().contains("saved"));
        });
        compose.read_with(&cx, |input, _| {
            assert_eq!(input.text(), "from unit");
        });

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn open_note_loads_body_into_editor() {
        let dir = temp_vault_root();
        let mut cx = TestAppContext::single();
        let (app, id) = cx.update(|cx| {
            let compose = cx.new(|cx| TextInput::new(cx, "placeholder").multiline(3));
            let body = cx.new(|cx| TextInput::new(cx, "body").multiline(4));
            let app = cx
                .new(|cx| {
                    TinkerwayApp::new_with_test_vault(compose, body, dir.clone(), cx).unwrap()
                });
            let id = app.update(cx, |app, cx| {
                app.submit_compose("body for click", cx);
                app.selected_id().cloned().unwrap()
            });
            (app, id)
        });

        app.update(&mut cx, |app, cx| {
            // Clear selection state then reopen
            app.selected_id = None;
            app.open_note(&id, cx);
        });
        app.read_with(&cx, |app, cx| {
            assert_eq!(app.selected_id(), Some(&id));
            assert_eq!(app.body_input().read(cx).text(), "body for click");
        });

        let _ = fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn submit_clears_on_self_without_nested_update(cx: &mut TestAppContext) {
        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |_, cx| {
                    cx.new(|cx| {
                        let mut input = TextInput::new(cx, "placeholder");
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
    fn cmd_enter_writes_note_and_updates_list(cx: &mut TestAppContext) {
        let dir = temp_vault_root();
        cx.update(bind_text_input_keys);

        let window = cx
            .update(|cx| {
                let vault_root = dir.clone();
                cx.open_window(Default::default(), |_, cx| {
                    let compose = cx.new(|cx| {
                        TextInput::new(cx, "What's on your mind?").multiline(3)
                    });
                    let body = cx.new(|cx| TextInput::new(cx, "Note body").multiline(6));
                    let app = cx.new(|cx| {
                        TinkerwayApp::new_with_test_vault(compose.clone(), body.clone(), vault_root, cx)
                            .unwrap()
                    });
                    TinkerwayApp::connect_compose(&app, &compose, cx);
                    TinkerwayApp::connect_body(&app, &body, cx);
                    app
                })
            })
            .unwrap();

        window
            .update(cx, |app, window, cx| {
                app.compose_input.update(cx, |input, cx| {
                    input.set_text("hello from enter", cx);
                });
                window.focus(&app.compose_input.focus_handle(cx));
            })
            .unwrap();

        cx.dispatch_keystroke(*window, Keystroke::parse("ctrl-enter").unwrap());

        window
            .update(cx, |app, _, cx| {
                assert_eq!(app.notes().len(), 1, "note list should refresh");
                assert!(
                    app.status().contains("saved"),
                    "status={}",
                    app.status()
                );
                assert!(
                    app.compose_input.read(cx).text().is_empty(),
                    "Submit should clear via TextInput::submit on self"
                );
            })
            .unwrap();

        // Ciphertext on disk
        let notes_dir = dir.join("notes");
        let mut found_tw = false;
        for entry in fs::read_dir(&notes_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("tw") {
                let bytes = fs::read(&path).unwrap();
                assert!(bytes.starts_with(b"TW01"));
                assert!(!String::from_utf8_lossy(&bytes).contains("hello from enter"));
                found_tw = true;
            }
        }
        assert!(found_tw);

        let _ = fs::remove_dir_all(&dir);
    }
}
