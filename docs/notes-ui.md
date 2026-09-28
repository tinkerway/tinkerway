# Capture

The writing field is the live thought. Sent captures stack above like a quiet messaging feed. You cannot change an older line. This is capture, not a notes editor, and not a chat with replies.

Demo v1 is **just capture** — no folders, tags, or auto-organization.

## Flows

1. **Write** — Type in the field at the bottom. **Shift+Enter** inserts a newline; the field grows with the text. Nothing is sealed until you **Send**.
2. **Send** — **Enter** or the arrow button seals the thought into the vault, puts it in the feed above, and clears the field.
3. **Read** — Click a row. Long captures are clamped to about three lines with an ellipsis; the full text opens in a modal titled with the item's timestamp (same quiet labels as the feed: Now, Today, Yesterday, weekday). **Esc** or the close control dismisses it.
4. **Scroll** — The feed fills the space above compose and scrolls when there are many items. Compose stays pinned at the bottom.

## One source of structure

Structure and metadata (ids, titles used by the app, vault index fields) must not be **double-owned** by freeform text **and** UI widgets unless there is an explicit validation path that keeps them consistent.

- The vault owns ids and ciphertext; the title is derived from the first words on write.
- Feed previews come from decrypting envelopes after unlock — not from plaintext filenames.

Do not let the UI and a hand-edited export silently diverge on the same facts (export is a separate pipe).
