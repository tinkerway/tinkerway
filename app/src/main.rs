mod text_input;
mod workspace_files;

use std::path::PathBuf;

use gpui::{
    App, Application, Bounds, Context, Entity, FocusHandle, Focusable, KeyBinding, MouseButton,
    SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb,
    size, white,
};

use text_input::{Backspace, Delete, End, Home, Left, LineInput, Right, Submit};
use workspace_files::{ensure_workspace, list_notes, workspace_dir, write_note};

struct TinkerwayApp {
    line_input: Entity<LineInput>,
    notes: Vec<SharedString>,
    status: SharedString,
    workspace: PathBuf,
    focus_handle: FocusHandle,
}

impl TinkerwayApp {
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

    fn submit_line(&mut self, line: &str, cx: &mut Context<Self>) {
        match write_note(&self.workspace, line) {
            Ok(name) => {
                self.status = format!("wrote {name}").into();
                self.line_input.update(cx, |input, cx| input.clear(cx));
                self.refresh_notes(cx);
            }
            Err(err) => {
                self.status = format!("write failed: {err}").into();
                cx.notify();
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
        self.submit_line(&line, cx);
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

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, Some("LineInput")),
            KeyBinding::new("delete", Delete, Some("LineInput")),
            KeyBinding::new("left", Left, Some("LineInput")),
            KeyBinding::new("right", Right, Some("LineInput")),
            KeyBinding::new("home", Home, Some("LineInput")),
            KeyBinding::new("end", End, Some("LineInput")),
            KeyBinding::new("enter", Submit, Some("LineInput")),
        ]);

        let workspace = workspace_dir();
        if let Err(err) = ensure_workspace(&workspace) {
            eprintln!("tinkerway: could not create workspace dir: {err}");
        }

        let bounds = Bounds::centered(None, size(px(520.), px(640.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("tinkerway".into()),
                        ..Default::default()
                    }),
                    app_id: Some("ai.tinkerway.app".into()),
                    ..Default::default()
                },
                |_, cx| {
                    let line_input = cx.new(|cx| {
                        let mut input = LineInput::new(cx, "What's on your mind?");
                        // Wired after app entity exists — see below.
                        input.on_submit = None;
                        input
                    });

                    let app = cx.new(|cx| {
                        let mut app = TinkerwayApp {
                            line_input: line_input.clone(),
                            notes: Vec::new(),
                            status: format!("workspace: {}", workspace.display()).into(),
                            workspace: workspace.clone(),
                            focus_handle: cx.focus_handle(),
                        };
                        app.refresh_notes(cx);
                        app
                    });

                    // Connect Enter → write note on the parent app.
                    let app_entity = app.clone();
                    line_input.update(cx, |input, _cx| {
                        input.on_submit = Some(Box::new(move |line, _window, _cx| {
                            app_entity.update(_cx, |app, cx| {
                                app.submit_line(line, cx);
                            });
                        }));
                    });

                    app
                },
            )
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.line_input.focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();
    });
}
