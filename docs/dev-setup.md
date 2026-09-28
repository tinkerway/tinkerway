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

## CI

[`.github/workflows/secrets.yml`](../.github/workflows/secrets.yml) runs betterleaks on push to `main`, every PR, and daily.

## Cursor agents

[`.cursor/hooks.json`](../.cursor/hooks.json) blocks `git commit` when staged secrets or sensitive paths appear (`failClosed`).
