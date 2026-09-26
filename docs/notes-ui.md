# Notes UI

Normative interaction model for the Demo v1 notes surface.

## Flows

1. **Multi-line compose → note** — Type in the compose field (Enter inserts a newline). **Add note** or **⌘↩ / Ctrl+Enter** creates a private `.tw` note. Title = first words of the body.
2. **List → open** — Selecting a list row loads that note into the body editor (title shown in the list).
3. **Edit markdown body → autosave** — Edits debounce (~400ms) then seal back to the vault. Status line confirms autosave.

## One source of structure

Structure and metadata (ids, titles used by the app, vault index fields) must not be **double-owned** by freeform markdown **and** UI widgets unless there is an explicit validation path that keeps them consistent.

- The vault owns note ids and ciphertext; title is derived from the body on write.
- List titles come from decrypting envelopes after unlock — not from plaintext filenames.

Do not let the UI and a hand-edited export `.md` silently diverge on the same facts (export is a separate pipe).
