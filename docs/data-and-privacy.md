# Data and privacy

**Product rule:** private by default. Sibling apps on the same Mac must not read note bodies in plaintext.

## Destination (vault-first)

| Piece | Rule |
| --- | --- |
| Note bodies at rest | **Ciphertext** (AEAD envelopes + small manifest). Not open `.md` as the vault. |
| Master key | **macOS Keychain** (Mac default). Never store the key in the vault folder or the repo. |
| Prefs / paths | Plaintext JSON OK (theme, last-selected id, vault root). |
| Plaintext in process | Only in tinkerway memory after unlock. |

Threat model (v0): resist casual filesystem browse and sync/backup of vault files without leaking note text. Full Keychain/process-memory attacks are out of scope for now.

**Do not** claim privacy from “files live under Application Support” alone. Privacy claim = **ciphertext on disk + key elsewhere**.

## Integrations are outbound only

Handoff means **tinkerway →** Cursor agent / MCP / local API (instructions out, results back). It does **not** mean other apps mount or read the vault as markdown.

## Export vs sync

Keep pipes separate:

| Pipe | Moves | Plaintext? |
| --- | --- | --- |
| **Vault sync** | Ciphertext + manifest only | No |
| **Export markdown** | User-triggered “Export…” to a folder they choose | Yes — deliberate |
| **Key** | Keychain only (v0 single Mac) | Never in synced folder |

Export markdown ≠ vault sync. Syncing ciphertext without the key is safe but useless on another device until a wrapped-key story exists.

## Open source vs secrets

Encryption **code** may be public. **Keys, key material, and plaintext vault data never go in the repo.**

## v0 today vs destination

Until vault crates are explicitly approved, the demo slice may still write plaintext under cwd `.tinkerway-workspace/` (gitignored). That is a temporary IO path — **not** the product default to market or keep.

Destination: encrypt at rest under a stable app data root; migrate legacy plaintext once, then stop writing it. **Do not add crypto crates** until approved.
