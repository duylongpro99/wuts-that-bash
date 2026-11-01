---
phase: 4
title: "Benchmark và README"
status: completed
priority: P2
effort: 1h
dependencies: [3]
---

# Phase 4: Benchmark và README

## Goal
Chứng minh chế độ từ khoá giữ ngân sách tốc độ hiện tại và hướng dẫn người dùng.

## Files to Modify
- `scripts/bench.sh`: thêm chế độ đo từ khoá (cờ hoặc đối số thứ tư), input là 1–2 từ lấy từ lệnh thật.
- `README.md`: mục mới "Tìm theo từ khoá", cập nhật "Dùng từ dòng lệnh", bảng số đo nếu có số mới.

## Tasks & Steps
1. `bench.sh`: chế độ `keywords` lấy ngẫu nhiên 1–2 từ (≥ 3 ký tự) của lệnh thật, nối bằng `,`, gọi
   `wib query --keywords -- …`. Chế độ mặc định giữ nguyên.
2. Đo trên file 100.000 lệnh / 400 ngày (`scripts/gen-history.sh`) và `~/.zsh_history` thật, cả hai chế độ.
3. README:
   - Ví dụ `wib pull`, `wib pull,master`, `wib master pull`; dấu cách = dấu phẩy; thứ tự bất kỳ.
   - Cách xếp hạng: trọn từ > đầu từ > giữa từ, rồi tần suất và độ gần đây.
   - Gõ tại prompt: danh sách live, ↓ + Enter chạy lệnh đã chọn; Enter luôn thì in danh sách.
   - `wib -- init` để tìm chữ trùng tên subcommand; dạng ngắn cần `export WIB_PERIOD`/`WIB_LIMIT` nếu muốn khác mặc định.

## Verification
- p95 chế độ từ khoá ≤ 1,3 lần chế độ thường trên cùng file.
- Ví dụ trong README chạy ra đúng như mô tả.

## Risk
Không có rủi ro hành vi; chỉ tài liệu và script đo.

## Rollback
Revert hai file.
