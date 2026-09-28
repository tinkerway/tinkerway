# tinkerway-tauri

**Demo v1 capture** on [Tauri 2](https://v2.tauri.app/): compose a private thought, Send it into the list, open View, New / Shortcuts. Same encrypted vault (`tinkerway-vault`); the master key stays in Rust and never reaches the webview.

This is the **shipping Demo v1 shell**. There is no Node app, so the dev command is Cargo, not `npm run tauri dev`.

First time (mise, Rust, and on macOS a linker-compatible SDK):

```bash
mise setup
```

After that, from the repo root:

```bash
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

`MBX_DISABLE=1` turns Mr Boxington’s Cargo cache off for this run. That cache can drop Tauri’s ACL permission files. Rust still comes from mise. You still need [mise](https://mise.jdx.dev) on `PATH`. `mise setup` is safe to re-run after a pull; it does not open the window.

- Window ~1200×800: compose left, library right (420px), mist card + Send pill, New, read-only View, Shortcuts page.
- Draft stays off the list until Send. Hold Cmd/Ctrl shows chords on Send / New.
- Commands only; the webview never receives the master key.

Architecture: [`docs/architecture.md`](../../docs/architecture.md). Privacy: [`docs/data-and-privacy.md`](../../docs/data-and-privacy.md).

## Run (macOS)

Same Keychain + vault path as before (`ai.tinkerway.app` / `vault-master-key`, Application Support vault). Full Xcode / Metal is **not** required (unlike the parked GPUI experiment). You need a normal Mac desktop WebView stack.

```bash
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

`mise setup` writes a gitignored `.cargo/config.toml` when the active SDK uses `arm64e.x1` stubs this linker cannot read, so later Cargo builds pick that SDK up. If you build before setup and linking fails with `unknown architecture`, point `SDKROOT` at an older SDK whose `usr/lib/libSystem.tbd` does not mention `arm64e.x1`:

```bash
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk MBX_DISABLE=1 cargo run -p tinkerway-tauri
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

Then `mise setup` once, and `MBX_DISABLE=1 cargo run -p tinkerway-tauri` to open the window.

CI on Ubuntu runs `cargo test -p tinkerway-vault` and `MBX_DISABLE=1 cargo check -p tinkerway-tauri` (WebKitGTK link deps installed; full GUI smoke is local / Mac).

### By hand

After mise is **2026.9.2** or newer (`mise self-update -y` when it is older):

```bash
mise trust
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

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
