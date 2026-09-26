//! Set the macOS Dock / app icon from embedded brand PNG bytes.
//!
//! GPUI 0.2.2 has no WindowOptions icon field; NSApplication is the practical path.

#![cfg(target_os = "macos")]

use cocoa::appkit::{NSApp, NSApplication, NSImage};
use cocoa::base::{id, nil};
use cocoa::foundation::NSData;
use objc::{class, msg_send, sel, sel_impl};

const ICON_PNG: &[u8] = include_bytes!("../assets/brand/tinkerway-icon.png");

/// Apply the brand PNG as the running app’s Dock / app icon.
///
/// Call once inside `Application::run`. Failures are logged and ignored so a
/// bad image never prevents launch.
pub fn apply_dock_icon() {
    unsafe {
        let data: id = NSData::dataWithBytes_length_(
            nil,
            ICON_PNG.as_ptr() as *const std::os::raw::c_void,
            ICON_PNG.len() as _,
        );
        if data == nil {
            eprintln!("tinkerway: could not wrap icon PNG as NSData");
            return;
        }

        let image: id = NSImage::alloc(nil).initWithData_(data);
        if image == nil {
            eprintln!("tinkerway: could not decode brand PNG for Dock icon");
            return;
        }

        let app = NSApp();
        if app == nil {
            // NSApp may still be nil very early; try sharedApplication.
            let app: id = msg_send![class!(NSApplication), sharedApplication];
            if app == nil {
                eprintln!("tinkerway: NSApp unavailable; Dock icon not set");
                return;
            }
            app.setApplicationIconImage_(image);
            return;
        }
        app.setApplicationIconImage_(image);
    }
}
