# tinkerway-tauri

**Demo v1 capture** on [Tauri 2](https://v2.tauri.app/): compose a private thought, Send it into the list, open View, New / Shortcuts. Same encrypted vault (`tinkerway-vault`). Ship adoption of this shell lives on [PR #1](https://github.com/tinkerway/tinkerway/pull/1).

- Window ~1200×800: compose left, library right (420px), mist card + Send pill, New, read-only View, Shortcuts page.
- Draft stays off the list until Send. Hold Cmd/Ctrl shows chords on Send / New.
- Commands only; the webview never receives the master key.

Dep brief (Project store): `docs/tauri-deps-brief.md`. Bake-off scorecard (historical compare): `docs/shell-bakeoff.md`.

## Run (macOS)

Same Keychain + vault path as the desktop app (`ai.tinkerway.app` / `vault-master-key`, Application Support vault). Full Xcode is **not** required for Tauri the way Metal/GPUI needs it, but you need a normal Mac desktop WebView stack.

From the repo root:

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

(`MBX_DISABLE=1 cargo run -p tinkerway-tauri` is enough to try the capture path.)

## Run (Linux)

Install WebKitGTK and related Tauri system deps (e.g. `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, `libdbus-1-dev`), plus the same keystore story (`libsecret` / gnome-keyring or XDG `master-key` fallback). Then:

```bash
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

CI on Ubuntu still runs **`cargo test/check -p tinkerway-vault -p tinkerway` only** so the GPUI binary stays green without WebKitGTK until ship CI is updated on #1.

## Workspace note

This crate lives under `apps/` beside `app/` (GPUI). Layout may change when Tauri becomes the ship shell on #1.
