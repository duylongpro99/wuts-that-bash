---
title: "wib implemented: Rust binary + zsh ZLE widget"
date: 2026-11-01
summary: Implemented wib through phase 5 except real-terminal trial; review surfaced and fixed autosuggest/keymap bugs
---

# wib implemented: Rust binary + zsh ZLE widget

## What happened
- Implemented `wib` (Rust, std only) and `shell/wib.zsh`: list under the prompt after 2 chars, prefix-then-fuzzy matching, frecency, period filter, Tab / Shift+Tab word fill / Space / Enter / Esc as specified.
- The first version passed its own 22 zpty tests, but an independent review reproduced 2 critical bugs that a bare-zsh harness could not catch:
  - with zsh-autosuggestions, → pasted wib's whole POSTDISPLAY list into BUFFER (Enter could run `git push --force`);
  - ↓ after ↑ history recall selected a list row.
- It also reproduced breakage of Ctrl-R with a space, vi-mode Esc, HISTORY_IGNORE leaks, a 0644 log and a quadratic fuzzy matcher.

## Decisions
- `zle -M` does not display when called from `line-pre-redraw` (verified), so the list stays in POSTDISPLAY. wib only appends and strips its own tail, and wraps the autosuggest accept functions.
- Main keymap binds only ↓, Shift+Tab, Esc and Ctrl-G, each with fallback to the original widget. The other keys live in `wib-nav`, which is rebuilt from main when navigation starts. Its widgets hand the key back to main if navigation already ended, because isearch restores the keymap.
- The list is hidden after history widgets (`$LASTWIDGET`), not via HISTNO; Ctrl-U does not reset HISTNO.
- `zshaddhistory`-hook rejections cannot be detected: `${history[$HISTCMD]}` still holds rejected lines. Documented as a limitation.

## Evidence
- cargo test: 25 unit + 7 CLI. zpty: 38/38 (emacs, vi, autosuggestions, no-memo fallback).
- Benchmark: p95 5.8–8.6 ms on 10,072 real entries.

## Next steps
- Real-terminal trial (resize, iTerm2, powerlevel10k); then mark phase 5 done.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
