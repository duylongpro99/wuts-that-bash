---
title: "wib: tìm lệnh theo từ khoá đã xong"
date: 2026-11-01
summary: "wib pull,master tìm lệnh cũ theo từ khoá ở live list và CLI; mọi test qua, p95 tương đương chế độ thường"
---

# wib: tìm lệnh theo từ khoá đã xong

## What happened
Hoàn thành plan `plans/261101-1736-keyword-search/` trên nhánh `feat/keyword-search` (chưa commit).
- `src/matcher.rs`: `KeywordPattern` (tách theo `,`/khoảng trắng, AND, thứ tự bất kỳ; trọn từ 3, đầu từ 2, giữa từ 1).
- `src/rank.rs`: tách `rank_by` dùng chung; `rank_keywords` loại các lệnh `wib ...` cũ.
- `src/main.rs`: `wib <kw>`, `wib -- <kw>`, `wib query --keywords`; cờ lạ bắt đầu bằng `-` vẫn báo lỗi.
- `shell/wib.zsh`: buffer `wib <kw>` gọi chế độ từ khoá; `WIB_MIN_CHARS` tính trên phần từ khoá.

## Decision
`wib bogus` không còn là lỗi mà là tìm từ khoá; test CLI cũ đổi sang `wib --bogus`.
Dạng ngắn đọc `WIB_PERIOD`/`WIB_LIMIT`/`WIB_ZSH_HISTORY` từ env; README hướng dẫn `export`.

## Evidence
cargo test 40 + 14, clippy sạch, zle 47/47. Bench 100k lệnh/400 ngày: p95 từ khoá 10,1 và 7,3 ms so với 7,9 và 7,6 ms.

## Next steps
Commit và mở PR khi người dùng yêu cầu.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
