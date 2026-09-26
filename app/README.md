# tinkerway app

Demo v1: multi-line private notes — encrypted `.tw` vault, Keychain master key, list/open/edit with debounced autosave.

Architecture, privacy, and notes UI norms: [`docs/`](../docs/).

Licensed under the repo root [LICENSE](../LICENSE) (O'Saasy). Not open source.

## Run (macOS)

You need a full Xcode install (not only Command Line Tools) so Metal / the macOS SDK are available. Point the active developer directory at Xcode if needed:

```bash
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
```

Then from the repo root:

```bash
mise install
cargo run -p tinkerway
```

First launch creates a Keychain item (`ai.tinkerway.app` / `vault-master-key`) and a vault under Application Support:

`~/Library/Application Support/ai.tinkerway.app/vault/`

Allow Keychain access when prompted (unsigned `cargo run` binaries may ask each rebuild until you Always Allow).

## Demo path

1. Type a multi-line thought in the compose field (Enter = newline).
2. Click **Add note** or press **⌘↩** — body is sealed as a `.tw` note; list shows a title from the first words.
3. Click a list row to open it in the body editor.
4. Edit the markdown body — changes autosave after a short debounce.
5. Confirm the window shows the tinkerway brand icon (from `app/assets/brand/`).

Legacy plaintext `.tinkerway-workspace/` (if present in cwd) is migrated once into the vault and removed.

## App / Dock icon

Brand assets live under `app/assets/brand/`:

| File | Use |
| --- | --- |
| `tinkerway-icon.png` (512) | Window chrome + Dock via `NSApplication.setApplicationIconImage` on macOS |
| `tinkerway-icon-1024.png` | High-res source |
| `tinkerway.icns` | Packaged `.app` / Finder (multi-size) |
| `icon.iconset/` | Source PNGs used to build the `.icns` |

GPUI **0.2.2** has no `WindowOptions` icon field, so on Mac we set the Dock icon through AppKit after launch. Rebuild the `.icns` anytime with the PNGs in `icon.iconset/` (or `iconutil -c icns icon.iconset` on a Mac).

## Linux

`cargo check` / `cargo test` work with GPUI’s default `wayland` / `x11` features when system libraries are present (see `.github/workflows/rust.yml`). Vault tests use an in-memory test key (`Vault::open_with_key`) so Keychain is not required. A full GUI run still needs a graphical session and GPU stack; day-to-day demo is aimed at Mac + Xcode.

## Dependencies

- [gpui](https://crates.io/crates/gpui) `=0.2.2` — Apache-2.0 (Zed). UI shell only.
- [tinkerway-vault](../crates/tinkerway-vault) — AES-256-GCM envelopes, Keychain via `keyring`, paths via `directories` (`rand`, `zeroize`). No GPUI.
- macOS only: `cocoa` / `objc` (same stack gpui already uses) to set the Dock icon from the brand PNG.
