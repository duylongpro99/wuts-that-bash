---
phase: 2
title: "CLI: --cwd, wib forget, file forget"
status: completed
priority: P2
effort: 1.5h
dependencies: [1]
---

# Phase 2: CLI cho `--cwd` và `wib forget`

## Goal
`wib query --cwd <dir>` và `wib <kw>` dùng scope dự án; `wib forget <lệnh>` ghi vào file forget và mọi
đường tra cứu đều bỏ các lệnh đó.

## Files to Modify
- `src/log_store.rs`: `FORGET_FILE`, `load_forgotten(dir)`, `forget(dir, cmd)`.
- `src/main.rs`: dispatch `forget`, `cmd_forget`, `cmd_query --cwd`, `cmd_keywords`, `USAGE`, doc comment đầu file.
- `tests/cli.rs`: test mới.

## Tasks & Steps
1. `log_store.rs`:
   - `pub const FORGET_FILE: &str = "forget";`
   - `load_forgotten(dir) -> HashSet<String>`: đọc file, mỗi dòng `unescape` rồi trim, bỏ dòng rỗng.
     File không có → tập rỗng.
   - `forget(dir, cmd) -> io::Result<bool>`: tạo thư mục 0700, mở file 0600 (siết quyền nếu lỏng, như `append`),
     trả `false` nếu lệnh đã có, ngược lại ghi `escape(cmd) + "\n"` bằng một lần `write_all`, dưới `lock(dir)`.
2. `main.rs`:
   - Dispatch `Some("forget") => cmd_forget(&args[1..])`.
   - `cmd_forget`: bỏ `--` đầu nếu có, nối đối số bằng dấu cách, trim. Rỗng → lỗi kèm usage (exit 2).
     In `forgot: <cmd>` hoặc `already forgotten: <cmd>` ra stdout.
   - `cmd_query`: thêm value flag `--cwd`. Có thì `scope = project::scope(cwd, &log_store::home())`.
     Luôn nạp `forgotten` từ `log_store::data_dir()`. Truyền `RankContext` vào `rank`/`rank_keywords`.
   - `cmd_keywords`: thêm `--cwd` lấy từ `$PWD`, không có thì `std::env::current_dir()`; cả hai đều thiếu thì bỏ qua.
   - `USAGE` và doc comment: thêm `wib forget <command>`, `--cwd <dir>` cho `query`; ghi `forget` vào danh sách
     tên cần `wib --`.

## Tests (`tests/cli.rs`)
- Log có `make test` (cwd `<sb>/repo/sub`, 1 lần) và `make build` (cwd `/elsewhere`, 3 lần), `<sb>/repo/.git` tồn tại:
  `query --cwd <sb>/repo -- make` → `make test` trước; không `--cwd` → `make build` trước.
- Log có `gti status` exit 127 và `git status` exit 0 cùng tuổi → `gi` trả `git status` trước `gti status`.
- `wib forget git push --force` → exit 0; `query -- git` không còn lệnh đó; chạy lại → file vẫn một dòng;
  quyền file 0600.
- `wib forget 'echo "a b"'` quên đúng lệnh có nháy; lệnh nhiều dòng quên được qua `forget -- $'a\nb'`.
- `wib forget` không đối số → exit 2. `wib -- forget` tìm chữ `forget`.
- `wib forget x` có trong history thì không xuất hiện trong `query -- wib`.
- Test cũ qua mà không sửa.

## Verification
`cargo test`, `cargo clippy --all-targets -- -D warnings`.

## Risk
- Chạy từ shell, đối số không nháy mất khoảng trắng kép (`ls  -la`). README khuyên đặt lệnh trong nháy đơn.
- `wib forget` chính nó được ghi vào `.zsh_history` (chứa token). Gợi ý đã lọc; README nhắc dùng dấu cách đầu
  (`HIST_IGNORE_SPACE`) khi lệnh nhạy cảm.

## Rollback
Bỏ dispatch `forget`, cờ `--cwd`, hai hàm trong `log_store.rs`.
