---
name: pull-latest
description: >-
  Fast-forward the current git branch to its remote, then summarize what
  changed and any commands to run. Use when the user says "pull latest",
  "pull", "update the branch", or "git pull".
---

# Pull latest

Stay on the current branch. Do not switch branches. Do not discard local work.

1. Run `git status --short --branch`.
2. If the work tree has uncommitted changes, stop. Name the files. Do not stash, reset, or discard them.
3. Run `git pull --ff-only`.
4. If the pull refuses because the branches diverged, stop. Do not rebase, merge, or force-push.
5. Read `git log --oneline <old>..<new>` and the pull's file list. Report:
   - The old commit and the new commit.
   - A short summary of what changed, in plain language. Group files. Do not paste the full file list when it is long.
   - Commands the user should run, only when the diff shows they are needed. Examples: `mise install` when `mise.toml` changes, `cargo build` or `cargo test` when Rust crates or `Cargo.lock` change, a package install when a lockfile for that package manager changes. If no command is needed, say that.
