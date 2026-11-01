---
title: "wib: thời gian tra theo WIB_PERIOD, log tự dọn"
date: 2026-11-01
summary: "Đọc .zsh_history từ cuối và dọn history.tsv theo WIB_PERIOD; p95 ở 500k lệnh/7 năm từ 40,8 ms xuống 6,2 ms"
---

# wib: thời gian tra theo WIB_PERIOD, log tự dọn

## What happened
Làm xong plan `plans/261101-1654-period-bounded-query`.
- `zsh_history::read_since` đọc từ cuối theo khối 256 KB (nhân đôi khi cần) và dừng ở header cũ hơn cutoff trừ 1 ngày.
- `log_store::append` khoá bằng `history.lock` riêng, rồi dọn `history.tsv` khi dòng đầu cũ hơn WIB_PERIOD + 1 ngày (tmp, fchown, sync, rename).

Lần so đầu tiên báo khác biệt ở mọi query. Nguyên nhân là file giả sinh tay xếp mới nhất lên đầu, không phải lỗi code. Từ đó viết `scripts/gen-history.sh` sinh đúng thứ tự.

Review độc lập tái hiện được lỗi mất dữ liệu: `parse_period` nhân tràn số với `9999999999999999d`, nên cutoff nằm ở tương lai và lần dọn xoá cả lệnh vừa ghi. Đã sửa bằng `checked_mul` và `saturating_sub`. Cũng đã sửa dòng heredoc giả header làm đọc ngược dừng sai, ghi khi không khoá được, và giữ owner khi root dọn.

## Result
- p95 với 500k lệnh trong 7 năm: 40,8 ms xuống 6,2 ms; bộ nhớ 22,9 MB xuống 3,3 MB.
- p95 với 500k lệnh trong 400 ngày: 23,0 ms, bằng 1,08 lần mốc chỉ chứa lệnh trong period. Chưa đạt mục tiêu tuyệt đối 20 ms vì file này có ~37.500 lệnh trong period.
- 205 query cho kết quả giống hệt bản cũ; 44 cargo test và 38 zle test đều qua.

## Next steps
Cài lại binary và mở shell mới để hook truyền `--period`. Chưa commit.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
