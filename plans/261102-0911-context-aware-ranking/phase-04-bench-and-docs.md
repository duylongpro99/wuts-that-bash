---
phase: 4
title: "Benchmark và README"
status: completed
priority: P2
effort: 1.5h
dependencies: [3]
---

# Phase 4: Benchmark và README

## Goal
Chứng minh `--cwd` không làm chậm tra cứu, và README mô tả ba tính năng mới.

## Files to Modify
- `scripts/bench.sh`: truyền `--cwd "$PWD"` để đo đúng đường widget dùng.
- `README.md`.

## Tasks & Steps
1. `bench.sh`: thêm `--cwd "$PWD"` vào lời gọi query; ghi chú trong phần đầu file.
2. Đo trên file giả 100.000 lệnh / 400 ngày, 200 lần tra, hai lượt xen kẽ có và không có `--cwd`.
   p95 có `--cwd` không quá 1,1 lần; ghi số vào bảng benchmark nếu thay đổi.
3. README:
   - "How it matches": lệnh dùng trong dự án hiện tại (gốc git, hoặc thư mục hiện tại ngoài git) đứng trước;
     lệnh chưa từng chạy thành công xếp sau; mã 130/141/143/148 không tính là lỗi.
   - Mục mới "Forgetting a suggestion": `wib forget 'cmd'`, file `<data dir>/forget`, sửa file để hoàn tác,
     `forget` không xoá `.zsh_history` (dùng dấu cách đầu dòng cho lệnh nhạy cảm).
   - "Command-line usage": thêm `wib forget`, `query --cwd`; `wib -- forget`.
   - Bullet "Ranked by you" ở pitch: thêm "and by the project you're in".
4. Kiểm tra lại các con số và tên biến trong README khớp code.

## Verification
`zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 200` với và không có `--cwd`; đọc lại README.

## Risk
Nhiễu đo đạc trên macOS; chạy xen kẽ hai lượt như plan trước.

## Rollback
Hoàn tác `bench.sh` và README.
