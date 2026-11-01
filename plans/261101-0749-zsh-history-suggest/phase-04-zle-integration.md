---
phase: 4
title: "Tích hợp ZLE + wib init zsh"
status: pending
priority: P1
effort: 8h
dependencies: [1, 3]
---

# Phase 4: Tích hợp ZLE + `wib init zsh`

## Goal
`eval "$(wib init zsh)"` bật đủ tính năng: danh sách tự hiện, chọn/điền/đóng, ghi lại lệnh vừa chạy. Cách vẽ theo kết luận của spike ở phase 1.

## Files to Create / Modify
- Create: `shell/wib.zsh` (nhúng vào binary bằng `include_str!`)
- Modify: `src/main.rs` (subcommand `init zsh` in script ra stdout)
- Delete: `spike/` sau khi port xong

## Tasks & Steps
1. Port widget từ spike, thay dữ liệu giả bằng `wib query --period "${WIB_PERIOD:-30d}" --limit "${WIB_LIMIT:-8}" -- "$BUFFER"`; ngưỡng `${WIB_MIN_CHARS:-2}`.
2. Chỉ gọi binary khi `BUFFER` khác lần trước (nhớ trong `_wib_last_buffer`). Không gọi khi đang ở chế độ chọn, khi `CURSOR != ${#BUFFER}`, hoặc khi menu completion của zsh đang mở.
3. Unescape `\n` và `\\` khi điền BUFFER, theo hợp đồng ở phase 3.
4. Ghi lệnh: `preexec` lưu `$1` vào `_wib_cmd`; `precmd` lấy `$?` ngay dòng đầu rồi gọi `wib record --ts $EPOCHSECONDS --exit $status --cwd $PWD -- "$_wib_cmd" &!` để chạy nền, không chặn prompt. Cần `zmodload zsh/datetime`.
5. Mọi biến và hàm có tiền tố `_wib_`. Dùng `add-zle-hook-widget` và `add-zsh-hook` để không đè hook của plugin khác. Lưu widget gốc của từng phím trước khi bind để gọi lại khi menu đóng.
6. Hiện thực đủ **Bảng phím** trong `plan.md` với biến trạng thái `_wib_state` (typing / navigating / hidden) và `_wib_sel`:
   - Tab (Navigating): `BUFFER=<lệnh chọn>`, về Typing.
   - Enter (Navigating): `BUFFER=<lệnh chọn>` rồi `zle .accept-line`.
   - Space (Navigating): về Typing, không chèn ký tự, không gọi lại `wib query`.
   - Esc/Ctrl-G: sang Hidden; hook redraw thấy BUFFER khác `_wib_last_buffer` thì về Typing.
7. Shift+Tab điền từng từ, hàm `_wib_next_word`:
   - Đích = dòng đang chọn, hoặc dòng đầu nếu đang Typing. Lưu vào `_wib_word_target`; giữ đích khi lần bấm trước cũng là Shift+Tab và BUFFER vẫn là tiền tố của đích.
   - Tách `${(z)target}` thành token, rồi dò lại trong chuỗi gốc để lấy vị trí kết thúc của từng token (giữ khoảng trắng gốc).
   - BUFFER là tiền tố của đích: điền tới hết token đầu tiên kết thúc sau `${#BUFFER}`. BUFFER không là tiền tố: thay bằng token đầu. BUFFER bằng đích: không làm gì.
8. Chỉ kích hoạt trong shell interactive. Nếu `wib` không có trong PATH thì tắt êm, không báo lỗi mỗi phím.

## Verification
- Kiểm tra thủ công AC1–AC4 và AC6–AC11 trong terminal thật.
- Ctrl-R, ↑ khi menu đóng và Tab completion khi menu đóng không đổi.
- `zsh -n shell/wib.zsh` sạch.
