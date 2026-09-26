//! Encrypted note vault — AES-256-GCM envelopes (`.tw`) + Keychain master key.
//!
//! No GPUI dependency. The app shell calls these APIs; crypto and disk IO stay here.

mod crypto;
mod keystore;
mod migrate;
mod paths;
mod store;

pub use crypto::{MasterKey, open as open_envelope, seal};
pub use keystore::{KEYCHAIN_SERVICE, KEYCHAIN_USER, load_or_create_master_key};
pub use migrate::{LEGACY_WORKSPACE_DIR, migrate_legacy_workspace};
pub use paths::{app_data_dir, default_vault_root};
pub use store::{NoteId, NoteMeta, NoteRecord, Vault, VaultError, title_from_body};
