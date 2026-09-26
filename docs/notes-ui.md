# Notes UI

Normative interaction model for the first notes surface.

## Flows

1. **Type a line → note** — Enter on the line input creates a note from that text.
2. **List → open body** — Selecting a list row loads and shows that note’s body in the UI.
3. **Markdown view** — First cut is **read-only** rendering of the body. An editable body is fine later, with validation before write-back.

## One source of structure

Structure and metadata (ids, titles used by the app, vault index fields) must not be **double-owned** by freeform markdown **and** UI widgets unless there is an explicit validation path that keeps them consistent.

- Either the vault/model owns structured fields and markdown is body/export content, **or**
- Edits go through a validator that reconciles widgets ↔ markdown before persist.

Do not let the UI and a hand-edited `.md` silently diverge on the same facts.
