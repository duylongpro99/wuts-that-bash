---
title: "Plan: one-step zsh setup and a visible not-loaded state"
date: 2026-11-17
summary: "Planned `wib setup zsh`, a stable binary path in `wib init zsh`, a stderr warning when the binary is gone, and a doctor check for terminals that Orca or an IDE started with an old environment"
---

# Plan: one-step zsh setup and a visible not-loaded state

## What happened
Users on some machines opened a new terminal in Orca or an IDE after installing wib and got no suggestions. `.zshrc` already had the init line, and `exec zsh` fixed it, so the host had started the shell from an environment it saved before the install. An `/ak-advise` session confirmed that no installer can reach a running shell or a host's saved environment. The goal therefore became "one command to install, and never fail silently". Created plan `plans/261117-1005-one-step-zsh-setup/` (4 phases, about 8h).

## Decision
- **`wib setup zsh`:** appends a block between `# >>> wib >>>` and `# <<< wib <<<` markers to `${ZDOTDIR:-$HOME}/.zshrc`, following symlinks. It adds an `export PATH` line when wib is not on `PATH`. It refuses (exit 2) when the file is not a regular file owned by the user. It never moves an existing init line. Afterwards it prints `exec zsh` and a restart hint for the host.
- **Stable path:** a check showed that on macOS, `current_exe()` already returns the `PATH` symlink rather than the Cellar path. The change is kept for Linux, where `/proc/self/exe` resolves the symlink (not verified).
- **Doctor:** when `.zshrc` has the init line but the shell has no wib state, doctor warns and gives a host-specific fix (VS Code/Cursor, JetBrains, or a generic message that covers Orca).
- **Not in scope:** loading wib into running shells or saved host environments, writing `.zshrc` from Homebrew `post_install`, and plugins for Orca or VS Code.

## Next steps
Run the diagnostic command on a failing Orca machine to find an environment variable that identifies Orca. Implement with `/ak-cook plans/261117-1005-one-step-zsh-setup/plan.md` on branch `feat/one-step-zsh-setup`.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
