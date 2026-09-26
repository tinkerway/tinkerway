//! tinkerway desktop app library — GPUI shell over tinkerway-vault.

mod assets;
mod home;
#[cfg(target_os = "macos")]
mod mac_icon;
mod text_input;

pub use assets::Assets;
pub use home::{TinkerwayApp, bind_text_input_keys};
#[cfg(target_os = "macos")]
pub use mac_icon::apply_dock_icon;
pub use text_input::{
    Backspace, Delete, End, Home, Left, Newline, Right, Submit, TextInput,
};
