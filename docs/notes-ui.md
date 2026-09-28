# Capture

The writing field is the live thought. An older line opens so you can read it. You cannot change an older line. This is capture, not a notes editor.

## Flows

1. **Write** — Type in the field (Enter inserts a newline). The draft stays off the list until you **Send**. An empty field stays out of the list.
2. **New** — **New** at the top right, or **Command-N**, returns to the writing field. **Send** or **Command-Enter** puts the words in the list and clears the field.
3. **Read** — Click an older line. The words open in View (read-only). You cannot change them. Click that line again to return to the writing field.
4. **Shortcuts** — Click **Shortcuts** at the bottom. In the writing field, **?** types a question mark. Press **Escape** to leave the field, then press **?** to open Shortcuts. Hold Command to see the shortcut keys on the screen you are already on.

## One source of structure

Structure and metadata (ids, titles used by the app, vault index fields) must not be **double-owned** by freeform text **and** UI widgets unless there is an explicit validation path that keeps them consistent.

- The vault owns ids and ciphertext; the title is derived from the first words on write.
- List titles come from decrypting envelopes after unlock — not from plaintext filenames.

Do not let the UI and a hand-edited export silently diverge on the same facts (export is a separate pipe).
