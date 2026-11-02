---
phase: 3
title: "Widget zsh truyền $PWD và test zpty"
status: completed
priority: P2
effort: 1h
dependencies: [2]
---

# Phase 3: Widget zsh

## Goal
Danh sách live dùng scope dự án của shell hiện tại; gõ `wib forget ...` không vào chế độ từ khoá.

## Files to Modify
- `shell/wib.zsh`: `_wib_fetch`, `_wib_keywords`.
- `tests/zle/run.zsh`: test mới.

## Tasks & Steps
1. `_wib_fetch`: thêm `--cwd "$PWD"` vào lời gọi `"$_wib_bin" query -0 ...`.
2. `_wib_keywords`: trả 1 khi buffer là `wib <subcommand>` với subcommand trong `forget|query|record|init`
   (theo sau là khoảng trắng hoặc hết dòng). Buffer đó đi đường chế độ thường như mọi lệnh khác; lịch sử
   `wib forget ...` đã bị Rust lọc nên danh sách trống.

## Tests (`tests/zle/run.zsh`)
- Hai repo tạm có `.git`, mỗi repo một lệnh `make` khác nhau chạy trong shell; `cd` sang repo 1, gõ `ma`
  → dòng đầu là lệnh của repo 1; `cd` sang repo 2 → dòng đầu đổi.
- Chạy `wib forget <lệnh>` rồi gõ tiền tố của nó → lệnh không còn trong danh sách.
- Gõ `wib forget gi` → không có danh sách từ khoá (probe đếm `_wib_items` = 0).
- Mọi test zpty cũ qua.

## Verification
`cargo install --path . --root .bin && zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`.

## Risk
Test zpty phụ thuộc thời gian (`sleep 0.3`); lệnh `record` chạy nền nên cần chờ trước khi gõ, như test AC4 hiện có.

## Rollback
Gỡ `--cwd "$PWD"` và nhánh loại subcommand.
