# tinkerway app

Demo v1: multi-line private notes — encrypted `.tw` vault, OS keystore master key, list/open/edit with debounced autosave.

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

## Run (Linux / Cursor cloud VM)

Day-to-day product demos are still Mac-first; Linux is for CI, Cursor cloud agents, and screenshot/video capture.

### System packages

Same link deps as CI (`.github/workflows/rust.yml`), plus a software Vulkan ICD for headless/VNC GPUs:

```bash
sudo apt-get update
sudo apt-get install -y \
  g++ cmake clang \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libxcb1-dev libxcb-xkb-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-randr0-dev \
  libegl1-mesa-dev libvulkan-dev \
  libfontconfig1-dev libfreetype6-dev \
  libdbus-1-dev libsecret-1-dev \
  mesa-vulkan-drivers gnome-keyring dbus-x11
```

### Toolchain

```bash
mise install   # rust 1.91+ from mise.toml
eval "$(mise activate bash)"
```

### Display + lavapipe (Cursor VMs often use `DISPLAY=:1`)

```bash
export DISPLAY="${DISPLAY:-:1}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/runtime-$UID}"
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"

# Force Mesa llvmpipe when there is no /dev/dri GPU
export VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json
export LIBGL_ALWAYS_SOFTWARE=1
```

If the ICD JSON uses a relative `library_path`, an absolute override works:

```bash
printf '%s\n' '{
  "file_format_version": "1.0.1",
  "ICD": {
    "library_path": "/usr/lib/x86_64-linux-gnu/libvulkan_lvp.so",
    "api_version": "1.4.318"
  }
}' > /tmp/lvp_icd.json
export VK_ICD_FILENAMES=/tmp/lvp_icd.json
```

### Keystore (Secret Service or XDG file)

Prefer a D-Bus Secret Service (libsecret / gnome-keyring). On a fresh Cursor session:

```bash
eval "$(dbus-launch --sh-syntax)"
gnome-keyring-daemon --start --components=secrets
# Unlock if the login collection is locked (empty password is common on VMs):
printf '\n' | gnome-keyring-daemon --unlock
```

If Secret Service / keyutils is unavailable, the vault uses a **0600** hex key file at:

`~/.local/share/ai.tinkerway.app/master-key`

Notes remain AES-256-GCM ciphertext under `…/vault/`. The file fallback is for Linux/dev and cloud VMs only — **macOS stays Keychain-only** (no file fallback).

### Run

```bash
export LIBRARY_PATH="$(dirname "$(g++ -print-file-name=libstdc++.so)"):${LIBRARY_PATH:-}"
cargo run -p tinkerway
```

Vault root on Linux: `~/.local/share/ai.tinkerway.app/vault/`.

Screenshots: `scrot` / `import` against `$DISPLAY`. Short recordings: `ffmpeg -f x11grab …`.

## Demo path

1. Type a multi-line thought in the compose field (Enter = newline).
2. Click **Add note** or press **⌘↩** / **Ctrl+Enter** — body is sealed as a `.tw` note; list shows a title from the first words.
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

## Dependencies

- [gpui](https://crates.io/crates/gpui) `=0.2.2` — Apache-2.0 (Zed). UI shell only.
- [tinkerway-vault](../crates/tinkerway-vault) — AES-256-GCM envelopes, OS keystore via `keyring` (`apple-native` + `linux-native-sync-persistent`), paths via `directories` (`rand`, `zeroize`). No GPUI.
- macOS only: `cocoa` / `objc` (same stack gpui already uses) to set the Dock icon from the brand PNG.
