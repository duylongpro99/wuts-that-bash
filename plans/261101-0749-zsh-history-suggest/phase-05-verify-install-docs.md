---
phase: 5
title: "Test tích hợp, benchmark, cài đặt, README"
status: pending
priority: P2
effort: 5h
dependencies: [4]
---

# Phase 5: Test tích hợp, benchmark, cài đặt, README

## Goal
Có bằng chứng tự động cho các acceptance criteria và hướng dẫn cài đặt hai bước.

## Files to Create / Modify
- Create: `tests/zle/run.zsh` (lái zsh interactive bằng `zmodload zsh/zpty`)
- Create: `scripts/bench.sh`
- Create: `README.md`

## Tasks & Steps
1. `tests/zle/run.zsh`: chạy `zsh -f` trong zpty với `WIB_ZSH_HISTORY` và `WIB_DATA_DIR` trỏ vào fixture, `eval "$(wib init zsh)"`, rồi gửi phím và kiểm tra:
   - 1 ký tự thì không có danh sách (AC7); `doc` thì có (AC1);
   - ↓ rồi Tab → buffer bằng lệnh được chọn, chưa chạy; ↓ rồi Enter → lệnh được chọn chạy (AC2);
   - ↓ rồi Space → buffer không đổi, danh sách vẫn hiện, trạng thái Typing (AC8);
   - Esc/Ctrl-G → danh sách tắt; gõ thêm 1 ký tự → hiện lại (AC9);
   - `gi` + ↓ chọn `git push origin master` + Shift+Tab ×4 → `git`, `git push`, `git push origin`, `git push origin master` (AC10);
   - Shift+Tab qua `git commit -m "fix bug"` → `"fix bug"` được điền trong một lần (AC11); `dcup` + Shift+Tab → `docker`;
   - chạy `echo wib-marker`, ở prompt sau gõ `ec` → thấy `echo wib-marker` (AC4);
   - lệnh ` secret` (có dấu cách đầu) không xuất hiện trong log.
2. Test AC3 ở mức binary: fixture có một lệnh 10 ngày trước; `--period 7d` không trả về, `--period 30d` có trả về.
3. `scripts/bench.sh`: build release, chạy `wib query` 500 lần với input ngẫu nhiên 2–5 ký tự trên bản copy của `~/.zsh_history` thật (copy vào thư mục tạm, không sửa file gốc), in p50/p95/max. Nếu p95 ≥ 10ms thì thêm cache theo mtime rồi đo lại.
4. README: cách cài (`cargo install --path .`, thêm `eval "$(wib init zsh)"`), biến cấu hình `WIB_PERIOD`, `WIB_LIMIT`, `WIB_MIN_CHARS`, bảng phím (kèm lý do không dùng được Cmd+Tab), ghi chú `KEYTIMEOUT=1` cho Esc, cách gỡ, nơi lưu dữ liệu.
5. Cài lên máy thật, dùng thử 1 ngày, ghi lại vấn đề gặp phải.

## Verification
- `cargo test && zsh tests/zle/run.zsh` pass.
- `scripts/bench.sh` báo p95 < 10ms; ghi số đo vào `plan.md`.
