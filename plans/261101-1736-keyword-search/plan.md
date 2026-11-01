---
title: "wib — tìm lệnh cũ theo từ khoá"
description: "Gõ `wib pull` hoặc `wib pull,master` để thấy các lệnh cũ chứa đủ các từ khoá, theo thứ tự bất kỳ, ngay trong danh sách live và khi chạy như lệnh."
status: completed
priority: P2
effort: 6.5h
branch: feat/keyword-search
tags: [feature, rust, zsh]
blockedBy: []
blocks: []
created: 2026-11-01
---

# wib — tìm lệnh cũ theo từ khoá

## Overview

Người dùng thường chỉ nhớ một vài từ trong lệnh cũ (ví dụ chỉ nhớ `pull` của `git pull origin master`).
Hiện tại `pull master` chỉ khớp khi đúng thứ tự, `pull,master` không khớp gì, và gõ `wib pull` tại prompt
không ra gì vì buffer được coi là lệnh bắt đầu bằng `wib`. Bấm Enter thì báo `unknown command 'pull'`.

Plan này thêm **chế độ từ khoá**: `wib` là tên tool, phần sau nó là các từ khoá cần tìm.

```text
PROMPT> wib pull,master          PROMPT> wib pull master          $ wib pull
› git pull origin master         › git pull origin master         git pull origin master
                                                                  docker pull nginx
```

## Kiến trúc sau thay đổi

```text
 zsh widget ── BUFFER = "wib <kw>" ? ──có──► wib query -0 --keywords -- "<kw>" ──► danh sách live
                     │                                   │
                     └─không──► wib query -0 -- "$BUFFER" (như cũ)
                                                         ▼
 Enter khi chưa chọn dòng ──► wib <kw> ──► cùng đường keyword ──► in ra stdout, mỗi dòng một lệnh

 rank.rs:  dedup + frecency + sort  (dùng chung)
              ├── Pattern         (prefix / fuzzy đầu từ, như cũ)
              └── KeywordPattern  (mới: AND các từ khoá, thứ tự bất kỳ)
```

## Quyết định đã xác nhận với người dùng (2026-11-01)

- **Bề mặt:** cả danh sách live (khi buffer bắt đầu bằng `wib `) lẫn CLI (`wib <từ khoá>` in ra stdout).
- **Dấu cách tương đương dấu phẩy:** `wib pull master` = `wib pull,master` = `wib master,pull`.
- **Khớp ở bất kỳ đâu, đầu từ xếp trên:** mỗi từ khoá khớp như chuỗi con ở bất kỳ vị trí nào; khi xếp hạng,
  trọn từ > đầu từ > giữa từ.
- **`wib` là tên tool, không phải từ khoá.** Chỉ phần sau `wib ` được tìm. Hệ quả: trong chế độ từ khoá, các lệnh
  `wib ...` đã chạy trước đó bị loại khỏi kết quả, vì chúng chứa chính các từ khoá đã gõ và chỉ là nhiễu.
  Chế độ thường không đổi.

## Quyết định kỹ thuật

- **Tách từ khoá theo dấu phẩy và khoảng trắng**, bỏ phần rỗng. Smart case như matcher hiện tại: có chữ hoa thì
  phân biệt hoa thường.
- **Điểm một lệnh = tổng điểm từng từ khoá**, mỗi từ khoá lấy vị trí tốt nhất: trọn từ 3, đầu từ 2, giữa từ 1.
  Ranh giới từ dùng lại `is_boundary` trong `matcher.rs`. Cùng điểm thì theo frecency rồi độ gần đây, như hiện nay.
- **zsh nhận diện tiền tố `wib `**, Rust không đoán. Widget gọi `wib query --keywords`; `wib query` không cờ giữ
  nguyên hành vi, nên test và script cũ không đổi.
- **CLI dạng ngắn:** đối số đầu không phải subcommand hay cờ đã biết → chế độ từ khoá. `wib -- <kw>` để tìm chữ trùng
  tên subcommand (`init`, `query`, `record`). Dạng ngắn đọc `WIB_PERIOD`, `WIB_LIMIT` từ môi trường nếu có.
- **Chọn dòng thay cả buffer.** `_wib_fill` đã gán `BUFFER=$1`, nên ↓ + Enter/Tab không cần sửa. Shift+Tab điền
  từng từ từ đầu lệnh vì buffer `wib pul` không phải tiền tố của lệnh; đây là hành vi mong muốn.

## Phases

| # | Phase | Effort | Phụ thuộc | Trạng thái |
|---|-------|--------|-----------|-----------|
| 1 | [Matcher và xếp hạng từ khoá](phase-01-keyword-matcher.md) | 2h | — | completed |
| 2 | [CLI: `wib <kw>` và `wib query --keywords`](phase-02-cli-keyword-mode.md) | 1.5h | 1 | completed |
| 3 | [Danh sách live khi gõ `wib `](phase-03-zsh-keyword-mode.md) | 2h | 2 | completed |
| 4 | [Benchmark và README](phase-04-bench-and-docs.md) | 1h | 3 | completed |

Các phase chạy tuần tự: phase sau gọi trực tiếp code của phase trước.

## Acceptance criteria

- [x] `wib pull`, `wib pull,master`, `wib master pull`, `wib 'master, pull'` đều trả `git pull origin master`;
      `docker pull nginx` chỉ có trong kết quả `wib pull`.
- [x] Mọi từ khoá phải xuất hiện (AND); thiếu một từ thì lệnh bị loại.
- [x] `8080` khớp `docker run -p8080:80 nginx`; lệnh có từ khoá trọn từ xếp trên lệnh chỉ khớp giữa từ.
- [x] Trong chế độ từ khoá, lệnh `wib ...` cũ không xuất hiện.
- [x] Gõ `wib pul` tại prompt hiện danh sách live; ↓ + Enter chạy lệnh đã chọn, không chạy `wib ...` (test zpty).
- [x] Buffer không bắt đầu bằng `wib ` cho kết quả giống hệt trước; mọi test cũ qua. Ngoại lệ có chủ đích:
      `tests/cli.rs` từng kiểm `wib bogus` báo lỗi, nay đó là tìm từ khoá nên assert đổi sang `wib --bogus`.
- [x] `wib -- query` tìm chữ `query`; `wib query`, `wib record`, `wib init zsh`, `-h`, `-V` giữ nguyên.
- [x] p95 của chế độ từ khoá không quá 1,3 lần chế độ thường: file giả 100.000 lệnh / 400 ngày, 200 lần tra,
      hai lượt xen kẽ: 10,1 ms so với 7,9 ms (1,27) và 7,3 ms so với 7,6 ms (0,96).
- [x] `cargo test` (40 unit + 14 CLI), `cargo clippy --all-targets -- -D warnings`, `zsh tests/zle/run.zsh` (47) đều qua.
- [x] README có mục tìm theo từ khoá.

## Non-goals

- Sửa lỗi chính tả, regex, toán tử OR/NOT, cú pháp cụm có thứ tự.
- Lọc theo thư mục hay exit code; tìm ngoài `WIB_PERIOD`.
- Giao diện chọn toàn màn hình kiểu fzf; alias khác tên cho `wib`.
- Đổi cách khớp hay xếp hạng của chế độ thường.
