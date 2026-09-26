//! One-shot migration from legacy plaintext `.tinkerway-workspace/`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::store::{Vault, VaultError};

/// Legacy cwd-relative plaintext notes folder from the first slice.
pub const LEGACY_WORKSPACE_DIR: &str = ".tinkerway-workspace";

#[derive(Debug, Default)]
pub struct MigrateReport {
    pub migrated: usize,
    pub skipped: usize,
    pub deleted: usize,
}

/// Detect cwd `.tinkerway-workspace/*.md`, encrypt into `vault`, delete plaintext
/// after a verified decrypt round-trip.
pub fn migrate_legacy_workspace(
    vault: &mut Vault,
    legacy_dir: Option<&Path>,
) -> Result<MigrateReport, VaultError> {
    let legacy = legacy_dir
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(LEGACY_WORKSPACE_DIR));

    if !legacy.is_dir() {
        return Ok(MigrateReport::default());
    }

    let mut report = MigrateReport::default();
    let mut entries: Vec<PathBuf> = fs::read_dir(&legacy)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    entries.sort();

    for path in entries {
        let body = match fs::read_to_string(&path) {
            Ok(b) => b,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        if body.trim().is_empty() {
            // Empty file — delete without migrating.
            let _ = fs::remove_file(&path);
            report.deleted += 1;
            continue;
        }

        let id = match vault.create_note(&body) {
            Ok(id) => id,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };

        // Verify decrypt round-trip before deleting plaintext.
        match vault.read_note(&id) {
            Ok(note) if note.body.trim_end() == body.trim_end() => {
                fs::remove_file(&path).map_err(VaultError::from)?;
                report.migrated += 1;
                report.deleted += 1;
            }
            _ => {
                report.skipped += 1;
            }
        }
    }

    // Remove legacy dir if empty.
    if let Ok(mut rd) = fs::read_dir(&legacy) {
        if rd.next().is_none() {
            let _ = fs::remove_dir(&legacy);
        }
    }

    Ok(report)
}

/// Detect whether a cwd-relative legacy plaintext workspace directory exists.
#[allow(dead_code)]
pub fn legacy_workspace_exists(cwd: &Path) -> bool {
    cwd.join(LEGACY_WORKSPACE_DIR).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::MasterKey;
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_pair() -> (PathBuf, PathBuf) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = env::temp_dir().join(format!("tinkerway-migrate-{nanos}"));
        let legacy = base.join(LEGACY_WORKSPACE_DIR);
        let vault_root = base.join("vault");
        fs::create_dir_all(&legacy).unwrap();
        (legacy, vault_root)
    }

    #[test]
    fn migrates_and_deletes_plaintext() {
        let (legacy, vault_root) = temp_pair();
        fs::write(legacy.join("note.md"), "legacy secret note\n").unwrap();

        let mut vault = Vault::open_with_key(&vault_root, MasterKey::generate()).unwrap();
        let report = migrate_legacy_workspace(&mut vault, Some(&legacy)).unwrap();
        assert_eq!(report.migrated, 1);
        assert_eq!(report.deleted, 1);
        assert!(!legacy.join("note.md").exists());

        let listed = vault.list_notes().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "legacy secret note");

        let parent = legacy.parent().unwrap();
        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn no_legacy_is_ok() {
        let missing = env::temp_dir().join("tinkerway-no-legacy-should-miss");
        let _ = fs::remove_dir_all(&missing);
        let vault_root = missing.join("vault");
        let mut vault = Vault::open_with_key(&vault_root, MasterKey::generate()).unwrap();
        let report = migrate_legacy_workspace(&mut vault, Some(&missing)).unwrap();
        assert_eq!(report.migrated, 0);
        let _ = fs::remove_dir_all(&vault_root);
    }
}
