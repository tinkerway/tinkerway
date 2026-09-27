# Architecture

Normative layout for the tinkerway desktop app.

## GPUI is UI only

[GPUI](https://gpui.rs) owns windows, views, Entities, and rendering. Domain logic, vault IO, paths, and crypto live in **plain Rust modules or crates** with **no GPUI types** in their public APIs. The shell calls those APIs (often via GPUI spawn/background), then updates UI state.

## No Rails-like native framework

There is no batteries-included “Rails for GPUI desktop.” Web stacks (Loco, Axum+ORM, etc.) are the wrong shape. Good defaults = **Cargo workspace discipline** plus small, approved crates — not an application framework.

## Workspace shape (DiskTree-style)

```text
crates/
  tinkerway-vault/   # encrypt, manifest, paths, migrate, Keychain — NO gpui
app/                 # binary — GPUI shell, Entities, calls vault APIs
apps/
  tinkerway-tauri/   # EXPLORE ONLY — Tauri 2 capture bake-off (not shipping)
```

Mirror [DiskTree](https://github.com/tobi/disktree): correctness for vault/domain must be testable **without** a display or GPU. Prefer modules inside the vault crate over premature micro-crates.

## Explore / bake-off (Tauri)

`apps/tinkerway-tauri` is an intentional parallel shell to compare capture UX against GPUI. It reuses `tinkerway-vault` only (commands; no master key in the webview). **Shipping shell remains GPUI** until Rinas says otherwise. Run notes: [`apps/tinkerway-tauri/README.md`](../apps/tinkerway-tauri/README.md).

## What not to do

- Put vault seal/open or master keys in long-lived GPUI Entity fields beyond the unlocked session handle.
- Structure the notes product as an HTTP/ORM “model layer.”
- Promote a second UI stack (Tauri, WebView, etc.) into the shipping path without an explicit decision — explore crates under `apps/` do not replace `app/`.
- Nest `entity.update` on an entity from inside that entity’s own update (see `.cursor/rules/gpui.mdc`).

## Run

How to build and run the Mac shell: [`app/README.md`](../app/README.md).
