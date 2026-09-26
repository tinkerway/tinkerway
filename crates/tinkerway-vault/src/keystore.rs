//! OS keychain (Keychain on macOS) for the vault master key.

use crate::crypto::{CryptoError, MasterKey};

pub const KEYCHAIN_SERVICE: &str = "ai.tinkerway.app";
pub const KEYCHAIN_USER: &str = "vault-master-key";

#[derive(Debug)]
pub enum KeystoreError {
    Keyring(String),
    Crypto(CryptoError),
}

impl std::fmt::Display for KeystoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Keyring(msg) => write!(f, "keychain: {msg}"),
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
///
/// On Linux CI (no Apple keychain feature applying), `keyring` uses its mock
/// store — fine within a process; tests should prefer [`MasterKey::generate`]
/// + [`crate::Vault::open_with_key`] instead.
pub fn load_or_create_master_key() -> Result<MasterKey, KeystoreError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_works_with_platform_or_mock() {
        // On Linux this hits keyring's mock (no apple-native applying).
        // Mock credentials are not shared across Entry instances, so we only
        // assert the happy path of create-on-missing within one call.
        let key = load_or_create_master_key().expect("create master key");
        assert_eq!(key.as_bytes().len(), 32);
    }
}
