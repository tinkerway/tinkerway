# tinkerway app

Capture a line as a note: type → save → see it in the list → click to read the body.

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

## Demo path

1. Type a thought in the line field.
2. Press Enter (or click **Add**) — it saves as a markdown note.
3. Find it in the **Notes** list.
4. Click the row to read the body in the pane below.

Notes are written next to your current working directory as `.tinkerway-workspace/*.md` (gitignored). That folder is **plaintext** today (any same-user tool can read it). An encrypted vault (ciphertext at rest, Keychain master key) is planned next — no vault/crypto crates in this slice.

## Linux

`cargo check` / `cargo test` work with GPUI’s default `wayland` / `x11` features when system libraries are present (for example `libwayland-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libxcb1-dev`). A full GUI run still needs a graphical session and GPU stack; day-to-day development of this slice is aimed at Mac + Xcode.

## Dependencies

- [gpui](https://crates.io/crates/gpui) `=0.2.2` — Apache-2.0 (Zed). UI shell only; note IO is plain Rust. Platform backends ship inside this crate for 0.2.x.
