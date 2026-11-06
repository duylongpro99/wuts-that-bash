---
phase: 6
title: "Benchmark, README, site"
status: completed
priority: P2
effort: "2h"
dependencies: [1, 2, 3, 4, 5]
---

# Phase 6: Benchmark, README, site

## Goal

Chứng minh không chậm đi, và tài liệu nói đúng hành vi mới.

## Files to Create / Modify

- Modify: `README.md`
- Modify: `site/` chỉ khi trang đang liệt kê tính năng hoặc lệnh con bị ảnh hưởng (kiểm tra bằng `grep -rn "forget\|Known limitations\|Ctrl" site/`)
- Modify: `src/main.rs` doc comment đầu file nếu các phase trước chưa cập nhật

## Tasks & Steps

- [x] Lấy số đo gốc đã ghi ở bước đầu của phase 1 (`reports/bench-before.txt`).
- [x] Chạy lại đúng các lệnh đó; p95 của `query` không tăng quá 10%. Chạy thêm chế độ `suggest-fix` và `slots`
       (thêm ở phase 4) trên file 500k lệnh / 400 ngày; p95 < 15 ms. Ghi kết quả vào `reports/bench-after.txt`.
       Nếu vượt, tối ưu trước khi tiếp tục.
- [x] README: thêm mục "Secrets", "Templates (Ctrl-])", "Did you mean" (chỉ exit 127, chỉ sửa tên chương trình), "Doctor", biến `WIB_SECRET_GUARD`, và `wib unforget` trong mục
       "Forgetting a suggestion" (bỏ câu "There is no `unforget`"). Cập nhật bảng phím, "Command-line usage",
       danh sách lệnh con cần `wib --`, và "Known limitations" (secret guard là lưới an toàn, không đảm bảo
       tuyệt đối; hàm `wib` trong shell tương tác).
- [x] Cập nhật bảng benchmark trong README nếu số thay đổi đáng kể.

## Verification

- `cargo test && cargo install --path . --root .bin && zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`
- Đọc lại README: mọi lệnh ví dụ chạy được trong sandbox.
