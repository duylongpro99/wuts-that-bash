---
phase: 2
title: "CLI: wib <kw> và wib query --keywords"
status: completed
priority: P2
effort: 1.5h
dependencies: [1]
---

# Phase 2: CLI cho chế độ từ khoá

## Goal
`wib pull,master` in kết quả ra stdout; widget zsh có cờ `--keywords` để lấy kết quả ở định dạng `-0`.

## Files to Modify
- `src/main.rs`: dispatch, `cmd_query`, `USAGE`, doc comment đầu file.
- `tests/cli.rs`: test mới.

## Tasks & Steps
1. `cmd_query` nhận thêm cờ boolean `--keywords`: có cờ thì gọi `rank::rank_keywords`, không thì `rank::rank`.
   Phần in kết quả (thường và `-0`) dùng chung.
2. Dispatch trong `main`:
   - `query`, `record`, `init`, `-V/--version`, `-h/--help`, không đối số: như cũ.
   - `--`: chế độ từ khoá với phần sau `--`.
   - Đối số khác bắt đầu bằng `-`: lỗi `unknown option` như hiện tại (tránh nuốt cờ gõ sai).
   - Còn lại: chế độ từ khoá với toàn bộ đối số nối bằng dấu cách.
3. Chế độ từ khoá dạng ngắn dựng lại đối số cho `cmd_query`: `--keywords`, `--period` từ `WIB_PERIOD` nếu đặt
   (mặc định `30d`), `--limit` từ `WIB_LIMIT` nếu đặt (mặc định 8). Giá trị env sai thì báo lỗi như cờ sai.
4. Không có từ khoá nào (`wib --`, `wib ,`) → in usage ra stderr, exit 2.
5. Cập nhật `USAGE` và doc comment: thêm `wib <keywords>` với ví dụ `wib pull,master`.

## Tests (`tests/cli.rs`)
- History có `git pull origin master`, `docker pull nginx`, `git push origin master`, `wib pull,master`:
  - `wib pull` → hai lệnh pull, không có dòng `wib`.
  - `wib pull,master`, `wib master pull`, `wib 'master, pull'` → chỉ `git pull origin master`.
  - `wib query --keywords -0 -- pull,master` → cùng kết quả, phân tách NUL.
- `wib -- query` tìm chữ `query` (history có `echo query`).
- `wib --bogus` → exit 2. `wib --` → exit 2.
- `WIB_LIMIT=1 wib pull` → một dòng; `WIB_PERIOD=1d` loại lệnh 3 ngày tuổi.
- Test CLI cũ qua mà không sửa.

## Verification
`cargo test`, `cargo clippy --all-targets -- -D warnings`; chạy tay `wib pull` trên `~/.zsh_history` thật.

## Risk
- Chạy từ shell, `HISTFILE` thường không được export nên dạng ngắn đọc `WIB_ZSH_HISTORY` hoặc `~/.zsh_history`.
  Người dùng có `HISTFILE` khác cần `export WIB_ZSH_HISTORY`; ghi rõ trong README (phase 4).
- Subcommand gõ sai (`wib qeury`) thành tìm từ khoá và in rỗng thay vì báo lỗi. Chấp nhận; usage ghi rõ.

## Rollback
Bỏ nhánh từ khoá trong dispatch và cờ `--keywords`.
