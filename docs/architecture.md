# Architecture

Normative layout for the tinkerway desktop app.

## GPUI is UI only

[GPUI](https://gpui.rs) owns windows, views, Entities, and rendering. Domain logic, vault IO, paths, and crypto live in **plain Rust modules or crates** with **no GPUI types** in their public APIs. The shell calls those APIs (often via GPUI spawn/background), then updates UI state.

## No Rails-like native framework

There is no batteries-included “Rails for GPUI desktop.” Web stacks (Loco, Axum+ORM, etc.) are the wrong shape. Good defaults = **Cargo workspace discipline** plus small, approved crates — not an application framework.

## Workspace shape (DiskTree-style)

Start small; grow only when a boundary hurts:

```text
crates/ (or modules today)
  core / vault   # encrypt, manifest, paths, migrate — NO gpui
app/             # binary — GPUI shell, Entities, calls core APIs
```

Mirror [DiskTree](https://github.com/tobi/disktree): correctness for vault/domain must be testable **without** a display or GPU. Prefer modules inside a vault/core crate over premature micro-crates.

## What not to do

- Put vault seal/open or master keys in long-lived GPUI Entity fields.
- Structure the notes product as an HTTP/ORM “model layer.”
- Add a second UI stack (Tauri, WebView, etc.) alongside GPUI.

## Run

How to build and run the Mac shell: [`app/README.md`](../app/README.md).
