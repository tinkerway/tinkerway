# Dev setup

Secrets stay out of git. Agents also have a short always-on reminder in [`.cursor/rules/no-secrets.mdc`](../.cursor/rules/no-secrets.mdc).

## Local

`mise.toml` needs mise **2026.9.2** or newer. Older mise does not ship `betterleaks` or `mr-boxington`, and `mise install` fails with "not found in mise tool registry". Update with `mise self-update -y`.

First time, and again after a pull when tools change:

```bash
mise setup
```

Hooks only:

```bash
mise install
hk install --mise   # wires pre-commit via mise
```

`hk` runs **betterleaks** on staged files (`hk.pkl`). Optional full scan:

```bash
mise run secrets-scan
```

## Mr Boxington (mbx)

`mise.toml` sets `mr_boxington` on Rust, so `cargo` from an activated mise shell goes through mbx's build cache.

That includes ordinary Cargo in this repo:

- `cargo test -p tinkerway-vault` locally, and the same step in [`.github/workflows/rust.yml`](../.github/workflows/rust.yml) (no `MBX_DISABLE`)
- the parked GPUI shell, `cargo run -p tinkerway` / `cargo check -p tinkerway`

The Tauri app turns the cache off. mbx can restore a compiled crate and skip Tauri's build script, which writes the ACL permission files the webview is allowed to call. `mise tauri` sets `MBX_DISABLE=1` for that run. CI's `cargo check -p tinkerway-tauri` does the same. Rust still comes from mise.

## CI

[`.github/workflows/secrets.yml`](../.github/workflows/secrets.yml) runs betterleaks on push to `main`, every PR, and daily.

## Cursor agents

[`.cursor/hooks.json`](../.cursor/hooks.json) blocks `git commit` when staged secrets or sensitive paths appear (`failClosed`).
