---
title: "Plan: thời gian tra của wib chỉ phụ thuộc WIB_PERIOD"
date: 2026-11-01
summary: wib query đọc toàn bộ hai file history mỗi phím; plan đọc .zsh_history từ cuối và tự dọn history.tsv theo WIB_PERIOD
---

# Plan: thời gian tra của wib chỉ phụ thuộc WIB_PERIOD

## What happened
Đo `wib query` bằng `scripts/bench.sh` trên file `.zsh_history` giả, lệnh trải đều trên 400 ngày, 100 lần mỗi cỡ:
p95 là 6,3 ms ở 10k lệnh, 17 ms ở 100k và 57 ms ở 500k. Bộ nhớ tối đa tăng từ ~2 MB lên ~31 MB.
Nguyên nhân: `load_entries` gọi `std::fs::read` trên cả `.zsh_history` và `history.tsv`, rồi mới lọc theo
period. `history.tsv` chỉ ghi thêm, không bao giờ dọn. Không có rò rỉ bộ nhớ vì mỗi lần tra là một tiến trình riêng.

## Decision
Plan `plans/261101-1654-period-bounded-query/`:
- Đọc `.zsh_history` từ cuối theo khối (256 KB, nhân đôi khi cần), dừng khi header đầu khối cũ hơn cutoff trừ 1 ngày
  (dung sai cho `sharehistory` ghi lệnh chạy lâu lệch thứ tự). Không bao giờ sửa `.zsh_history`.
- `wib record --period` dọn `history.tsv` khi dòng đầu cũ hơn WIB_PERIOD cộng 1 ngày. Khoá bằng file riêng
  `history.lock` (`File::lock`), vì khoá trên `history.tsv` sẽ hỏng sau rename. Ghi tmp, `sync_all`, rồi rename.
- Người dùng chốt: ngưỡng period + 1 ngày; chấp nhận mất dữ liệu nếu sau này tăng WIB_PERIOD; không sao lưu.

## Next steps
Chạy `/ak-cook` theo plan. Phase 1 và 2 độc lập; phase 3 đo lại và cập nhật README
(bỏ đoạn `awk` dọn tay). Acceptance: p95 ở 500k lệnh không quá 1,3 lần mốc chỉ chứa lệnh trong period và dưới 20 ms.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
