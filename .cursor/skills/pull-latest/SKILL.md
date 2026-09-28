---
name: pull-latest
description: >-
  Update the current git branch from its remote. Fast-forward when possible,
  otherwise rebase unpushed local commits. Summarize what changed and any
  commands to run. Use when the user says "pull latest", "pull", "update the
  branch", or "git pull".
---

# Pull latest

Stay on the current branch. Do not switch branches. Do not discard local work.

1. Run `git status --short --branch`.
2. If the work tree has uncommitted changes, stop. Name the files. Do not stash, reset, or discard them.
3. Run `git pull --ff-only`.
4. If that pull refuses because the branches diverged, rebase the unpushed local commits onto the remote: `git pull --rebase`. A merge commit is the other way to join the two histories. Do not merge unless the user asks. Do not force-push. If the rebase stops on conflicts, stop and name the files.
5. Read `git log --oneline <old>..<new>` for the commits that came from the remote, and the pull's file list. Report:
   - The old commit and the new commit.
   - A short summary of what changed, in plain language. Group files. Do not paste the full file list when it is long.
   - Commands the user should run, only when the diff shows they are needed. Examples: `mise install` when `mise.toml` changes, `cargo build` or `cargo test` when Rust crates or `Cargo.lock` change, a package install when a lockfile for that package manager changes. If no command is needed, say that.
