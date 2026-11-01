---
phase: 1
title: "Spike vẽ danh sách trong ZLE"
status: pending
priority: P1
effort: 3h
dependencies: []
---

# Phase 1: Spike vẽ danh sách trong ZLE

## Goal
Chứng minh bằng zsh thuần với dữ liệu giả rằng có thể hiện danh sách dưới prompt, cập nhật sau mỗi phím, chọn/điền/đóng được, không nháy và không phá các phím sẵn có.

## Files to Create / Modify
- Create: `spike/render-spike.zsh` (xoá sau phase 4, không ship)

## Tasks & Steps
1. Hook `line-pre-redraw` qua `add-zle-hook-widget`. Khi `${#BUFFER} >= 2`, lọc một mảng 20 lệnh giả bằng `[[ $cmd == $BUFFER* ]]` rồi gán `POSTDISPLAY=$'\n'"${(F)matches}"`.
2. Highlight dòng đang chọn bằng `region_highlight` với style `standout`. Offset tính từ `${#BUFFER}` cộng vị trí dòng trong POSTDISPLAY.
3. Dựng máy trạng thái Typing / Navigating / Hidden và widget bọc cho ↓ (`^[[B`, `^[OB`), ↑, Tab, Shift+Tab (`^[[Z`), Space, Enter, Esc, Ctrl-G theo **Bảng phím** trong `plan.md`:
   - khi phím không thuộc trạng thái hiện tại thì gọi widget gốc đã lưu (`down-line-or-history`, `up-line-or-history`, `expand-or-complete`, `accept-line`, `self-insert`/`magic-space`, …);
   - điền `BUFFER` xong thì đặt `CURSOR=$#BUFFER`.
   Shift+Tab ở spike chỉ cần tách theo khoảng trắng; tách token đúng làm ở phase 4.
4. Xoá POSTDISPLAY trong `zle-line-finish` để danh sách không còn trên màn hình sau khi chạy lệnh.
5. Thử thủ công trong Terminal.app và iTerm2: gõ nhanh, xoá lùi, resize cửa sổ, prompt nhiều dòng, buffer nhiều dòng, có oh-my-zsh. Kiểm tra Shift+Tab và ↓ vẫn nhận đúng khi `KEYTIMEOUT=1`, và Esc phản hồi đủ nhanh.
6. Nếu POSTDISPLAY có vấn đề, thử `zle -M` và ghi lại so sánh.

## Verification
- Các mục ở bước 5 đều đạt. Ghi kết luận (chọn POSTDISPLAY hay `zle -M`, các vấn đề gặp phải) vào cuối file này.
- Ctrl-R, ↑ khi menu đóng và Tab completion khi menu đóng vẫn như cũ.

## Exit
Nếu cả hai cách đều không chấp nhận được thì dừng và quay lại brainstorm. Phương án thay thế: mở danh sách bằng phím tắt thay vì tự hiện.
