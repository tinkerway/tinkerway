# tinkerway-tauri

**Demo v1 capture** on [Tauri 2](https://v2.tauri.app/): compose a private thought, Send it into the list, open View, New / Shortcuts. Same encrypted vault (`tinkerway-vault`); the master key stays in Rust and never reaches the webview.

This is the **shipping Demo v1 shell**. How to run (one obvious path):

```bash
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

- Window ~1200×800: compose left, library right (420px), mist card + Send pill, New, read-only View, Shortcuts page.
- Draft stays off the list until Send. Hold Cmd/Ctrl shows chords on Send / New.
- Commands only; the webview never receives the master key.

Architecture: [`docs/architecture.md`](../../docs/architecture.md). Privacy: [`docs/data-and-privacy.md`](../../docs/data-and-privacy.md).

## Run (macOS)

Keychain + vault under `ai.tinkerway.app` (`vault-master-key`, Application Support vault). Full Xcode / Metal is **not** required (unlike the parked GPUI experiment). You need a normal Mac desktop WebView stack.

```bash
mise install
# mr-boxington can drop Tauri ACL permission files from the cache — disable for this crate.
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

Optional (hot reload / CLI helpers):

```bash
cargo install tauri-cli --version "^2" --locked
MBX_DISABLE=1 cargo tauri dev --config apps/tinkerway-tauri/tauri.conf.json
```

## Run (Linux / Cursor cloud VM)

Install WebKitGTK and related system deps, plus the same keystore story (`libsecret` / gnome-keyring or XDG `master-key` fallback):

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev patchelf libdbus-1-dev libsecret-1-dev \
  pkg-config build-essential
```

Then:

```bash
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

CI on Ubuntu runs `cargo test -p tinkerway-vault` and `MBX_DISABLE=1 cargo check -p tinkerway-tauri` (WebKitGTK link deps installed; full GUI smoke is local / Mac).

## Demo path

1. Type in **What's new?** (Enter = newline). Draft stays off the list until Send.
2. **Send** or **⌘/Ctrl+Enter** — thought appears in the list; field clears.
3. Click a row → **View** (read-only). Click again or **New** / brand → compose.
4. **New** / **⌘N** with content saves then clears; empty New returns home.
5. Leave the field → **?** / Shortcuts foot → Shortcuts page; **Esc** / × / brand leave.

## Workspace note

- Ship crate: `apps/tinkerway-tauri` (this package).
- Vault: `crates/tinkerway-vault` (no UI types).
- Parked GPUI experiment: `app/` (`cargo run -p tinkerway`) — not the Demo v1 default.
