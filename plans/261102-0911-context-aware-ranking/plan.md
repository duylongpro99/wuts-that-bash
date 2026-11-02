---
title: "wib — xếp hạng theo dự án, đẩy lệnh lỗi xuống, và wib forget"
description: "Ưu tiên lệnh đã chạy trong dự án hiện tại, đẩy lệnh chưa từng chạy thành công xuống dưới, và thêm `wib forget` để gỡ hẳn một gợi ý."
status: completed
priority: P2
effort: 6.5h
branch: feat/context-aware-ranking
tags: [feature, rust, zsh]
blockedBy: []
blocks: []
created: 2026-11-02
---

# wib — xếp hạng theo dự án, đẩy lệnh lỗi xuống, và `wib forget`

## Overview

`wib record` đã lưu thư mục (`cwd`) và exit code của mọi lệnh vào `history.tsv`, nhưng `rank_by` trong
`src/rank.rs` chỉ dùng mức khớp, số lần dùng và độ gần đây. Plan này dùng hai dữ liệu sẵn có đó và thêm
một lệnh gỡ gợi ý:

1. **Theo dự án:** gõ `np` trong repo A thì `npm run dev` của repo A đứng trước lệnh `npm` của repo khác.
2. **Lệnh lỗi xuống dưới:** `gti status`, lệnh gõ sai cờ (chưa từng exit 0) xếp sau lệnh cùng mức khớp.
3. **`wib forget <lệnh>`:** gỡ hẳn một lệnh khỏi gợi ý (lệnh chứa token, lệnh sai), không phải sửa tay
   `.zsh_history`.

Không đổi định dạng log, không thêm phím, không thêm dependency.

## Kiến trúc sau thay đổi

```text
 zsh _wib_fetch ──► wib query -0 --cwd "$PWD" [--keywords] -- <input>
 wib <kw> (CLI) ──► cmd_keywords, cwd = $PWD
                         │
 main.rs: load_entries + project::scope(cwd) + log_store::load_forgotten
                         ▼
 rank.rs  RankContext { scope: Option<String>, forgotten: HashSet<String> }
   bỏ: lệnh trong forgotten, lệnh `wib forget ...`
   gộp: count, last_ts, local (cwd trong scope), known_exit, succeeded
   sort: tier ↑, score ↓, failed ↑, local ↓, frecency ↓, last_ts ↓

 wib forget <cmd> ──► log_store::forget ──► <data_dir>/forget (0600, mỗi dòng một lệnh đã escape)
```

## Quyết định đã xác nhận với người dùng (2026-11-02)

- **Phạm vi:** cả ba tính năng (#1 theo dự án, #2 lệnh lỗi, #3 `wib forget`).
- **"Dùng ở đây" nghĩa là cùng dự án git:** Rust đi ngược từ `$PWD` lên tìm `.git` (thư mục hoặc file, để
  worktree cũng đúng), không gọi process `git`. Lệnh chạy ở bất kỳ đâu trong dự án đều tính. Ngoài git thì
  dùng đúng `$PWD`.
- **Lệnh trong dự án luôn đứng trước** khi mức khớp bằng nhau (khoá sort, không phải hệ số nhân), rồi mới
  tới frecency.
- **"Lỗi" nghĩa là chưa từng thành công:** mọi lần chạy có exit code đều khác 0. Các mã do người dùng dừng
  lệnh không tính là lỗi: 130 (Ctrl-C), 141 (SIGPIPE), 143 (SIGTERM). Lệnh lỗi chỉ bị đẩy xuống, không bị ẩn.

## Quyết định kỹ thuật

- **Thứ tự khoá sort:** tier, score, failed, local, frecency, last_ts. Mức khớp vẫn quyết định trước; lệnh
  lỗi trong dự án vẫn đứng sau lệnh chạy được ở nơi khác.
- **Thêm 148 (Ctrl-Z) vào nhóm mã trung tính.** Cùng tinh thần với 130/143: `vim` bị suspend không phải lỗi.
- **Exit code không rõ không tính.** Lệnh chỉ có trong `.zsh_history` (không có exit) không bao giờ bị coi
  là lỗi. Lệnh bị coi là lỗi khi có ít nhất một lần chạy có exit code và không lần nào thành công.
- **Bỏ qua `.git` ở `$HOME` và `/`.** Người dùng quản lý dotfiles bằng git ở `$HOME` sẽ khiến mọi thư mục
  thành "cùng dự án"; khi đó dùng đúng `$PWD`, như ngoài git.
- **Khớp scope theo ranh giới thư mục:** `cwd == scope` hoặc `cwd` bắt đầu bằng `scope + "/"`, để
  `/repo` không khớp `/repo-old`.
- **`wib query` không có `--cwd` thì không ưu tiên dự án.** Script và test cũ cho kết quả như trước. Widget
  zsh luôn truyền `--cwd "$PWD"`; `wib <kw>` dạng ngắn đọc `$PWD` (đường dẫn logic, giống lúc `record`),
  không có thì `current_dir()`.
- **`forget` khớp đúng chuỗi lệnh** sau khi trim, giống khoá dedup. Lưu ở `<data_dir>/forget`, mỗi dòng một
  lệnh, escape bằng `escape`/`unescape` sẵn có (lệnh nhiều dòng vẫn đúng). Ghi lặp lại thì không thêm dòng.
- **Lệnh `wib forget ...` cũng bị loại khỏi gợi ý** ở cả hai chế độ, vì nó chứa chính chuỗi cần quên
  (có thể là token).
- **`forget` thành subcommand.** Tìm chữ `forget` thì dùng `wib -- forget`, như `init`/`query`/`record`.
  Widget zsh không vào chế độ từ khoá khi buffer là `wib <subcommand> ...`.

## Phases

| # | Phase | Effort | Phụ thuộc | Trạng thái |
|---|-------|--------|-----------|-----------|
| 1 | [Xếp hạng theo ngữ cảnh trong rank.rs](phase-01-context-ranking.md) | 2.5h | — | completed |
| 2 | [CLI: `--cwd`, `wib forget`, file forget](phase-02-cli-cwd-and-forget.md) | 1.5h | 1 | completed |
| 3 | [Widget zsh truyền `$PWD` và test zpty](phase-03-zsh-widget.md) | 1h | 2 | completed |
| 4 | [Benchmark và README](phase-04-bench-and-docs.md) | 1.5h | 3 | completed |

Các phase chạy tuần tự: phase sau gọi trực tiếp code của phase trước.

## Acceptance criteria

- [x] Hai lệnh cùng mức khớp: lệnh từng chạy trong dự án hiện tại (kể cả thư mục con của nó) đứng trước,
      dù lệnh kia dùng nhiều hơn. Ra ngoài dự án thì thứ tự về như cũ.
- [x] `/repo` không coi lệnh chạy ở `/repo-old` là cùng dự án; `.git` ở `$HOME` không gộp cả home thành một dự án.
- [x] Lệnh có exit code và chưa lần nào exit 0/130/141/143/148 xếp sau lệnh cùng mức khớp; vẫn hiện trong danh sách.
- [x] Lệnh chỉ có trong `.zsh_history` không bị coi là lỗi.
- [x] `wib forget git push --force` (hoặc dạng có nháy) làm lệnh đó biến mất khỏi danh sách live, `wib query`
      và `wib <kw>`; lệnh `wib forget ...` cũng không được gợi ý. Chạy lại không thêm dòng trùng.
- [x] File forget có quyền 0600, thư mục 0700. `wib forget` không đối số thì exit 2.
- [x] `wib -- forget` tìm chữ `forget`; `wib query` không `--cwd` cho kết quả như trước; mọi test cũ qua.
- [x] p95 của `wib query --cwd` không quá 1,1 lần khi không có `--cwd` (file giả 100.000 lệnh / 400 ngày, 200 lần tra).
- [x] `cargo test`, `cargo clippy --all-targets -- -D warnings`, `zsh tests/zle/run.zsh` đều qua.
- [x] README mô tả cả ba tính năng, file forget, và việc `forget` không xoá `.zsh_history`.

## Non-goals

- Lọc chỉ hiện lệnh trong dự án (chế độ filter kiểu atuin); chỉ đổi thứ tự.
- Ẩn lệnh lỗi; xoá dòng khỏi `.zsh_history` hay `history.tsv`.
- Lệnh `wib forget --list` hay `wib unforget`; muốn hoàn tác thì sửa file forget (README ghi rõ).
- Đổi định dạng `history.tsv`, đổi cách khớp, thêm phím.

## Validation Log

### Session 1 (2026-11-02), phỏng vấn trước khi viết plan
1. Phạm vi → **#1 + #2 + #3**.
2. "Dùng ở đây" → **cùng dự án git** (fallback `$PWD`).
3. Độ mạnh ưu tiên → **lệnh trong dự án luôn đứng trước** khi khớp bằng nhau.
4. Định nghĩa lỗi → **chưa từng thành công**, bỏ qua 130/141/143.

### Session 2 (2026-11-02), sau khi viết plan
5. Thêm 148 vào mã trung tính và bỏ qua `.git` ở `$HOME` và `/` → **giữ cả hai**.

### Verification Results
- Claims checked: 8 (`log_store::escape`, `unescape`, `lock`, `home`, `data_dir`; `rank::is_wib_call`;
  `_wib_fetch`/`_wib_keywords` trong `shell/wib.zsh`; test AC4 trong `tests/zle/run.zsh`)
- Verified: 8 | Failed: 0 | Unverified: 0
- Tier: Standard
