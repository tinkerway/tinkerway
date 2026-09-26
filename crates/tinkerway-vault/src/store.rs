//! Vault store: manifest + per-note `.tw` envelopes.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngCore;

use crate::crypto::{self, MasterKey};
use crate::keystore::{self, KeystoreError};

const MANIFEST_NAME: &str = "manifest.json";
const NOTES_DIR: &str = "notes";
const MANIFEST_VERSION: u32 = 1;
const AEAD_ID: &str = "aes-256-gcm";
const NOTE_EXT: &str = "tw";

/// Opaque note id (32 hex chars from 16 random bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NoteId(String);

impl NoteId {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 16];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        Self(hex_encode(&bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(s: &str) -> Result<Self, VaultError> {
        let s = s.trim();
        if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(VaultError::InvalidNoteId);
        }
        Ok(Self(s.to_ascii_lowercase()))
    }
}

impl std::fmt::Display for NoteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct NoteMeta {
    pub id: NoteId,
    pub title: String,
    pub updated_unix: u64,
}

#[derive(Debug, Clone)]
pub struct NoteRecord {
    pub id: NoteId,
    pub title: String,
    pub body: String,
    pub updated_unix: u64,
}

#[derive(Debug)]
pub enum VaultError {
    Io(io::Error),
    Crypto(crypto::CryptoError),
    Keystore(KeystoreError),
    CorruptManifest(String),
    InvalidNoteId,
    NoteNotFound,
    EmptyBody,
}

impl std::fmt::Display for VaultError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Crypto(e) => write!(f, "{e}"),
            Self::Keystore(e) => write!(f, "{e}"),
            Self::CorruptManifest(msg) => write!(f, "corrupt manifest: {msg}"),
            Self::InvalidNoteId => write!(f, "invalid note id"),
            Self::NoteNotFound => write!(f, "note not found"),
            Self::EmptyBody => write!(f, "note body is empty"),
        }
    }
}

impl std::error::Error for VaultError {}

impl From<io::Error> for VaultError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<crypto::CryptoError> for VaultError {
    fn from(value: crypto::CryptoError) -> Self {
        Self::Crypto(value)
    }
}

impl From<KeystoreError> for VaultError {
    fn from(value: KeystoreError) -> Self {
        Self::Keystore(value)
    }
}

struct ManifestEntry {
    id: NoteId,
    updated_unix: u64,
}

struct Manifest {
    version: u32,
    aead: String,
    notes: Vec<ManifestEntry>,
}

/// Unlocked vault handle. Holds the master key in memory for the session.
pub struct Vault {
    root: PathBuf,
    key: MasterKey,
    manifest: Manifest,
}

impl Vault {
    /// Unlock the default vault root using the OS keychain master key.
    pub fn unlock_default() -> Result<Self, VaultError> {
        let root = crate::paths::default_vault_root().ok_or_else(|| {
            VaultError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "could not resolve Application Support / XDG data dir",
            ))
        })?;
        let key = keystore::load_or_create_master_key()?;
        Self::open_with_key(root, key)
    }

    /// Open (or create) a vault at `root` with an explicit master key.
    /// Prefer this in unit tests / Linux CI so Keychain is not required.
    pub fn open_with_key(root: impl Into<PathBuf>, key: MasterKey) -> Result<Self, VaultError> {
        let root = root.into();
        fs::create_dir_all(root.join(NOTES_DIR))?;
        let manifest = load_or_init_manifest(&root)?;
        Ok(Self {
            root,
            key,
            manifest,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// List notes newest-first. Titles come from decrypting each envelope.
    pub fn list_notes(&self) -> Result<Vec<NoteMeta>, VaultError> {
        let mut out = Vec::with_capacity(self.manifest.notes.len());
        for entry in &self.manifest.notes {
            match self.read_payload(&entry.id) {
                Ok((title, _body)) => out.push(NoteMeta {
                    id: entry.id.clone(),
                    title,
                    updated_unix: entry.updated_unix,
                }),
                Err(_) => out.push(NoteMeta {
                    id: entry.id.clone(),
                    title: "(unreadable)".into(),
                    updated_unix: entry.updated_unix,
                }),
            }
        }
        out.sort_by(|a, b| {
            b.updated_unix
                .cmp(&a.updated_unix)
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        Ok(out)
    }

    pub fn read_note(&self, id: &NoteId) -> Result<NoteRecord, VaultError> {
        let (title, body) = self.read_payload(id)?;
        let updated_unix = self
            .manifest
            .notes
            .iter()
            .find(|n| n.id == *id)
            .map(|n| n.updated_unix)
            .unwrap_or(0);
        Ok(NoteRecord {
            id: id.clone(),
            title,
            body,
            updated_unix,
        })
    }

    /// Create a note from body text. Title = first words of body.
    pub fn create_note(&mut self, body: &str) -> Result<NoteId, VaultError> {
        let body = normalize_body(body)?;
        let title = title_from_body(&body);
        let id = NoteId::generate();
        let updated = now_unix()?;
        self.write_payload(&id, &title, &body)?;
        self.manifest.notes.push(ManifestEntry {
            id: id.clone(),
            updated_unix: updated,
        });
        save_manifest(&self.root, &self.manifest)?;
        Ok(id)
    }

    /// Update body (and derived title) for an existing note.
    pub fn update_note(&mut self, id: &NoteId, body: &str) -> Result<(), VaultError> {
        if !self.manifest.notes.iter().any(|n| n.id == *id) {
            return Err(VaultError::NoteNotFound);
        }
        let body = normalize_body(body)?;
        let title = title_from_body(&body);
        let updated = now_unix()?;
        self.write_payload(id, &title, &body)?;
        if let Some(entry) = self.manifest.notes.iter_mut().find(|n| n.id == *id) {
            entry.updated_unix = updated;
        }
        save_manifest(&self.root, &self.manifest)?;
        Ok(())
    }

    fn note_path(&self, id: &NoteId) -> PathBuf {
        self.root
            .join(NOTES_DIR)
            .join(format!("{}.{NOTE_EXT}", id.as_str()))
    }

    fn read_payload(&self, id: &NoteId) -> Result<(String, String), VaultError> {
        let path = self.note_path(id);
        if !path.is_file() {
            return Err(VaultError::NoteNotFound);
        }
        let envelope = fs::read(path)?;
        let plaintext = crypto::open(&self.key, &envelope)?;
        decode_payload(&plaintext)
    }

    fn write_payload(&self, id: &NoteId, title: &str, body: &str) -> Result<(), VaultError> {
        let plaintext = encode_payload(title, body);
        let envelope = crypto::seal(&self.key, &plaintext)?;
        fs::write(self.note_path(id), envelope)?;
        Ok(())
    }
}

/// Title from the first line / first ~8 words of the body.
pub fn title_from_body(body: &str) -> String {
    let first_line = body.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return "Untitled".into();
    }
    let words: Vec<&str> = first_line.split_whitespace().take(8).collect();
    let mut title = words.join(" ");
    if title.chars().count() > 60 {
        title = title.chars().take(57).collect::<String>() + "…";
    }
    if words.len() == 8 && first_line.split_whitespace().count() > 8 {
        title.push('…');
    }
    title
}

fn normalize_body(body: &str) -> Result<String, VaultError> {
    let trimmed = body.trim_end();
    if trimmed.trim().is_empty() {
        return Err(VaultError::EmptyBody);
    }
    Ok(trimmed.to_string())
}

fn encode_payload(title: &str, body: &str) -> Vec<u8> {
    // title_len:u32 BE | title utf-8 | body utf-8
    let title_bytes = title.as_bytes();
    let mut out = Vec::with_capacity(4 + title_bytes.len() + body.len());
    let len = title_bytes.len() as u32;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(title_bytes);
    out.extend_from_slice(body.as_bytes());
    out
}

fn decode_payload(bytes: &[u8]) -> Result<(String, String), VaultError> {
    if bytes.len() < 4 {
        return Err(VaultError::Crypto(crypto::CryptoError::InvalidEnvelope));
    }
    let title_len = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if 4 + title_len > bytes.len() {
        return Err(VaultError::Crypto(crypto::CryptoError::InvalidEnvelope));
    }
    let title = std::str::from_utf8(&bytes[4..4 + title_len])
        .map_err(|_| VaultError::Crypto(crypto::CryptoError::InvalidEnvelope))?
        .to_string();
    let body = std::str::from_utf8(&bytes[4 + title_len..])
        .map_err(|_| VaultError::Crypto(crypto::CryptoError::InvalidEnvelope))?
        .to_string();
    Ok((title, body))
}

fn load_or_init_manifest(root: &Path) -> Result<Manifest, VaultError> {
    let path = root.join(MANIFEST_NAME);
    if !path.exists() {
        let manifest = Manifest {
            version: MANIFEST_VERSION,
            aead: AEAD_ID.into(),
            notes: Vec::new(),
        };
        save_manifest(root, &manifest)?;
        return Ok(manifest);
    }
    let text = fs::read_to_string(&path)?;
    parse_manifest(&text)
}

fn save_manifest(root: &Path, manifest: &Manifest) -> Result<(), VaultError> {
    let path = root.join(MANIFEST_NAME);
    let text = render_manifest(manifest);
    fs::write(path, text)?;
    Ok(())
}

fn render_manifest(manifest: &Manifest) -> String {
    let mut notes = String::new();
    for (i, n) in manifest.notes.iter().enumerate() {
        if i > 0 {
            notes.push(',');
        }
        notes.push_str(&format!(
            "{{\"id\":\"{}\",\"updated\":{}}}",
            n.id.as_str(),
            n.updated_unix
        ));
    }
    format!(
        "{{\"version\":{},\"aead\":\"{}\",\"notes\":[{}]}}\n",
        manifest.version, manifest.aead, notes
    )
}

fn parse_manifest(text: &str) -> Result<Manifest, VaultError> {
    let text = text.trim();
    let version = extract_u32_field(text, "version")
        .ok_or_else(|| VaultError::CorruptManifest("missing version".into()))?;
    let aead = extract_string_field(text, "aead")
        .ok_or_else(|| VaultError::CorruptManifest("missing aead".into()))?;
    if aead != AEAD_ID {
        return Err(VaultError::CorruptManifest(format!("unsupported aead {aead}")));
    }
    let notes_slice = extract_array_slice(text, "notes")
        .ok_or_else(|| VaultError::CorruptManifest("missing notes".into()))?;
    let mut notes = Vec::new();
    for obj in split_json_objects(notes_slice) {
        let id_str = extract_string_field(obj, "id")
            .ok_or_else(|| VaultError::CorruptManifest("note missing id".into()))?;
        let updated = extract_u64_field(obj, "updated").unwrap_or(0);
        notes.push(ManifestEntry {
            id: NoteId::parse(&id_str)?,
            updated_unix: updated,
        });
    }
    Ok(Manifest {
        version,
        aead: aead.to_string(),
        notes,
    })
}

fn extract_string_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let idx = json.find(&pattern)?;
    let after = &json[idx + pattern.len()..];
    let after = after.trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let end = after.find('"')?;
    Some(&after[..end])
}

fn extract_u32_field(json: &str, key: &str) -> Option<u32> {
    extract_number_field(json, key)?.parse().ok()
}

fn extract_u64_field(json: &str, key: &str) -> Option<u64> {
    extract_number_field(json, key)?.parse().ok()
}

fn extract_number_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let idx = json.find(&pattern)?;
    let after = &json[idx + pattern.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let end = after
        .find(|c: char| !(c.is_ascii_digit()))
        .unwrap_or(after.len());
    if end == 0 {
        return None;
    }
    Some(&after[..end])
}

fn extract_array_slice<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{key}\"");
    let idx = json.find(&pattern)?;
    let after = &json[idx + pattern.len()..];
    let after = after.trim_start().strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('[')?;
    let end = after.rfind(']')?;
    Some(&after[..end])
}

fn split_json_objects(array_inner: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (i, ch) in array_inner.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            out.push(&array_inner[s..=i]);
                        }
                        start = None;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn now_unix() -> Result<u64, VaultError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| VaultError::Io(io::Error::other(e)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_vault() -> PathBuf {
        let mut dir = env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        dir.push(format!("tinkerway-vault-test-{nanos}"));
        dir
    }

    #[test]
    fn create_list_read_update() {
        let dir = temp_vault();
        let key = MasterKey::generate();
        let mut vault = Vault::open_with_key(&dir, key).unwrap();

        let id = vault
            .create_note("Hello world\n\nMore markdown here.")
            .unwrap();
        let listed = vault.list_notes().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Hello world");
        assert_eq!(listed[0].id, id);

        let note = vault.read_note(&id).unwrap();
        assert_eq!(note.body, "Hello world\n\nMore markdown here.");

        vault
            .update_note(&id, "Updated title line\nbody")
            .unwrap();
        let listed = vault.list_notes().unwrap();
        assert_eq!(listed[0].title, "Updated title line");
        assert_eq!(vault.read_note(&id).unwrap().body, "Updated title line\nbody");

        // Ciphertext on disk — not plaintext markdown.
        let raw = fs::read(dir.join("notes").join(format!("{id}.tw"))).unwrap();
        assert!(raw.starts_with(b"TW01"));
        assert!(!String::from_utf8_lossy(&raw).contains("Updated title"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn title_from_first_words() {
        assert_eq!(title_from_body("one two three"), "one two three");
        assert_eq!(
            title_from_body("a b c d e f g h i j"),
            "a b c d e f g h…"
        );
        assert_eq!(title_from_body("   \n"), "Untitled");
    }

    #[test]
    fn rejects_empty() {
        let dir = temp_vault();
        let mut vault = Vault::open_with_key(&dir, MasterKey::generate()).unwrap();
        assert!(matches!(
            vault.create_note("   "),
            Err(VaultError::EmptyBody)
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn disk_is_ciphertext() {
        let dir = temp_vault();
        let key = MasterKey::generate();
        let mut vault = Vault::open_with_key(&dir, key.clone()).unwrap();
        let id = vault.create_note("super secret content").unwrap();
        let path = dir.join("notes").join(format!("{id}.tw"));
        let bytes = fs::read(&path).unwrap();
        assert!(!bytes.windows(6).any(|w| w == b"secret"));

        // Wrong key cannot open.
        let vault2 = Vault::open_with_key(&dir, MasterKey::generate()).unwrap();
        assert!(vault2.read_note(&id).is_err());

        let _ = fs::remove_dir_all(&dir);
    }
}
