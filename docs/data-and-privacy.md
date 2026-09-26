# Data and privacy

**Product rule:** private by default. Sibling apps on the same Mac must not read note bodies in plaintext.

## Vault-first (Demo v1)

| Piece | Rule |
| --- | --- |
| Note bodies at rest | **Ciphertext** — AES-256-GCM envelopes (`.tw`) + small `manifest.json`. Not open `.md` as the vault. |
| Master key | **OS keystore**: macOS Keychain entry `ai.tinkerway.app` / `vault-master-key` (32-byte key, hex). On Linux: Secret Service / keyutils via the same `keyring` API; if unavailable, a **0600** XDG file `…/ai.tinkerway.app/master-key` (dev/cloud VM fallback only). Never store the key in the vault folder or the repo. |
| Vault root | Application Support / XDG: `…/ai.tinkerway.app/vault/` via `directories`. |
| Prefs / paths | Plaintext JSON OK later (theme, last-selected id, vault root). |
| Plaintext in process | Only in tinkerway memory after unlock. |

Threat model (v0): resist casual filesystem browse and sync/backup of vault files without leaking note text. Full Keychain/process-memory attacks are out of scope for now.

**Do not** claim privacy from “files live under Application Support” alone. Privacy claim = **ciphertext on disk + key elsewhere**.

## Layout

```text
~/Library/Application Support/ai.tinkerway.app/   # Mac
~/.local/share/ai.tinkerway.app/                  # Linux (XDG)
  master-key            # Linux fallback only (0600); Mac uses Keychain
  vault/
    manifest.json       # version, aead id, note ids — no bodies
    notes/<id>.tw       # TW01 || nonce || ciphertext+tag
```

Titles are derived from the first words of the body and live **inside** the ciphertext; the list decrypts on unlock.

## Integrations are outbound only

Handoff means **tinkerway →** Cursor agent / MCP / local API (instructions out, results back). It does **not** mean other apps mount or read the vault as markdown.

## Export vs sync

Keep pipes separate:

| Pipe | Moves | Plaintext? |
| --- | --- | --- |
| **Vault sync** | Ciphertext + manifest only | No |
| **Export markdown** | User-triggered “Export…” to a folder they choose (not Demo v1) | Yes — deliberate |
| **Key** | Keychain (Mac) / Secret Service or XDG file (Linux) | Never in synced vault folder |

## Migration

On launch, if cwd `.tinkerway-workspace/*.md` exists, notes are encrypted into the vault and plaintext files are **deleted** after a verified decrypt round-trip. The app no longer writes that folder.

## Linux CI / tests

Most vault unit tests use `Vault::open_with_key` (explicit test key). The keystore smoke test calls `load_or_create_master_key`, which on Linux uses Secret Service when a session D-Bus keyring is present, otherwise the XDG `master-key` file. Production Mac path remains Keychain-only via `keyring` (`apple-native`) — no file fallback.

## Open source vs secrets

Encryption **code** may be public. **Keys, key material, and plaintext vault data never go in the repo.**

## Approved crypto / path crates (Demo v1)

`keyring`, `aes-gcm`, `rand`, `zeroize`, `directories` — do not add further deps without an explicit yes.
