---
phase: 3
title: "Template: nhảy giữa các phần hay đổi"
status: completed
priority: P1
effort: "6h"
dependencies: [1]
---

# Phase 3: Template — nhảy giữa các phần hay đổi

## Goal

Dùng một lệnh cũ làm khuôn: Ctrl-] đưa con trỏ tới phần mà các lần chạy trước của lệnh đó từng khác nhau; gõ
ngay thì thay cả phần đó.

## Files to Create / Modify

- Create: `src/slots.rs`
- Modify: `src/main.rs` (lệnh con nội bộ `slots`, đọc buffer từ stdin; USAGE)
- Modify: `src/rank.rs` (dùng chung bộ lọc forgotten + secret với `rank_by`, không lặp logic)
- Modify: `shell/wib.zsh` (widget `_wib_slot`, `_wib_nav_slot`, "gõ để thay" trong `_wib_pre_redraw`, bind `^]`)
- Modify: `tests/cli.rs`, `tests/zle/run.zsh`

## Thuật toán (`slots.rs`)

Đầu vào: buffer và các lệnh trong period (`load_entries`). `load_entries` **không** lọc gì; `slots` tự bỏ lệnh
đã quên và lệnh có secret (khi guard bật) bằng cùng bộ lọc mà `rank_by` dùng.

1. **Tách token** kiểu shell: ngăn bởi khoảng trắng; giữ nguyên `'…'`, `"…"`; `\` thoát ký tự kế tiếp. Mỗi token
   mang khoảng char offset.
2. **Sibling:** cùng số token, cùng token đầu, ít nhất một nửa số vị trí trùng buffer. Dedup theo chuỗi lệnh.
3. **Vị trí hay đổi:** vị trí `i ≥ 1` mà tập giá trị (buffer cộng sibling) có ≥ 2 giá trị.
4. **Thu hẹp theo `=`:** mọi giá trị cùng phần trước dấu `=` đầu tiên thì slot chỉ là phần sau `=`.
5. Không có sibling thì không có slot (đã chốt: không đoán theo hình dạng).

`wib slots --stdin` đọc buffer từ stdin; in mỗi dòng `start end` (char offset 0-based, `end` không tính).
Không có slot thì không in gì, exit 0.

## Hành vi trong zsh

- Phím **Ctrl-]** (đã chốt). Emacs keymap: `undefined-key`. Vi insert: `self-insert`. Bind qua
  `_wib_bind_main` và thêm vào `wib-nav`.
- **Đang chọn một dòng:** Ctrl-] điền dòng đó (như Tab) rồi nhảy tới slot đầu tiên.
- **Buffer bất kỳ:** `print -rn -- "$BUFFER" | _wib_run slots --stdin`; nhảy tới slot đầu tiên có
  `start >= CURSOR`, không còn thì quay về slot đầu. Đặt `CURSOR=end`, `MARK=start`, `REGION_ACTIVE=1`.
- **Gõ để thay:** lưu `_wib_slot_pending=(start end buffer)`. Trong `_wib_pre_redraw`, **trước** nhánh refetch:
  nếu buffer mới là buffer cũ chèn thêm `X` tại `end` và `CURSOR == end + ${#X}`, thay `buffer[start,end)` bằng
  `X`. Dán khi có region đã được zsh tự thay; trường hợp này buffer không khớp mẫu chèn nên chỉ huỷ trạng thái
  chờ. Mọi thay đổi khác cũng chỉ huỷ trạng thái chờ và đặt `REGION_ACTIVE=0`.
- **Không lưu vị trí giữa các lần bấm:** mỗi lần bấm tính lại từ buffer hiện tại.
- **Không có slot hoặc buffer nhiều dòng:** hiện `zle -M "wib: no earlier runs to compare"`. Chỉ gọi widget cũ
  khi nó không phải `self-insert` hoặc `undefined-key` (tránh chèn byte `0x1d` trong vi insert).

## Tasks & Steps

- [x] `slots.rs`: `tokenize`, `find`; test: `docker run -p 8080:80 nginx:1.25` với hai sibling khác port và tag →
       hai slot; `kubectl logs pod-a`/`pod-b` → một slot; chuỗi ngoặc kép chứa khoảng trắng là một token;
       `--tag=1.2` thu hẹp sau `=`; không sibling → rỗng; ký tự nhiều byte cho offset theo char; sibling bị
       forget hoặc có secret không được tính.
- [x] `main.rs`: `Some("slots") => cmd_slots(..)`, nhận `--stdin` và `--period`.
- [x] `wib.zsh`: widget, bind, `_wib_keywords` thêm `slots`.
- [x] `tests/cli.rs`: `slots --stdin` in đúng offset; với `WIB_ZSH_HISTORY` trỏ tới file tuỳ chỉnh.
- [x] `tests/zle/run.zsh`: thêm hai `docker run` khác port qua tham số phụ của `start_shell`; gõ `dock`, ↓, Ctrl-]
       → con trỏ ở cuối port; gõ `7070:80` → port được thay; Ctrl-] → tới tag. Với `start_shell 'bindkey -v'`:
       Ctrl-] ở chế độ insert hoạt động, và trên dòng không có sibling buffer **không đổi**.

## Verification

- `cargo test slots` và `cargo test --test cli slots`
- `zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`

## Rủi ro

- **Tokenizer Rust khác `${(z)}`** với cú pháp lạ (`$(…)`, heredoc): offset vẫn nhất quán vì zsh dùng đúng offset
  Rust trả; xấu nhất là slot không trùng ranh giới từ.
- **Terminal chiếm Ctrl-]** (telnet, một số cấu hình tmux): README nói cách bind lại `_wib_slot`.
