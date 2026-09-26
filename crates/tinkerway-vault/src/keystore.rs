//! OS credential store for the vault master key.
//!
//! - macOS: Keychain via `keyring` (`apple-native`). No file fallback.
//! - Linux: Secret Service / keyutils via `keyring` (`linux-native-sync-persistent`).
//!   If that fails (no D-Bus Secret Service), fall back to a 0600 key file under
//!   XDG data (`…/ai.tinkerway.app/master-key`). Notes stay AES-GCM ciphertext;
//!   the fallback only keeps the master key off the vault folder with filesystem
//!   permissions — prefer a real Secret Service when available.

use crate::crypto::{CryptoError, MasterKey};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::io::Write;
#[cfg(target_os = "linux")]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(target_os = "linux")]
use std::path::PathBuf;

#[cfg(target_os = "linux")]
use crate::paths::app_data_dir;

pub const KEYCHAIN_SERVICE: &str = "ai.tinkerway.app";
pub const KEYCHAIN_USER: &str = "vault-master-key";

/// Filename for the Linux XDG file-backed key fallback (not used on macOS).
pub const LINUX_KEY_FILE: &str = "master-key";

#[derive(Debug)]
pub enum KeystoreError {
    Keyring(String),
    Io(String),
    Crypto(CryptoError),
}

impl std::fmt::Display for KeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyring(msg) => write!(f, "keychain: {msg}"),
            Self::Io(msg) => write!(f, "keystore io: {msg}"),
            Self::Crypto(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for KeystoreError {}

impl From<CryptoError> for KeystoreError {
    fn from(value: CryptoError) -> Self {
        Self::Crypto(value)
    }
}

/// Load the master key from the OS keychain, or generate and store one.
pub fn load_or_create_master_key() -> Result<MasterKey, KeystoreError> {
    #[cfg(target_os = "macos")]
    {
        load_or_create_via_keyring()
    }

    #[cfg(target_os = "linux")]
    {
        match load_or_create_via_keyring() {
            Ok(key) => Ok(key),
            Err(err) => {
                eprintln!(
                    "tinkerway: Secret Service / keyutils unavailable ({err}); using XDG file key at …/{}/{}",
                    crate::paths::APP_DATA_FOLDER,
                    LINUX_KEY_FILE
                );
                load_or_create_via_xdg_file()
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        load_or_create_via_keyring()
    }
}

fn load_or_create_via_keyring() -> Result<MasterKey, KeystoreError> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_USER)
        .map_err(|e| KeystoreError::Keyring(e.to_string()))?;

    match entry.get_password() {
        Ok(hex) => MasterKey::from_hex(&hex).map_err(KeystoreError::from),
        Err(keyring::Error::NoEntry) => {
            let key = MasterKey::generate();
            entry
                .set_password(&key.to_hex())
                .map_err(|e| KeystoreError::Keyring(e.to_string()))?;
            Ok(key)
        }
        Err(err) => Err(KeystoreError::Keyring(err.to_string())),
    }
}

#[cfg(target_os = "linux")]
fn linux_key_path() -> Result<PathBuf, KeystoreError> {
    let dir = app_data_dir().ok_or_else(|| KeystoreError::Io("no XDG data dir".into()))?;
    Ok(dir.join(LINUX_KEY_FILE))
}

/// Linux/dev fallback: hex master key in a 0600 file under the app data dir.
/// Not used on macOS. Prefer Secret Service when the daemon is available.
#[cfg(target_os = "linux")]
fn load_or_create_via_xdg_file() -> Result<MasterKey, KeystoreError> {
    let path = linux_key_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| KeystoreError::Io(e.to_string()))?;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }

    if path.exists() {
        let hex = fs::read_to_string(&path).map_err(|e| KeystoreError::Io(e.to_string()))?;
        let key = MasterKey::from_hex(hex.trim()).map_err(KeystoreError::from)?;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        return Ok(key);
    }

    let key = MasterKey::generate();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| KeystoreError::Io(e.to_string()))?;
    file.write_all(key.to_hex().as_bytes())
        .map_err(|e| KeystoreError::Io(e.to_string()))?;
    file.sync_all()
        .map_err(|e| KeystoreError::Io(e.to_string()))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_works_with_platform_store() {
        let key = load_or_create_master_key().expect("create master key");
        assert_eq!(key.as_bytes().len(), 32);

        // Second call must return the same key (Secret Service, keyutils cache,
        // or XDG file fallback — not the in-process mock).
        let again = load_or_create_master_key().expect("reload master key");
        assert_eq!(key.to_hex(), again.to_hex());
    }
}
