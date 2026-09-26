//! Application Support / XDG paths for the vault.

use std::path::PathBuf;

use directories::BaseDirs;

/// Bundle-ish folder name under Application Support / XDG data home.
pub const APP_DATA_FOLDER: &str = "ai.tinkerway.app";

/// Root app data directory (`~/Library/Application Support/ai.tinkerway.app` on Mac).
pub fn app_data_dir() -> Option<PathBuf> {
    BaseDirs::new().map(|dirs| dirs.data_dir().join(APP_DATA_FOLDER))
}

/// Default vault root: `<app_data>/vault`.
pub fn default_vault_root() -> Option<PathBuf> {
    app_data_dir().map(|d| d.join("vault"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_vault_under_app_data() {
        let root = default_vault_root().expect("base dirs");
        assert!(root.ends_with("vault"));
        let s = root.to_string_lossy();
        assert!(
            s.contains(APP_DATA_FOLDER),
            "unexpected path {s}"
        );
    }
}
