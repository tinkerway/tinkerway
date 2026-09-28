# Dev setup

Secrets stay out of git. Agents also have a short always-on reminder in [`.cursor/rules/no-secrets.mdc`](../.cursor/rules/no-secrets.mdc).

## Local

```bash
mise install
hk install --mise   # wires pre-commit via mise
```

`hk` runs **betterleaks** on staged files (`hk.pkl`). Optional full scan:

```bash
mise run secrets-scan
```

## CI

[`.github/workflows/secrets.yml`](../.github/workflows/secrets.yml) runs betterleaks on push to `main`, every PR, and daily.

[`.github/workflows/rust.yml`](../.github/workflows/rust.yml) installs the mise toolchain, restores Cargo outputs with [`Swatinem/rust-cache`](https://github.com/Swatinem/rust-cache) (same action jdx/mise uses on GitHub-hosted runners), then runs vault tests and a Tauri `cargo check`.

## Cursor agents

[`.cursor/hooks.json`](../.cursor/hooks.json) blocks `git commit` when staged secrets or sensitive paths appear (`failClosed`).
