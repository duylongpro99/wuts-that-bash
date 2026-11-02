---
title: "Plan: context-aware ranking and wib forget"
date: 2026-11-02
summary: "Planned project-scoped ranking, failed-command demotion and wib forget using cwd/exit data wib already records"
---

# Plan: context-aware ranking and wib forget

## What happened
A feature review found that `wib record` already stores cwd and exit code in `history.tsv`, but `rank_by` in `src/rank.rs` ignores both. Created plan `plans/261102-0911-context-aware-ranking/` (4 phases, about 6.5h).

## Decision
- Commands used anywhere in the current git project rank before others when match quality is equal. Project root is found by walking up for `.git` in Rust (no `git` process). A `.git` at `$HOME` or `/` is ignored, and outside git the exact `$PWD` is used.
- A command is "failed" when it has a known exit code and never succeeded. Codes 0, 130, 141, 143 and 148 count as success. Failed commands are pushed down, never hidden.
- Sort order: tier, score, failed, local, frecency, last_ts.
- `wib forget <cmd>` adds an exact command to `<data_dir>/forget` (0600). `wib forget ...` invocations are also filtered from suggestions.
- `wib query` without `--cwd` behaves exactly as before.

## Next steps
Implement with `/ak-cook plans/261102-0911-context-aware-ranking/plan.md` on branch `feat/context-aware-ranking`.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
