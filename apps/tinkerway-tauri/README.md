# tinkerway-tauri (explore / bake-off)

Parallel **Demo v1 capture** shell on [Tauri 2](https://v2.tauri.app/). Same encrypted vault as the GPUI app (`tinkerway-vault`). **Not** the shipping UI — see [PR #1](https://github.com/tinkerway/tinkerway/pull/1) for GPUI.

- Window ~1200×800: compose left, library right (420px), Send / New, read-only open, basic Shortcuts.
- Draft stays off the list until Send.
- Commands only; the webview never receives the master key.

Dep brief (Project store): `docs/tauri-deps-brief.md`. Bake-off scorecard: `docs/shell-bakeoff.md`.

## Run (macOS)

Same Keychain + vault path as GPUI (`ai.tinkerway.app` / `vault-master-key`, Application Support vault). Full Xcode is **not** required for Tauri the way Metal/GPUI needs it, but you need a normal Mac desktop WebView stack.

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

(`MBX_DISABLE=1 cargo run -p tinkerway-tauri` is enough for the spike.)

## Run (Linux)

Install WebKitGTK and related Tauri system deps (e.g. `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `patchelf`, `libdbus-1-dev`), plus the same keystore story as GPUI (`libsecret` / gnome-keyring or XDG `master-key` fallback). Then:

```bash
mise install
MBX_DISABLE=1 cargo run -p tinkerway-tauri
```

CI on Ubuntu still runs **`cargo test/check -p tinkerway-vault -p tinkerway` only** so the GPUI path stays green without WebKitGTK.

## Workspace note

This crate lives under `apps/` so it does not replace `app/` (GPUI binary `tinkerway`). Architecture “one shell” still means GPUI for ship; this folder is an intentional bake-off.
