---
title: "Cook: secret guard, templates, typo fix, doctor and unforget"
date: 2026-11-05
summary: "Implemented all six phases of the zsh UX plan; review caught detector gaps for quoted assignments and tool password variables, and the zle harness needed hang-proof reads"
---

# Cook: secret guard, templates, typo fix, doctor and unforget

## What happened
Implemented `plans/261105-0735-zsh-ux-depth/` on branch `feat/zsh-ux-depth`. Five new modules were added (`secret`, `audit`, `slots`, `fix`, `doctor`), the subcommands `audit`, `slots`, `suggest-fix`, `unforget` and `doctor`, and Ctrl-] plus the "did you mean" row in `shell/wib.zsh`. Results: `cargo test` 101 unit and 36 CLI tests pass, and the zle suite passes 80 of 80. Benchmark p95 for `query` stayed within 10% of the baseline; `suggest-fix` (12.9 ms) and `slots` (12.2 ms) are under 15 ms on a history of 500k commands over 400 days.

## Decisions made during implementation
- **`wib query` reads its input from `WIB_INPUT` (`--input-env`):** the acceptance criterion says no wib process may carry a secret in its arguments, but the plan had left query on argv. A pipe would cost about 0.75 ms per keystroke. Other users cannot read a process's environment the way they can read its arguments in `ps`.
- **The "did you mean" row is rendered in `line-init`:** zle draws the first prompt before `line-init` runs, so without this the correction appeared only after a keypress.
- **Esc and Ctrl-G drop a slot selection:** otherwise vi command mode started in visual mode on the slot.
- **`audit --fix` with `WIB_SECRET_GUARD=0` keeps the forget file:** removing a forgotten secret command would make it suggestible again from `.zsh_history`.

## What went wrong
- The independent review found the detector missing assignments inside quoted words (`docker run -e "API_KEY=…"`, `bash -c '…'`) and the password variables that tools read (`PGPASSWORD`, `MYSQL_PWD`, `SSHPASS`). It also flagged false positives such as `sk-` branch names and a lone `BEGIN PRIVATE KEY` header. All were fixed and covered by tests.
- The zle harness hung intermittently inside `zpty -r -t`, apparently waiting for a newline; master never did. Reads now take one character at a time with a pattern. The full suite takes about 4.5 minutes.
- An EXIT trap in the harness ran in a zpty-forked child and deleted the temp directory. It is now guarded by the real pid (`$sysparams[pid]`).

## Re-check on 2026-11-06
A second session re-ran every check on a fresh build. `cargo test` passed 101 unit and 36 CLI tests, and the zle suite passed 80 of 80. The two clippy warnings in the new code (`fix.rs`, `audit.rs`) were fixed. The earlier `query` numbers came from different history files, so `query` was benchmarked again with master and this branch on the same file. The branch's p95 was equal to or lower than master's. The numbers are in `reports/bench-after.txt`.

## Open
- `wib forget` / `wib unforget` still take the command as arguments, so a secret typed there is briefly visible in `ps`.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
