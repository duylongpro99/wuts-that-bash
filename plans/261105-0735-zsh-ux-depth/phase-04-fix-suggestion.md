---
phase: 4
title: "Gợi ý sửa lệnh gõ sai tên chương trình"
status: completed
priority: P2
effort: "4h"
dependencies: [1]
---

# Phase 4: Gợi ý sửa lệnh gõ sai

## Goal

Sau một lệnh "command not found" (exit 127), prompt kế tiếp hiện sẵn lệnh đã sửa tên chương trình dựa trên các
chương trình người dùng thật sự dùng; chọn bằng ↓ Enter như danh sách thường. Không bao giờ đổi đối số.

## Quyết định đã chốt

- Chỉ chạy khi exit 127. Lệnh lỗi vì lý do khác (`grep`, `make`, `git psuh` exit 1) không tạo process nào.
- Chỉ sửa **token đầu** (tên chương trình); các token còn lại giữ nguyên byte-for-byte. Vì vậy không thể biến
  `kubectl delete pod web-2` thành `web-1`.
- Tên chương trình hợp lệ: token đầu của lệnh có exit thành công trong log của wib, **hoặc** token đầu xuất hiện
  ≥ 3 lần trong `.zsh_history` trong period (để người mới cài vẫn có gợi ý).

## Files to Create / Modify

- Create: `src/fix.rs`
- Modify: `src/rank.rs` (tách phần gộp thống kê của `rank_by` thành `pub(crate) fn stats(entries, ctx)`; `rank_by`
  gọi lại, hành vi không đổi)
- Modify: `src/main.rs` (lệnh con nội bộ `suggest-fix`, đọc lệnh từ stdin; USAGE)
- Modify: `shell/wib.zsh` (`_wib_precmd`, `_wib_fetch`, `_wib_should_fetch`, `_wib_render`, `_wib_down`, `_wib_line_finish`)
- Modify: `scripts/bench.sh` (chế độ `suggest-fix` và `slots`)
- Modify: `tests/cli.rs`, `tests/zle/run.zsh`

Tên `suggest-fix` (không phải `fix`) để `wib fix` vẫn là tìm từ khoá ("fix bug").

## Thuật toán (`fix.rs`)

`wib suggest-fix --stdin --cwd <dir> [--period 30d]` in tối đa một lệnh.

1. Không gợi ý khi: guard bật và lệnh có secret, lệnh nhiều dòng, dài quá 300 ký tự, là lệnh `wib …`, hoặc token
   đầu đã là một tên chương trình hợp lệ.
2. Ứng viên: tên chương trình hợp lệ (xem trên), không thuộc lệnh đã quên.
3. Khoảng cách Damerau–Levenshtein theo char giữa token đầu và ứng viên ≤ 2 (≤ 1 nếu token ≤ 3 ký tự). Bỏ ứng
   viên chỉ khác ở chữ số (`python2` → `python3`).
4. Thứ tự: khoảng cách nhỏ hơn, rồi dùng trong dự án hiện tại, rồi frecency, rồi gần đây.
5. In token đầu đã thay + phần còn lại nguyên văn: `gti push origin feat-x` → `git push origin feat-x`.

## Hành vi trong zsh

- `_wib_precmd`: sau khi record nền, chỉ khi `$st == 127`:
  `_wib_fix=$(print -rn -- "$_wib_cmd" | _wib_run suggest-fix --stdin --cwd "$PWD" 2>/dev/null)`.
- Buffer rỗng và `_wib_fix` có giá trị: `_wib_fetch` đặt `_wib_items=( $_wib_fix )`; `_wib_render` thêm chú thích
  `  ← did you mean?` màu `fg=244`. Nhánh này bỏ qua kiểm tra `WIB_MIN_CHARS` và `LASTWIDGET` trong
  `_wib_should_fetch`. ↓ chọn, Enter chạy, Tab điền, Esc ẩn.
- Gõ bất kỳ ký tự nào: xoá `_wib_fix`. `_wib_line_finish` cũng xoá, nên gợi ý chỉ sống trong một prompt.

## Tasks & Steps

- [x] `rank::stats` + refactor `rank_by`; toàn bộ test hiện có trong `rank.rs` vẫn qua.
- [x] `fix.rs`: `damerau_levenshtein(a, b, max)` có cắt sớm; `suggest`. Test: `gti status` → `git status`;
       `dokcer ps` → `docker ps`; `sl` khi có `ls` → `ls`; `python2 x` khi chỉ có `python3` → không gợi ý; tên
       chương trình đã hợp lệ → không gợi ý; lệnh có secret → không; ứng viên chỉ thấy 2 lần trong
       `.zsh_history` và không có trong log → không; thấy 3 lần → có; ứng viên cùng dự án đứng trước.
- [x] `main.rs`: `Some("suggest-fix") => cmd_suggest_fix(..)`; `_wib_keywords` thêm `suggest-fix`.
- [x] `wib.zsh`: các thay đổi ở trên.
- [x] `scripts/bench.sh`: thêm chế độ `suggest-fix` và `slots`.
- [x] `tests/cli.rs`: sandbox có `history.tsv` với `echo hi` exit 0; `suggest-fix` cho `ehco hi` in `echo hi`.
- [x] `tests/zle/run.zsh`: chạy `echo hi` (thành công trong sandbox), rồi `ehco hi` (exit 127) → prompt mới hiện
       `echo hi` kèm chú thích; ↓ Enter chạy nó. Ca khác: gõ `x` → gợi ý biến mất. Ca khác: `grep nothing /dev/null`
       (exit 1) → không có gợi ý.

## Verification

- `cargo test fix rank` và `cargo test --test cli suggest_fix`
- `zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`
- `zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 100 suggest-fix`: p95 < 15 ms trên file 500k lệnh / 400 ngày.

## Rủi ro

- **Gợi ý sai:** không bao giờ tự chạy; chỉ đổi tên chương trình nên không thể đổi mục tiêu của lệnh.
- **Không bắt `git psuh`:** chấp nhận theo quyết định chỉ chạy với exit 127; git tự in gợi ý của nó.
