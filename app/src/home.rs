//! Home view: write a thought, keep it in the list, open an older one to read it.

use std::path::PathBuf;
use std::time::Duration;

use gpui::{
    actions, div, img, prelude::*, px, rgb, App, Context, Entity, FocusHandle, Focusable,
    ModifiersChangedEvent, MouseButton, ScrollHandle, SharedString, Task, Timer, Window,
};
use gpui_component::input::{Escape as FieldEscape, Input, InputEvent, InputState};
use gpui_component::scroll::{Scrollbar, ScrollbarShow};
use gpui_component::{Colorize, Theme};

actions!(capture, [NewCapture, OpenShortcuts, CloseShortcuts]);
use tinkerway_vault::{
    migrate_legacy_workspace, title_from_body, MasterKey, NoteId, NoteMeta, Vault,
};

use crate::text_input::TextInput;

const AUTOSAVE_DELAY_MS: u64 = 400;

/// Quiet edge, a little darker than the page. The active edge is the library focus border.
fn apply_field_theme(cx: &mut App) {
    let theme = Theme::global_mut(cx);
    theme.radius = px(20.);
    theme.shadow = false;
    theme.background = gpui::Hsla::parse_hex("#F7FBFC").expect("mist");
    theme.foreground = gpui::Hsla::parse_hex("#142A3A").expect("ink");
    theme.muted_foreground = gpui::Hsla::parse_hex("#7E96A4").expect("time");
    theme.caret = gpui::Hsla::parse_hex("#142A3A").expect("ink");
    theme.selection = gpui::Hsla::parse_hex("#4A6170")
        .expect("slate")
        .opacity(0.30);
    theme.input = gpui::Hsla::parse_hex("#C9D7DF").expect("horizon");
    theme.ring = gpui::Hsla::parse_hex("#8AA3B0").expect("focus");
    theme.scrollbar = gpui::Hsla::parse_hex("#C9D7DF")
        .expect("horizon")
        .opacity(0.0);
    theme.scrollbar_thumb = gpui::Hsla::parse_hex("#4A6170")
        .expect("slate")
        .opacity(0.55);
    theme.scrollbar_thumb_hover = gpui::Hsla::parse_hex("#4A6170").expect("slate");
    theme.scrollbar_show = ScrollbarShow::Always;
}

/// Bind TextInput key chords on the application keymap.
pub fn bind_text_input_keys(cx: &mut App) {
    use crate::text_input::{
        Backspace, Copy, Cut, Delete, DeleteToLineStart, DeleteWordLeft, Down, End, Home, Left,
        Newline, Paste, Right, SelectAll, SelectDown, SelectEnd, SelectHome, SelectLeft,
        SelectRight, SelectUp, SelectWordLeft, SelectWordRight, Submit, Up, WordLeft, WordRight,
    };
    use gpui::KeyBinding;

    for context in ["LineInput", "MultilineInput"] {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, Some(context)),
            KeyBinding::new("delete", Delete, Some(context)),
            KeyBinding::new("left", Left, Some(context)),
            KeyBinding::new("right", Right, Some(context)),
            KeyBinding::new("up", Up, Some(context)),
            KeyBinding::new("down", Down, Some(context)),
            KeyBinding::new("shift-left", SelectLeft, Some(context)),
            KeyBinding::new("shift-right", SelectRight, Some(context)),
            KeyBinding::new("shift-up", SelectUp, Some(context)),
            KeyBinding::new("shift-down", SelectDown, Some(context)),
            KeyBinding::new("home", Home, Some(context)),
            KeyBinding::new("end", End, Some(context)),
            KeyBinding::new("shift-home", SelectHome, Some(context)),
            KeyBinding::new("shift-end", SelectEnd, Some(context)),
            KeyBinding::new("cmd-left", Home, Some(context)),
            KeyBinding::new("cmd-right", End, Some(context)),
            KeyBinding::new("ctrl-left", Home, Some(context)),
            KeyBinding::new("ctrl-right", End, Some(context)),
            KeyBinding::new("cmd-shift-left", SelectHome, Some(context)),
            KeyBinding::new("cmd-shift-right", SelectEnd, Some(context)),
            KeyBinding::new("ctrl-shift-left", SelectHome, Some(context)),
            KeyBinding::new("ctrl-shift-right", SelectEnd, Some(context)),
            KeyBinding::new("alt-left", WordLeft, Some(context)),
            KeyBinding::new("alt-right", WordRight, Some(context)),
            KeyBinding::new("alt-shift-left", SelectWordLeft, Some(context)),
            KeyBinding::new("alt-shift-right", SelectWordRight, Some(context)),
            KeyBinding::new("cmd-a", SelectAll, Some(context)),
            KeyBinding::new("ctrl-a", SelectAll, Some(context)),
            KeyBinding::new("cmd-c", Copy, Some(context)),
            KeyBinding::new("ctrl-c", Copy, Some(context)),
            KeyBinding::new("cmd-x", Cut, Some(context)),
            KeyBinding::new("ctrl-x", Cut, Some(context)),
            KeyBinding::new("cmd-v", Paste, Some(context)),
            KeyBinding::new("ctrl-v", Paste, Some(context)),
            KeyBinding::new("alt-backspace", DeleteWordLeft, Some(context)),
            KeyBinding::new("cmd-backspace", DeleteToLineStart, Some(context)),
            KeyBinding::new("ctrl-backspace", DeleteToLineStart, Some(context)),
        ]);
    }
    cx.bind_keys([
        KeyBinding::new("enter", Submit, Some("LineInput")),
        KeyBinding::new("enter", Newline, Some("MultilineInput")),
        KeyBinding::new("cmd-enter", Submit, Some("MultilineInput")),
        KeyBinding::new("ctrl-enter", Submit, Some("MultilineInput")),
        KeyBinding::new("cmd-n", NewCapture, Some("Tinkerway")),
        KeyBinding::new("ctrl-n", NewCapture, Some("Tinkerway")),
        // `?` opens Shortcuts only after the writing field has been left.
        KeyBinding::new(
            "shift-/",
            OpenShortcuts,
            Some("Tinkerway && !MultilineInput && !LineInput && !Input"),
        ),
        KeyBinding::new(
            "?",
            OpenShortcuts,
            Some("Tinkerway && !MultilineInput && !LineInput && !Input"),
        ),
        KeyBinding::new("escape", CloseShortcuts, Some("Tinkerway")),
        KeyBinding::new("escape", CloseShortcuts, Some("Input")),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Home,
    Shortcuts,
}

pub struct TinkerwayApp {
    compose_input: Entity<InputState>,
    body_input: Entity<TextInput>,
    notes: Vec<NoteMeta>,
    /// The capture being written. Saved, and kept off the list until Send.
    draft_id: Option<NoteId>,
    /// An older capture, shown read-only.
    selected_id: Option<NoteId>,
    screen: Screen,
    /// True while Command (or Control on Linux) is held.
    keys_visible: bool,
    draft_dirty: bool,
    status: SharedString,
    vault: Vault,
    focus_handle: FocusHandle,
    save_generation: u64,
    _save_task: Option<Task<()>>,
    dirty: bool,
    list_scroll: ScrollHandle,
}

impl TinkerwayApp {
    pub fn new(
        compose_input: Entity<InputState>,
        body_input: Entity<TextInput>,
        vault: Vault,
        cx: &mut Context<Self>,
    ) -> Self {
        apply_field_theme(cx);
        let mut app = Self {
            compose_input,
            body_input,
            notes: Vec::new(),
            draft_id: None,
            selected_id: None,
            screen: Screen::Home,
            keys_visible: false,
            draft_dirty: false,
            status: SharedString::default(),
            vault,
            focus_handle: cx.focus_handle(),
            save_generation: 0,
            _save_task: None,
            dirty: false,
            list_scroll: ScrollHandle::new(),
        };
        app.refresh_notes(cx);
        app
    }

    /// Open vault with an explicit key (tests / Linux CI). Skips Keychain.
    pub fn new_with_test_vault(
        compose_input: Entity<InputState>,
        body_input: Entity<TextInput>,
        root: PathBuf,
        cx: &mut Context<Self>,
    ) -> Result<Self, String> {
        let vault = Vault::open_with_key(root, MasterKey::generate()).map_err(|e| e.to_string())?;
        Ok(Self::new(compose_input, body_input, vault, cx))
    }

    /// Wire the writing field. Command-Enter (Control-Enter on Linux) sends.
    /// Enter stays a new line. The clear runs after the field update ends.
    pub fn connect_compose(
        app: &Entity<Self>,
        compose: &Entity<InputState>,
        window: &mut Window,
        cx: &mut App,
    ) {
        app.update(cx, |_, cx| {
            cx.subscribe_in(
                compose,
                window,
                |this, _input, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => this.on_compose_changed(cx),
                    InputEvent::PressEnter { secondary: true } => {
                        cx.defer_in(window, |this, window, cx| {
                            this.send_secondary_enter(window, cx);
                        });
                    }
                    InputEvent::Focus | InputEvent::Blur => cx.notify(),
                    InputEvent::PressEnter { secondary: false } => {}
                },
            )
            .detach();
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

    pub fn compose_input(&self) -> &Entity<InputState> {
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

    pub fn draft_id(&self) -> Option<&NoteId> {
        self.draft_id.as_ref()
    }

    /// Leave shortcuts and any open capture. The writing field is home.
    pub fn go_home(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Home;
        self.selected_id = None;
        cx.notify();
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

    /// Load a capture. The one you are writing stays editable. An older one is read-only.
    pub fn open_note(&mut self, id: &NoteId, cx: &mut Context<Self>) {
        self.flush_draft(cx);
        self.flush_autosave(cx);
        self.screen = Screen::Home;

        if self.draft_id.as_ref() == Some(id) {
            self.selected_id = None;
            cx.notify();
            return;
        }

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

    /// Send the current words into the list. Returns whether that succeeded.
    /// Does not clear the field. Send clears the field after this returns true.
    pub fn submit_compose(&mut self, text: &str, cx: &mut Context<Self>) -> bool {
        if text.trim().is_empty() {
            return false;
        }
        self.flush_autosave(cx);
        self.save_generation = self.save_generation.wrapping_add(1);
        let saved = if let Some(id) = self.draft_id.clone() {
            self.vault.update_note(&id, text).map(|()| id)
        } else {
            self.vault.create_note(text)
        };
        match saved {
            Ok(_) => {
                let title = title_from_body(text);
                self.draft_id = None;
                self.draft_dirty = false;
                self.selected_id = None;
                self.screen = Screen::Home;
                self.refresh_notes(cx);
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

    fn on_compose_changed(&mut self, cx: &mut Context<Self>) {
        if self.selected_id.is_some() {
            return;
        }
        self.draft_dirty = true;
        self.schedule_autosave(cx);
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
                    app.flush_draft(cx);
                    app.flush_autosave(cx);
                }
            })
            .ok();
        }));
    }

    fn flush_draft(&mut self, cx: &mut Context<Self>) {
        if !self.draft_dirty {
            return;
        }
        let body = self.compose_input.read(cx).value().to_string();
        if body.trim().is_empty() {
            self.draft_dirty = false;
            return;
        }
        let saved = if let Some(id) = self.draft_id.clone() {
            self.vault.update_note(&id, &body).map(|()| id)
        } else {
            self.vault.create_note(&body)
        };
        match saved {
            Ok(id) => {
                self.draft_id = Some(id);
                self.draft_dirty = false;
                let title = title_from_body(&body);
                self.status = format!("saved “{title}”").into();
                self.refresh_notes(cx);
            }
            Err(err) => {
                self.status = format!("save failed: {err}").into();
                cx.notify();
            }
        }
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

    fn on_send_click(
        &mut self,
        _: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.send_capture(window, cx);
    }

    fn send_capture(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.compose_input.read(cx).value().to_string();
        if self.submit_compose(&text, cx) {
            self.clear_compose(window, cx);
        }
    }

    /// The text area inserts a new line before PressEnter. Remove that line, then send.
    fn send_secondary_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut text = self.compose_input.read(cx).value().to_string();
        if text.ends_with('\n') {
            text.pop();
        }
        if self.submit_compose(&text, cx) {
            self.clear_compose(window, cx);
        } else {
            self.compose_input.update(cx, |input, cx| {
                input.set_value(text, window, cx);
            });
        }
    }

    fn clear_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.compose_input.update(cx, |input, cx| {
            input.set_value("", window, cx);
        });
        window.focus(&self.compose_input.focus_handle(cx));
    }

    /// Save the current words when the field has content, then open a clear field.
    fn start_new(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.compose_input.read(cx).value().to_string();
        if !text.trim().is_empty() {
            self.send_capture(window, cx);
            return;
        }
        self.go_home(cx);
        window.focus(&self.compose_input.focus_handle(cx));
    }

    fn open_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Shortcuts;
        cx.notify();
    }

    fn close_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen == Screen::Shortcuts {
            self.screen = Screen::Home;
            cx.notify();
            return;
        }
        if self.compose_input.focus_handle(cx).is_focused(window) {
            window.focus(&self.focus_handle);
            cx.notify();
        }
    }

    fn on_brand_click(
        &mut self,
        _: &gpui::MouseUpEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_home(cx);
        window.focus(&self.compose_input.focus_handle(cx));
    }
}

fn when_label(updated_unix: u64, is_draft: bool) -> &'static str {
    if is_draft {
        return "Now";
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(updated_unix);
    let age = now.saturating_sub(updated_unix);
    if age < 120 {
        "Now"
    } else if age < 18 * 3600 {
        "Today"
    } else if age < 42 * 3600 {
        "Yesterday"
    } else {
        const DAYS: [&str; 7] = [
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
        ];
        DAYS[(updated_unix / 86_400) as usize % 7]
    }
}

fn keycap(label: &'static str) -> impl IntoElement {
    div()
        .h(px(32.))
        .px_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xc9d7df))
        .bg(rgb(0xf7fbfc))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(14.))
        .child(label)
}

fn shortcut_row(keys: impl IntoElement, label: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_5()
        .h(px(56.))
        .border_b_1()
        .border_color(rgb(0xc9d7df))
        .child(
            div()
                .w(px(210.))
                .flex_shrink_0()
                .flex()
                .flex_row()
                .items_center()
                .child(keys),
        )
        .child(div().text_size(px(18.)).child(label))
}

fn shortcuts_page() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .px(px(44.))
        .pt(px(8.))
        .gap_8()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .text_size(px(32.))
                .child("Hold")
                .child(keycap("Cmd"))
                .child("to see the shortcut keys anywhere in the app."),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .w(px(520.))
                .child(shortcut_row(keycap("?"), "Shortcuts"))
                .child(shortcut_row(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(keycap("Cmd"))
                        .child(div().text_color(rgb(0x7e96a4)).child("+"))
                        .child(keycap("Enter")),
                    "Send",
                ))
                .child(shortcut_row(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(keycap("Cmd"))
                        .child(div().text_color(rgb(0x7e96a4)).child("+"))
                        .child(keycap("N")),
                    "New",
                ))
                .child(shortcut_row(keycap("esc"), "Leave")),
        )
}

fn shortcuts_hint(cx: &mut Context<TinkerwayApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .justify_end()
        .items_center()
        .px(px(44.))
        .pb(px(28.))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .cursor_pointer()
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.open_shortcuts(cx)),
                )
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(0x4a6170))
                        .child("Shortcuts"),
                )
                .child(keycap("?")),
        )
}

impl TinkerwayApp {
    fn chrome(&self, on_shortcuts: bool, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(44.))
            .pt(px(28.))
            .pb(px(12.))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_3()
                    .child(img("brand/tinkerway-icon.png").size(px(32.)))
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .cursor_pointer()
                            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_brand_click))
                            .child("Tinkerway"),
                    )
                    .when(on_shortcuts, |row| {
                        row.child(div().text_color(rgb(0x7e96a4)).child(">")).child(
                            div()
                                .text_size(px(16.))
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .child("Shortcuts"),
                        )
                    }),
            )
            .when(on_shortcuts, |bar| {
                bar.child(
                    div()
                        .w(px(32.))
                        .h(px(32.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(22.))
                        .cursor_pointer()
                        .on_mouse_up(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| this.close_shortcuts(window, cx)),
                        )
                        .child("×"),
                )
            })
    }

    fn home_stage(
        &self,
        notes: Vec<NoteMeta>,
        selected_id: Option<NoteId>,
        draft_id: Option<NoteId>,
        reading: bool,
        reading_text: String,
        keys_visible: bool,
        can_send: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let shown: Vec<NoteMeta> = notes
            .into_iter()
            .filter(|note| draft_id.as_ref() != Some(&note.id))
            .collect();
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
            .gap(px(56.))
            .px(px(44.))
            .pt(px(16.))
            .child(
                div()
                    .id("capture-field")
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .h_full()
                    .overflow_hidden()
                    .bg(rgb(0xf7fbfc))
                    .rounded(px(20.))
                    .when(reading, |card| {
                        card.border_1().border_color(rgb(0xc9d7df)).p(px(36.))
                    })
                    .child(if reading {
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(0x4a6170))
                                    .child("View"),
                            )
                            .child(
                                div()
                                    .text_size(px(28.))
                                    .line_height(px(38.))
                                    .text_color(rgb(0x3e5563))
                                    .child(SharedString::from(reading_text)),
                            )
                            .into_any_element()
                    } else {
                        Input::new(&self.compose_input)
                            .h_full()
                            .px(px(36.))
                            .pt(px(36.))
                            .pb(px(80.))
                            .text_size(px(28.))
                            .line_height(px(38.))
                            .text_color(rgb(0x142a3a))
                            .font_family(".SystemUIFont")
                            .into_any_element()
                    })
                    .when(!reading, |card| {
                        card.child(
                            div()
                                .absolute()
                                .bottom(px(20.))
                                .right(px(20.))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_3()
                                .h(px(36.))
                                .px_4()
                                .rounded(px(18.))
                                .bg(rgb(0x142a3a))
                                .text_color(rgb(0xf7fbfc))
                                .cursor_pointer()
                                .when(!can_send, |button| button.opacity(0.35))
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::on_send_click))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_size(px(14.))
                                        .child("Send"),
                                )
                                .when(keys_visible, |button| {
                                    button.child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(rgb(0xc5d4de))
                                            .child("Cmd + Enter"),
                                    )
                                }),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(420.))
                    .h_full()
                    .min_h(px(0.))
                    .overflow_hidden()
                    .gap(px(48.))
                    .child(
                        div().flex().flex_row().justify_end().child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_3()
                                .h(px(40.))
                                .px(px(18.))
                                .rounded(px(20.))
                                .bg(rgb(0x142a3a))
                                .text_color(rgb(0xf7fbfc))
                                .cursor_pointer()
                                .on_mouse_up(
                                    MouseButton::Left,
                                    cx.listener(|this, _, window, cx| {
                                        this.start_new(window, cx);
                                    }),
                                )
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::MEDIUM)
                                        .text_size(px(14.))
                                        .child("New"),
                                )
                                .when(keys_visible, |button| {
                                    button.child(
                                        div()
                                            .text_size(px(13.))
                                            .text_color(rgb(0xc5d4de))
                                            .child("Cmd + N"),
                                    )
                                }),
                        ),
                    )
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h(px(0.))
                            .child(
                                div()
                                    .id("capture-list")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&self.list_scroll)
                                    .flex()
                                    .flex_col()
                                    .children(shown.into_iter().map(|meta| {
                                        let is_marked =
                                            selected_id.as_ref().is_some_and(|id| id == &meta.id);
                                        let open_id = meta.id.clone();
                                        let title = SharedString::from(meta.title);
                                        let when = when_label(meta.updated_unix, false);
                                        div()
                                            .flex()
                                            .flex_row()
                                            .flex_shrink_0()
                                            .items_center()
                                            .gap(px(20.))
                                            .h(px(72.))
                                            .px(px(8.))
                                            .border_b_1()
                                            .border_color(rgb(0xc9d7df))
                                            .cursor_pointer()
                                            .on_mouse_up(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, window, cx| {
                                                    if this.selected_id.as_ref() == Some(&open_id) {
                                                        this.go_home(cx);
                                                        window.focus(
                                                            &this.compose_input.focus_handle(cx),
                                                        );
                                                    } else {
                                                        this.open_note(&open_id, cx);
                                                    }
                                                }),
                                            )
                                            .child(
                                                div()
                                                    .flex_shrink_0()
                                                    .w(px(3.))
                                                    .h(px(22.))
                                                    .rounded_sm()
                                                    .when(is_marked, |bar| bar.bg(rgb(0x142a3a))),
                                            )
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .overflow_hidden()
                                                    .whitespace_nowrap()
                                                    .text_ellipsis()
                                                    .text_size(px(16.))
                                                    .when(is_marked, |title| {
                                                        title.font_weight(gpui::FontWeight::MEDIUM)
                                                    })
                                                    .child(title),
                                            )
                                            .child(
                                                div()
                                                    .flex_shrink_0()
                                                    .w(px(96.))
                                                    .text_size(px(13.))
                                                    .text_color(rgb(0x7e96a4))
                                                    .text_right()
                                                    .child(when),
                                            )
                                    })),
                            )
                            .child(
                                Scrollbar::vertical(&self.list_scroll)
                                    .scrollbar_show(ScrollbarShow::Always),
                            ),
                    ),
            )
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
        let selected_id = self.selected_id.clone();
        let draft_id = self.draft_id.clone();
        let on_shortcuts = self.screen == Screen::Shortcuts;
        let keys_visible = self.keys_visible;
        let reading = selected_id.is_some();
        let reading_text = if reading {
            self.body_input.read(cx).text().to_string()
        } else {
            String::new()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xe4edf1))
            .text_color(rgb(0x142a3a))
            .font_family(".SystemUIFont")
            .key_context("Tinkerway")
            .track_focus(&self.focus_handle)
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                let show = event.modifiers.secondary();
                if this.keys_visible != show {
                    this.keys_visible = show;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &NewCapture, window, cx| this.start_new(window, cx)))
            .on_action(cx.listener(|this, _: &OpenShortcuts, _, cx| this.open_shortcuts(cx)))
            .on_action(
                cx.listener(|this, _: &CloseShortcuts, window, cx| {
                    this.close_shortcuts(window, cx)
                }),
            )
            .on_action(
                cx.listener(|this, _: &FieldEscape, window, cx| this.close_shortcuts(window, cx)),
            )
            .child(self.chrome(on_shortcuts, cx))
            .child(if on_shortcuts {
                shortcuts_page().into_any_element()
            } else {
                let can_send = !self.compose_input.read(cx).value().trim().is_empty();
                self.home_stage(
                    notes,
                    selected_id,
                    draft_id,
                    reading,
                    reading_text,
                    keys_visible,
                    can_send,
                    cx,
                )
                .into_any_element()
            })
            .child(shortcuts_hint(cx))
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

    fn open_test_app(
        cx: &mut TestAppContext,
        dir: PathBuf,
    ) -> (
        gpui::WindowHandle<gpui_component::Root>,
        Entity<TinkerwayApp>,
    ) {
        cx.update(|cx| {
            gpui_component::init(cx);
            bind_text_input_keys(cx);
        });
        let mut app_slot = None;
        let window = cx
            .update(|cx| {
                let vault_root = dir.clone();
                cx.open_window(Default::default(), |window, cx| {
                    let compose = cx.new(|cx| {
                        InputState::new(window, cx)
                            .multi_line(true)
                            .placeholder("What's new?")
                    });
                    let body = cx.new(|cx| TextInput::new(cx, "body").multiline(4));
                    let app = cx.new(|cx| {
                        TinkerwayApp::new_with_test_vault(
                            compose.clone(),
                            body.clone(),
                            vault_root,
                            cx,
                        )
                        .unwrap()
                    });
                    TinkerwayApp::connect_compose(&app, &compose, window, cx);
                    TinkerwayApp::connect_body(&app, &body, cx);
                    app_slot = Some(app.clone());
                    cx.new(|cx| gpui_component::Root::new(app, window, cx))
                })
            })
            .unwrap();
        (window, app_slot.expect("app"))
    }

    #[gpui::test]
    fn submit_compose_writes_and_lists_without_clearing_input(cx: &mut TestAppContext) {
        let dir = temp_vault_root();
        let (window, app) = open_test_app(cx, dir.clone());

        window
            .update(cx, |_root, window, cx| {
                app.update(cx, |app, cx| {
                    app.compose_input().update(cx, |input, cx| {
                        input.set_value("from unit", window, cx);
                    });
                    assert!(app.submit_compose("from unit", cx));
                });
            })
            .unwrap();
        app.read_with(cx, |app, _| {
            assert_eq!(app.notes().len(), 1);
            assert!(app.status().contains("saved"));
        });
        app.read_with(cx, |app, cx| {
            assert_eq!(app.compose_input().read(cx).value().as_ref(), "from unit");
        });

        let _ = fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn open_note_loads_body_into_editor(cx: &mut TestAppContext) {
        let dir = temp_vault_root();
        let (_window, app) = open_test_app(cx, dir.clone());

        let id = app.update(cx, |app, cx| {
            app.submit_compose("body for click", cx);
            app.notes().first().map(|note| note.id.clone()).unwrap()
        });
        app.update(cx, |app, cx| {
            app.draft_id = None;
            app.selected_id = None;
            app.open_note(&id, cx);
        });
        app.read_with(cx, |app, cx| {
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
        let (window, app) = open_test_app(cx, dir.clone());

        window
            .update(cx, |_root, window, cx| {
                app.update(cx, |app, cx| {
                    app.compose_input.update(cx, |input, cx| {
                        input.set_value("hello from enter", window, cx);
                    });
                    window.focus(&app.compose_input.focus_handle(cx));
                });
            })
            .unwrap();

        cx.dispatch_keystroke(*window, Keystroke::parse("secondary-enter").unwrap());

        window
            .update(cx, |_root, _, cx| {
                app.update(cx, |app, cx| {
                    assert_eq!(app.notes().len(), 1, "note list should refresh");
                    assert!(app.status().contains("saved"), "status={}", app.status());
                    assert!(
                        app.compose_input.read(cx).value().is_empty(),
                        "Send should clear the writing field"
                    );
                });
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

    #[gpui::test]
    fn new_saves_current_words_then_clears_the_field(cx: &mut TestAppContext) {
        let dir = temp_vault_root();
        let (window, app) = open_test_app(cx, dir.clone());

        window
            .update(cx, |_root, window, cx| {
                app.update(cx, |app, cx| {
                    app.compose_input.update(cx, |input, cx| {
                        input.set_value("keep this", window, cx);
                    });
                    app.start_new(window, cx);
                });
            })
            .unwrap();
        app.read_with(cx, |app, cx| {
            assert_eq!(app.notes().len(), 1);
            assert!(app.compose_input().read(cx).value().is_empty());
            assert!(app.selected_id().is_none());
        });

        window
            .update(cx, |_root, window, cx| {
                app.update(cx, |app, cx| app.start_new(window, cx));
            })
            .unwrap();
        app.read_with(cx, |app, _| {
            assert_eq!(app.notes().len(), 1);
        });

        let _ = fs::remove_dir_all(&dir);
    }

    #[gpui::test]
    fn go_home_leaves_shortcuts_and_the_opened_capture(cx: &mut TestAppContext) {
        let dir = temp_vault_root();
        let (_window, app) = open_test_app(cx, dir.clone());

        let id = app.update(cx, |app, cx| {
            app.submit_compose("back home", cx);
            app.notes().first().map(|note| note.id.clone()).unwrap()
        });
        app.update(cx, |app, cx| {
            app.draft_id = None;
            app.open_note(&id, cx);
            app.screen = Screen::Shortcuts;
            app.go_home(cx);
        });
        app.read_with(cx, |app, _| {
            assert_eq!(app.screen, Screen::Home);
            assert!(app.selected_id().is_none());
        });

        let _ = fs::remove_dir_all(&dir);
    }
}
