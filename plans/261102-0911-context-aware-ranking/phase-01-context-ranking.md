---
phase: 1
title: "Xếp hạng theo ngữ cảnh trong rank.rs"
status: completed
priority: P2
effort: 2.5h
dependencies: []
---

# Phase 1: Xếp hạng theo ngữ cảnh

## Goal
`rank` và `rank_keywords` nhận một `RankContext` (scope dự án và tập lệnh đã quên) và sắp xếp theo
tier, score, failed, local, frecency, last_ts.

## Files to Create / Modify
- Create: `src/project.rs`: `scope(cwd: &str, home: &Path) -> String`.
- Modify: `src/rank.rs`: `RankContext`, `Candidate`, `rank_by`, các test.
- Modify: `src/main.rs`: `mod project;` (chỉ khai báo; phase 2 mới dùng).

## Tasks & Steps
1. `src/project.rs`:
   - `scope(cwd, home)`: đi qua `Path::new(cwd).ancestors()`; thư mục đầu tiên có `.git` (`exists()`, file
     hoặc thư mục) là gốc dự án, trừ khi nó là `home` hoặc `/`. Không tìm thấy thì trả `cwd`.
   - `in_scope(dir, scope) -> bool`: `dir == scope` hoặc `dir` bắt đầu bằng `scope` + `/`
     (scope là `/` thì mọi đường dẫn tuyệt đối đều khớp).
2. `src/rank.rs`:
   - `pub struct RankContext<'a> { pub scope: Option<&'a str>, pub forgotten: &'a HashSet<String> }` và
     `RankContext::none()` (dùng cho test và `query` không `--cwd`). Một `HashSet` rỗng tĩnh hoặc
     `Option<&HashSet>`, chọn cách gọn hơn khi viết.
   - `Candidate` thêm `local: bool`, `known_exit: bool`, `succeeded: bool`. Khi gộp entry:
     `local |= e.cwd.is_some_and(|d| in_scope(d, scope))`; nếu `e.exit` là `Some(code)` thì
     `known_exit = true` và `succeeded |= is_ok(code)`.
   - `fn is_ok(code) -> bool`: `matches!(code, 0 | 130 | 141 | 143 | 148)`.
   - `failed = known_exit && !succeeded`.
   - `keep`: bỏ lệnh có trong `forgotten` (so trên chuỗi đã trim) và lệnh là `wib forget` (mở rộng
     `is_wib_call` thành hàm nhận tên subcommand, hoặc thêm `is_wib_forget`).
   - Sort: `tier ↑`, `score ↓`, `failed ↑` (false trước), `local ↓` (true trước), `frecency ↓`, `last_ts ↓`.
3. Cập nhật chữ ký `rank(entries, input, now, limit, ctx)` và `rank_keywords(...)`; sửa test cũ truyền
   `RankContext::none()`.

## Tests (unit, trong `rank.rs` và `project.rs`)
- Local trước: `npm test` (2 lần, cwd `/repo/a`) và `npm install` (10 lần, cwd `/other`), scope `/repo`
  → `npm test` trước. Không scope → `npm install` trước.
- Mức khớp vẫn thắng: lệnh prefix ở nơi khác đứng trước lệnh fuzzy trong dự án.
- Failed: `gti status` exit 127 xếp sau `git status` exit 0 cùng tier/score. Lệnh exit 1 rồi exit 0 không failed.
  Lệnh chỉ exit 130 không failed. Lệnh `exit: None` không failed.
- Failed thắng local: lệnh lỗi trong dự án đứng sau lệnh chạy được ngoài dự án.
- Forgotten: lệnh trong tập bị loại ở cả `rank` và `rank_keywords`; `wib forget x` không bao giờ hiện.
- `project::scope` với thư mục tạm (tạo bằng `std::env::temp_dir()` + tên riêng, như `Sandbox` trong
  `tests/cli.rs`, không thêm crate): có `.git` thư mục, có `.git` file, không có `.git`, `.git` ở `home`.
- `in_scope("/repo-old", "/repo") == false`, `in_scope("/repo/x", "/repo") == true`.

## Verification
`cargo test`, `cargo clippy --all-targets -- -D warnings`.

## Risk
- Đi ngược thư mục gọi vài `stat` mỗi lần tra; chỉ làm một lần mỗi process, không đáng kể (đo ở phase 4).
- Thư mục mạng chậm (NFS) làm `stat` chậm. Chấp nhận; ghi chú nếu bench cho thấy khác biệt.

## Rollback
Bỏ `RankContext`, khôi phục khoá sort cũ; xoá `src/project.rs`.
