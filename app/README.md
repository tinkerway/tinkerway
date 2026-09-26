# tinkerway app

First desktop slice: type a line, write a markdown file under `.tinkerway-workspace/`, see the list.

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

Notes are written next to your current working directory as `.tinkerway-workspace/*.md` (gitignored).

## Linux

`cargo check` / `cargo test` work with GPUI’s default `wayland` / `x11` features when system libraries are present (for example `libwayland-dev`, `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libxcb1-dev`). A full GUI run still needs a graphical session and GPU stack; day-to-day development of this slice is aimed at Mac + Xcode.

## Dependencies

- [gpui](https://crates.io/crates/gpui) `=0.2.2` — Apache-2.0 (Zed). Platform backends ship inside this crate for 0.2.x; a separate `gpui_platform` crate is not required on crates.io yet for this pin.
