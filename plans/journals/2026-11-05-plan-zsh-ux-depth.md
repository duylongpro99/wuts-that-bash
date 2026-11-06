---
title: "Plan: secret guard, templates, typo fix, doctor and unforget"
date: 2026-11-05
summary: "Planned five zsh-user features from the terminal pain-point research; red-team review reshaped the secret handling and narrowed the typo fixer"
---

# Plan: secret guard, templates, typo fix, doctor and unforget

## What happened
Research into common terminal pain points (`plans/reports/research-261105-1429-terminal-pain-points-next-features.md`) led the user to prioritize deeper value for existing zsh users over bash/fish support. Created plan `plans/261105-0735-zsh-ux-depth/` (6 phases, about 27h). Two red-team reviewers found 16 issues, and all of them were applied.

## Decision
- **Secret guard:** commands with secrets are never recorded or suggested. `WIB_SECRET_GUARD=0` turns it off. Commands reach `record`, `suggest-fix` and `slots` over stdin, so secrets never appear in process arguments. Each shell gets its own notice file (`secret-notice.<pid>`).
- **`wib audit`:** masks every finding and keeps only public token prefixes. `--fix` edits only wib's own files, never `~/.zsh_history`.
- **Templates (Ctrl-]):** recomputed from the current buffer on each key press. Slots are positions that differ among earlier commands of the same shape. A command with no such earlier runs gets only a notice, never a guess.
- **Typo fix:** runs only after exit 127 and changes only the program name. That way a destructive command can never end up pointed at a different target.
- **File rewrites:** go through `rewrite_private`, which cannot be called without holding the lock.

## Next steps
Implement with `/ak-cook plans/261105-0735-zsh-ux-depth/plan.md` on branch `feat/zsh-ux-depth`.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
