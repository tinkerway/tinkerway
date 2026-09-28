# Architecture

Normative layout for the tinkerway desktop app.

## Tauri is the Demo v1 UI shell

[Tauri 2](https://v2.tauri.app/) owns the window and system webview. Domain logic, vault IO, paths, and crypto live in **plain Rust crates** with **no webview or Tauri types** in their public APIs. The shell calls vault APIs through `#[tauri::command]` handlers and returns note DTOs only — **never** the master key.

## No Rails-like native framework

There is no batteries-included “Rails for desktop.” Web stacks (Loco, Axum+ORM, etc.) are the wrong shape for the vault. Good defaults = **Cargo workspace discipline** plus small, approved crates — not an application framework.

## Workspace shape (DiskTree-style)

```text
crates/
  tinkerway-vault/     # encrypt, manifest, paths, migrate, Keychain — NO UI
apps/
  tinkerway-tauri/     # Demo v1 ship binary — Tauri 2 + plain HTML/CSS/JS
app/                   # PARKED — former GPUI experiment (not Demo v1 default)
```

Mirror [DiskTree](https://github.com/tobi/disktree): correctness for vault/domain must be testable **without** a display or WebView. Prefer modules inside the vault crate over premature micro-crates.

Default workspace members are `apps/tinkerway-tauri` and `crates/tinkerway-vault`. The GPUI `app/` crate remains in `members` so it still builds when asked (`cargo run -p tinkerway`) but is excluded from `default-members`.

## What not to do

- Pass the master key, key hex, or raw key material through IPC / into the webview.
- Put vault seal/open in long-lived frontend state beyond unlocked session note DTOs.
- Structure the notes product as an HTTP/ORM “model layer.”
- Add another UI stack (React/SvelteKit, a second shell) without an explicit decision.
- Treat the parked GPUI `app/` crate as the shipping Demo v1 path.

## Run

How to build and run Demo v1: [`apps/tinkerway-tauri/README.md`](../apps/tinkerway-tauri/README.md).
