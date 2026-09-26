use gpui::{
    Application, Bounds, Focusable, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use tinkerway::{Assets, TextInput, TinkerwayApp, bind_text_input_keys};
use tinkerway_vault::Vault;

fn main() {
    Application::new().with_assets(Assets).run(|cx| {
        #[cfg(target_os = "macos")]
        tinkerway::apply_dock_icon();

        bind_text_input_keys(cx);

        let vault = match Vault::unlock_default() {
            Ok(v) => v,
            Err(err) => {
                eprintln!("tinkerway: could not unlock vault: {err}");
                eprintln!(
                    "tinkerway: on Mac, Keychain must be available for ai.tinkerway.app / vault-master-key"
                );
                return;
            }
        };

        let bounds = Bounds::centered(None, size(px(560.), px(720.)), cx);
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
                    let compose = cx.new(|cx| {
                        TextInput::new(cx, "What's on your mind?").multiline(4)
                    });
                    let body = cx.new(|cx| TextInput::new(cx, "Note body").multiline(8));
                    let app = cx.new(|cx| {
                        let mut app = TinkerwayApp::new(compose.clone(), body.clone(), vault, cx);
                        app.migrate_legacy_if_needed(cx);
                        app
                    });
                    TinkerwayApp::connect_compose(&app, &compose, cx);
                    TinkerwayApp::connect_body(&app, &body, cx);
                    app
                },
            )
            .unwrap();

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.compose_input().focus_handle(cx));
                cx.activate(true);
            })
            .unwrap();
    });
}
