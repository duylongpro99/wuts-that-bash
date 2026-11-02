---
title: "wib: xếp hạng theo dự án, đẩy lệnh lỗi xuống và wib forget đã xong"
date: 2026-11-02
summary: "Lệnh trong dự án git hiện tại đứng trước, lệnh chưa từng thành công xếp sau, wib forget gỡ một gợi ý; mọi test qua, --cwd không làm chậm"
---

# wib: xếp hạng theo dự án, lệnh lỗi và `wib forget` đã xong

## What happened
Hoàn thành plan `plans/261102-0911-context-aware-ranking/` trên nhánh `feat/context-aware-ranking`.
- `src/project.rs`: `scope` (gốc `.git` gần nhất, bỏ qua `$HOME` và `/`), `in_scope` theo ranh giới thư mục.
- `src/rank.rs`: `RankContext { scope, forgotten }`; khoá sort tier, score, failed, local, frecency, last_ts;
  mã 0/130/141/143/148 không tính là lỗi; `wib forget ...` không bao giờ được gợi ý.
- `src/log_store.rs`: file `forget` (0600, escape từng dòng, không ghi trùng, dưới lock chung).
- `src/main.rs`: `wib forget`, `wib query --cwd`, `wib <kw>` đọc `$PWD`.
- `shell/wib.zsh`: truyền `--cwd "$PWD"`; `wib forget|query|record|init ...` không vào chế độ từ khoá.
- `scripts/bench.sh`: mặc định truyền `--cwd`, `WIB_BENCH_CWD=0` để so sánh.

## Decision
Test lệnh lỗi dùng input `g` thay cho `gi` như plan: matcher không coi `gi` là khớp với `gti status`.
Test quên lệnh nhiều dòng dùng `printf a\nb`: `.zsh_history` lưu xuống dòng bằng dấu `\` cuối dòng.

## Evidence
cargo test 52 + 18, clippy sạch, zle 51/51. Bench 100k lệnh/400 ngày, 200 lần tra, xen kẽ:
p95 có `--cwd` 8,10 và 7,58 ms; không có 7,77 và 8,32 ms. Với log 7.500 lệnh đều có cwd: 9,66/8,41 so với
9,02/8,88 ms mỗi lần tra. Đường dẫn không tồn tại dưới `/home` (autofs trên macOS) mất ~50 ms vì `stat`,
không phải do xếp hạng; `$PWD` thật luôn tồn tại.

## Next steps
Push và mở PR khi người dùng yêu cầu.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
