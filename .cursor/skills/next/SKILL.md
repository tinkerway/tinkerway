---
name: next
description: Answer "what's next" for tinkerway from project notes and git state. Use when the user runs /next or asks what to do next, what's left, or what to work on.
disable-model-invocation: true
---

# Next

Give the next concrete step for this project. Do not start the work unless they ask.

## Sources

Read these before answering:

1. Project Context `notes.md` — open items under **Now** first, then **Housekeeping**
2. `git status -sb` and whether `main` is ahead of `origin/main`
3. Settled calls under **Product** only if they change the next step

Do not invent work. Do not reopen settled Product items.

## Answer shape

Keep it short:

1. The single next action (one sentence)
2. Why that is next (one short sentence, optional)
3. The step after that, only if useful

Lead with the result. No recap of the whole project. No menu of options unless two real choices block progress.

## Rules that still apply

- Do not edit the README "That said" list
- Push after commits by default (squash-merge PRs into main)
- Do not post to WIP
- Do not scaffold `core/` or `macos/` until they ask to start the app
