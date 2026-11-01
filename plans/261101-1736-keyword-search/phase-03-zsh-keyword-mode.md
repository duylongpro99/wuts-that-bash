---
phase: 3
title: "Danh sách live khi gõ wib "
status: completed
priority: P2
effort: 2h
dependencies: [2]
---

# Phase 3: Danh sách live trong chế độ từ khoá

## Goal
Gõ `wib pul` tại prompt hiện danh sách lệnh khớp từ khoá; chọn một dòng thì lệnh đó thay cả buffer.

## Files to Modify
- `shell/wib.zsh`: `_wib_should_fetch`, `_wib_fetch`.
- `tests/zle/run.zsh`: thêm lệnh vào history giả và các check mới.

## Tasks & Steps
1. `_wib_fetch`: nếu `BUFFER` khớp `wib[[:blank:]]##?*` thì lấy phần sau `wib` và khoảng trắng làm từ khoá,
   gọi `"$_wib_bin" query -0 --keywords --period … --limit … -- "$kw"`; ngược lại giữ lời gọi cũ.
2. `_wib_should_fetch`: ngưỡng `WIB_MIN_CHARS` áp lên phần từ khoá trong chế độ này (gõ `wib ` thôi thì chưa hiện),
   các điều kiện khác (con trỏ ở cuối, không trong history/search, không nhiều dòng) giữ nguyên.
3. Không sửa `_wib_fill`, `_wib_nav_enter`, `_wib_nav_tab`, `_wib_word`: chúng đã thay cả buffer. Xác nhận bằng test.

## Tests (`tests/zle/run.zsh`)
Thêm `git pull origin master` và `docker pull nginx` vào history giả (chỉnh số đếm của các check cũ nếu đổi,
ví dụ `gi` từ 3 thành 4, và ghi lý do trong commit).
- `wib p` → không có danh sách (dưới `WIB_MIN_CHARS`).
- `wib pull` → 2 dòng; `wib pull,master` → 1 dòng; `wib master pull` → 1 dòng.
- `wib pull,master` + ↓ + Tab → buffer `git pull origin master`.
- `wib pull,master` + ↓ + Enter → lệnh chạy là `git pull origin master` (kiểm tra qua log `history.tsv`).
- `wib pull,master` + Enter (không chọn) → in `git pull origin master` ra terminal.
- `wib pull,master` + Shift+Tab → buffer `git`.
- Check cũ cho chế độ thường vẫn qua.

## Verification
`cargo install --path . --root .bin && zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`; thử tay trong một terminal
có oh-my-zsh và zsh-autosuggestions.

## Risk
- Người dùng có lệnh thật tên `wib` khác (alias, script) sẽ thấy danh sách từ khoá khi gõ nó. Không xử lý; đây là
  tên của chính tool.

## Rollback
Trả `_wib_fetch` và `_wib_should_fetch` về bản cũ.
