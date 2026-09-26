//! tinkerway desktop app library — first slice UI and workspace helpers.

mod home;
mod text_input;
mod workspace_files;

pub use home::{TinkerwayApp, bind_line_input_keys};
pub use text_input::{
    Backspace, Delete, End, Home, Left, LineInput, Right, Submit,
};
pub use workspace_files::{
    WORKSPACE_DIR_NAME, ensure_workspace, list_notes, read_note, workspace_dir, write_note,
};
