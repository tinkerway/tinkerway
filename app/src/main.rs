use gpui::{
    prelude::*, px, size, Application, Bounds, Focusable, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use gpui_component::input::InputState;
use gpui_component::Root;
use tinkerway::{bind_text_input_keys, Assets, TextInput, TinkerwayApp};
use tinkerway_vault::Vault;

fn main() {
    Application::new().with_assets(Assets).run(|cx| {
        #[cfg(target_os = "macos")]
        tinkerway::apply_dock_icon();

        gpui_component::init(cx);
        bind_text_input_keys(cx);

        let vault = match Vault::unlock_default() {
            Ok(v) => v,
            Err(err) => {
                eprintln!("tinkerway: could not unlock vault: {err}");
                eprintln!(
                    "tinkerway: need OS keystore for ai.tinkerway.app / vault-master-key \
                     (macOS Keychain; Linux Secret Service or XDG file fallback — see app/README.md)"
                );
                return;
            }
        };

        let bounds = Bounds::centered(None, size(px(1200.), px(800.)), cx);
        let mut app_slot = None;
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("Tinkerway".into()),
                        ..Default::default()
                    }),
                    app_id: Some("ai.tinkerway.app".into()),
                    ..Default::default()
                },
                |window, cx| {
                    let compose = cx.new(|cx| {
                        InputState::new(window, cx)
                            .multi_line(true)
                            .rows(6)
                            .placeholder("What's new?")
                    });
                    let body = cx.new(|cx| TextInput::new(cx, "Capture").multiline(8));
                    let app = cx.new(|cx| {
                        let mut app = TinkerwayApp::new(compose.clone(), body.clone(), vault, cx);
                        app.migrate_legacy_if_needed(cx);
                        app
                    });
                    TinkerwayApp::connect_compose(&app, &compose, window, cx);
                    TinkerwayApp::connect_body(&app, &body, cx);
                    app_slot = Some(app.clone());
                    cx.new(|cx| Root::new(app, window, cx))
                },
            )
            .unwrap();

        let app = app_slot.expect("app");
        window
            .update(cx, |_root, window, cx| {
                app.update(cx, |app, cx| {
                    window.focus(&app.compose_input().focus_handle(cx));
                });
                cx.activate(true);
            })
            .unwrap();
    });
}
