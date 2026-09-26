use gpui::{
    Application, Bounds, Focusable, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use tinkerway::{LineInput, TinkerwayApp, bind_line_input_keys, ensure_workspace, workspace_dir};

fn main() {
    Application::new().run(|cx| {
        bind_line_input_keys(cx);

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
                    let line_input = cx.new(|cx| LineInput::new(cx, "What's on your mind?"));
                    let app =
                        cx.new(|cx| TinkerwayApp::new(line_input.clone(), workspace.clone(), cx));
                    TinkerwayApp::connect_submit(&app, &line_input, cx);
                    app
                },
            )
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.line_input().focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();
    });
}
