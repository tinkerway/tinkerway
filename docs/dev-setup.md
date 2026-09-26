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

## Cursor agents

[`.cursor/hooks.json`](../.cursor/hooks.json) blocks `git commit` when staged secrets or sensitive paths appear (`failClosed`).
