---
title: "One-step zsh setup and a visible not-loaded state"
date: 2026-11-17
summary: "Implemented `wib setup zsh`, a PATH-based binary path in `wib init zsh`, a stderr warning for a missing binary, and doctor advice for shells that have not loaded wib; review found and fixed PATH shadowing, a false doctor diagnosis and unexported ZDOTDIR"
---

# One-step zsh setup and a visible not-loaded state

## What happened
Implemented plan `plans/261117-1005-one-step-zsh-setup/` on branch `feat/one-step-zsh-setup`. New module `src/setup.rs` holds `wib setup zsh` and the helpers that `wib init zsh` and `wib doctor` share. Docs (README, Homebrew caveats, site, changelog) now point to `wib setup zsh`.

## Review findings that changed the code
- Adding the binary's directory to `PATH` whenever *this* binary was not on it meant that `cargo run -- setup zsh`, or running a downloaded binary before moving it, put a build or Downloads folder ahead of an installed `wib` in every future shell. Setup now adds the `PATH` line only when no `wib` is on `PATH`, and otherwise says which one new shells will load.
- The plan's doctor wording ("this shell has not loaded it") was false for `command wib doctor` in a loaded shell, because the shell state reaches doctor only through the `wib` function. The message now names both causes.
- `ZDOTDIR` is often set in `~/.zshenv` without `export`, so setup and doctor read the wrong `.zshrc`. Both now ask `zsh -c` for `ZDOTDIR` when the environment lacks it.
- A missing `.zshrc` is created only when its directory belongs to the current user, so `sudo wib setup zsh` cannot leave a root-owned file. Refusal messages now include the `PATH` line.

## Gotchas
- `scripts/homebrew-formula.sh` builds the formula in an unquoted shell heredoc, so backticks in caveats must be escaped.
- `return` inside `eval` in `.zshrc` ends the rest of `.zshrc`. This behaviour predates the change and matters if the init line is not last.

## Open
- No environment variable is known to identify Orca, so it gets the generic hint.
- The Linux `current_exe()` symlink behaviour and Cursor's `TERM_PROGRAM=vscode` are still unverified. The site pages have not been checked in a browser.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
