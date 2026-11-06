---
title: "Release v0.2.0 and Homebrew tap"
date: 2026-11-06
summary: "Merged the zsh UX work, released v0.2.0, added a Homebrew tap updated by the release workflow, and documented upgrading"
---

# Release v0.2.0 and Homebrew tap

## What happened
- PR #5 (secret guard, `wib audit`, Ctrl-] templates, did-you-mean, `wib unforget`, `wib doctor`) was merged into `master`. The PR was rebase-merged, so the commit IDs on `master` differ from the branch.
- README and the landing page now present the new features; the site gained a "What's new" section and a Ctrl-] key row.
- v0.2.0 was released: `Cargo.toml` was bumped, download links in README, site and `docs/deploy-cloudflare-pages.md` point at v0.2.0, and the release workflow built all four targets. The Apple Silicon archive was downloaded, its checksum verified, and `wib --version` printed `wib 0.2.0`. Highlights and a behavior-change note were added to the release notes.
- Created `duylongpro99/homebrew-tap` (`brew install duylongpro99/tap/wib`). `scripts/homebrew-formula.sh` renders the formula from a release's `.sha256` files, and a new `homebrew` job in `release.yml` pushes it with the `HOMEBREW_TAP_DEPLOY_KEY` deploy key (write access to the tap only).
- README gained a Homebrew install line and an "Upgrading" section; the site gained matching notes.

## Decisions
- **No `wib update` command:** wib never uses the network, and the site promises "0 bytes sent anywhere". Upgrades go through Homebrew, a new download, or cargo.
- **v0.2.0, not v0.1.1:** `audit`, `doctor`, `slots`, `suggest-fix` and `unforget` became subcommands, so keyword searches for those words now need `wib -- <word>`.

## What went wrong
- `brew audit` flagged the formula's explicit `version` as redundant with the URL; it was removed.
- Uninstalling the test install with `brew uninstall` autoremoved four unrelated dependency formulae on the maintainer's Mac (python@3.14, openssl@3, libnghttp3, libngtcp2). They were reinstalled and marked as dependencies again; libnghttp3 and libngtcp2 came back one minor version newer. Use `HOMEBREW_NO_AUTOREMOVE=1` for such checks.

## Open
- The `homebrew` job has not run yet; check it on the next release. If it fails, the GitHub Release is still complete and the job can be re-run without a new tag.
- `wib forget` / `wib unforget` still take the command as arguments, so a secret typed there is briefly visible in `ps`.
- The remote branch `feat/zsh-ux-depth` still exists.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
