//! tinkerway desktop app library — GPUI shell over tinkerway-vault.

mod assets;
mod home;
mod text_input;

pub use assets::Assets;
pub use home::{TinkerwayApp, bind_text_input_keys};
pub use text_input::{
    Backspace, Delete, End, Home, Left, Newline, Right, Submit, TextInput,
};
