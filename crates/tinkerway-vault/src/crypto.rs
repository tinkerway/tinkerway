//! Master key + AES-256-GCM envelope helpers.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// 32-byte AES-256 key. Zeroized on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);

impl MasterKey {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn try_from_slice(bytes: &[u8]) -> Result<Self, CryptoError> {
        let arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKeyLength)?;
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex encode for Keychain storage (UTF-8 password field).
    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, CryptoError> {
        let bytes = hex_decode(s)?;
        Self::try_from_slice(&bytes)
    }
}

const ENVELOPE_MAGIC: &[u8; 4] = b"TW01";
const NONCE_LEN: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKeyLength,
    InvalidHex,
    InvalidEnvelope,
    Encrypt,
    Decrypt,
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidKeyLength => write!(f, "master key must be 32 bytes"),
            Self::InvalidHex => write!(f, "invalid hex master key"),
            Self::InvalidEnvelope => write!(f, "invalid .tw envelope"),
            Self::Encrypt => write!(f, "encryption failed"),
            Self::Decrypt => write!(f, "decryption failed"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Seal plaintext into a `.tw` envelope: magic || nonce || ciphertext+tag.
pub fn seal(key: &MasterKey, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::Encrypt)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| CryptoError::Encrypt)?;

    let mut out = Vec::with_capacity(ENVELOPE_MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(ENVELOPE_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Open a `.tw` envelope to plaintext bytes.
pub fn open(key: &MasterKey, envelope: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if envelope.len() < ENVELOPE_MAGIC.len() + NONCE_LEN + 16 {
        return Err(CryptoError::InvalidEnvelope);
    }
    if &envelope[..4] != ENVELOPE_MAGIC {
        return Err(CryptoError::InvalidEnvelope);
    }
    let nonce_bytes = &envelope[4..4 + NONCE_LEN];
    let ciphertext = &envelope[4 + NONCE_LEN..];
    let cipher = Aes256Gcm::new_from_slice(key.as_bytes()).map_err(|_| CryptoError::Decrypt)?;
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| CryptoError::Decrypt)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn hex_decode(s: &str) -> Result<Vec<u8>, CryptoError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(CryptoError::InvalidHex);
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, CryptoError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(CryptoError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_seal_open() {
        let key = MasterKey::generate();
        let pt = b"hello private note";
        let env = seal(&key, pt).unwrap();
        assert!(env.starts_with(ENVELOPE_MAGIC));
        let out = open(&key, &env).unwrap();
        assert_eq!(out, pt);
    }

    #[test]
    fn unique_nonces() {
        let key = MasterKey::generate();
        let a = seal(&key, b"same").unwrap();
        let b = seal(&key, b"same").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn wrong_key_fails() {
        let env = seal(&MasterKey::generate(), b"secret").unwrap();
        assert!(open(&MasterKey::generate(), &env).is_err());
    }

    #[test]
    fn hex_round_trip() {
        let key = MasterKey::generate();
        let hex = key.to_hex();
        let restored = MasterKey::from_hex(&hex).unwrap();
        assert_eq!(key.as_bytes(), restored.as_bytes());
    }
}
