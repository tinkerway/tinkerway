//! Tauri Demo v1 capture shell. Vault IO stays in `tinkerway-vault`.
//! Commands return note DTOs only — never the master key.

use std::sync::Mutex;

use serde::Serialize;
use tauri::State;
use tinkerway_vault::{
    migrate_legacy_workspace, NoteId, NoteRecord, Vault, VaultError,
};

struct VaultState {
    vault: Mutex<Vault>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NoteRecordDto {
    id: String,
    title: String,
    body: String,
    updated_unix: u64,
}

impl From<NoteRecord> for NoteRecordDto {
    fn from(n: NoteRecord) -> Self {
        Self {
            id: n.id.to_string(),
            title: n.title,
            body: n.body,
            updated_unix: n.updated_unix,
        }
    }
}

fn map_err(err: VaultError) -> String {
    err.to_string()
}

fn with_vault<R>(
    state: &State<'_, VaultState>,
    f: impl FnOnce(&Vault) -> Result<R, VaultError>,
) -> Result<R, String> {
    let vault = state
        .vault
        .lock()
        .map_err(|_| "vault lock poisoned".to_string())?;
    f(&vault).map_err(map_err)
}

fn with_vault_mut<R>(
    state: &State<'_, VaultState>,
    f: impl FnOnce(&mut Vault) -> Result<R, VaultError>,
) -> Result<R, String> {
    let mut vault = state
        .vault
        .lock()
        .map_err(|_| "vault lock poisoned".to_string())?;
    f(&mut vault).map_err(map_err)
}

#[tauri::command]
fn list_notes(state: State<'_, VaultState>) -> Result<Vec<NoteRecordDto>, String> {
    with_vault(&state, |vault| {
        Ok(vault
            .list_captures()?
            .into_iter()
            .map(NoteRecordDto::from)
            .collect())
    })
}

#[tauri::command]
fn read_note(state: State<'_, VaultState>, id: String) -> Result<NoteRecordDto, String> {
    let note_id = NoteId::parse(&id).map_err(map_err)?;
    with_vault(&state, |vault| {
        vault.read_note(&note_id).map(NoteRecordDto::from)
    })
}

#[tauri::command]
fn create_note(state: State<'_, VaultState>, body: String) -> Result<String, String> {
    with_vault_mut(&state, |vault| {
        vault.create_note(&body).map(|id| id.to_string())
    })
}

#[tauri::command]
fn update_note(state: State<'_, VaultState>, id: String, body: String) -> Result<(), String> {
    let note_id = NoteId::parse(&id).map_err(map_err)?;
    with_vault_mut(&state, |vault| vault.update_note(&note_id, &body))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut vault = match Vault::unlock_default() {
        Ok(v) => v,
        Err(err) => {
            eprintln!("tinkerway-tauri: could not unlock vault: {err}");
            eprintln!(
                "tinkerway-tauri: need OS keystore for ai.tinkerway.app / vault-master-key \
                 (macOS Keychain; Linux Secret Service or XDG file fallback)"
            );
            std::process::exit(1);
        }
    };

    if let Err(err) = migrate_legacy_workspace(&mut vault, None) {
        eprintln!("tinkerway-tauri: legacy migrate skipped/failed: {err}");
    }

    tauri::Builder::default()
        .manage(VaultState {
            vault: Mutex::new(vault),
        })
        .invoke_handler(tauri::generate_handler![
            list_notes,
            read_note,
            create_note,
            update_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tinkerway-tauri");
}
