---
title: "wib — thời gian tra chỉ phụ thuộc WIB_PERIOD"
description: "Đọc .zsh_history từ cuối và dừng ở mốc WIB_PERIOD, tự dọn history.tsv khi có lệnh cũ hơn WIB_PERIOD, để thời gian tra không tăng theo năm tháng sử dụng."
status: completed
priority: P2
effort: 7h
branch: perf/period-bounded-query
tags: [performance, rust, zsh]
blockedBy: []
blocks: []
created: 2026-11-01
---

# wib — thời gian tra chỉ phụ thuộc WIB_PERIOD

## Overview

Hiện mỗi lần buffer đổi, `wib query` đọc **toàn bộ** `~/.zsh_history` và `~/.local/share/wib/history.tsv`,
rồi mới bỏ các lệnh cũ hơn `WIB_PERIOD`. Thời gian tra vì thế tăng theo tổng số lệnh từng gõ:

| Số lệnh trong file (trải trên ~400 ngày) | p95 hiện tại | Bộ nhớ tối đa |
|---|---|---|
| 10.000 | 6,3 ms | ~2 MB |
| 100.000 | 17 ms | ~8 MB |
| 500.000 | 57 ms | ~31 MB |

Người dùng chấp nhận wib không giữ lại lệnh cũ hơn `WIB_PERIOD`. Plan này làm hai việc:

1. **`.zsh_history` (file của zsh, wib không được sửa):** đọc từ cuối file lên theo từng khối, dừng khi gặp
   lệnh cũ hơn mốc period. Phần cũ của file không bao giờ được đọc.
2. **`history.tsv` (file của wib):** sau khi ghi một lệnh, nếu dòng đầu file cũ hơn `WIB_PERIOD` thêm một
   ngày thì viết lại file chỉ với các dòng còn trong period.

## Kiến trúc sau thay đổi

```text
 query ──► history.tsv  (đã được dọn, chỉ chứa ≈ period + 1 ngày)  ──► đọc hết
       └─► .zsh_history ──► đọc khối cuối 256 KB ──► header đầu khối còn trong period?
                                 ▲                         │ có: nhân đôi khối, đọc lại
                                 └─────────────────────────┘
                                                           │ không: parse từ header đó → cuối file
 record (chạy nền) ──► khoá history.lock ──► append ──► dòng đầu cũ hơn period + 1d?
                                                           │ có: ghi file .tmp chỉ với dòng mới → rename
```

## Quyết định chính

- **Không đụng vào `.zsh_history`.** File này thuộc zsh và người dùng; wib chỉ đọc ít đi.
- **Ngưỡng dọn là `WIB_PERIOD` cộng 1 ngày.** Phần thêm một ngày giúp việc viết lại file chỉ xảy ra khoảng
  một lần mỗi ngày thay vì sau mỗi lệnh. Query vẫn lọc đúng `WIB_PERIOD` nên kết quả không đổi.
- **Dọn trong `wib record`, không dọn trong `wib query`.** `record` vốn chạy nền sau mỗi lệnh nên không làm
  chậm prompt; `query` giữ chỉ đọc, không cần khoá.
- **Khoá bằng file riêng `history.lock`** (`File::lock`, có từ Rust 1.89; máy đang dùng 1.99). Cả append và
  dọn đều giữ khoá này. Khoá file riêng thay vì khoá `history.tsv` vì sau khi rename, tiến trình đang chờ khoá
  trên inode cũ sẽ ghi vào file đã bị thay và mất dòng.
- **Viết lại kiểu an toàn:** ghi `history.tsv.tmp` quyền 0600, `sync_all`, rồi `rename`. Query đang chạy đọc
  được file cũ hoặc file mới, không bao giờ đọc file dở.
- **`record` chỉ dọn khi được truyền `--period`.** Script zsh truyền `${WIB_PERIOD:-30d}`. Shell cũ chưa nạp
  lại script, hoặc `--period` sai định dạng, thì vẫn ghi lệnh bình thường nhưng không dọn.
- **Đọc ngược có dung sai 1 ngày.** Với `sharehistory`, lệnh chạy lâu được ghi sau các lệnh mới hơn, nên
  timestamp trong file không tăng tuyệt đối. Chỉ dừng khi header đầu khối cũ hơn mốc period trừ 1 ngày.

## Quyết định đã xác nhận với người dùng (2026-11-01)

- Ngưỡng dọn là `WIB_PERIOD` cộng 1 ngày.
- Tăng `WIB_PERIOD` về sau không lấy lại được lệnh đã bị dọn: chấp nhận, ghi rõ trong README.
- Không sao lưu `history.tsv` trước lần dọn đầu tiên.

## Hệ quả người dùng cần biết

- Tăng `WIB_PERIOD` (ví dụ 30d lên 90d) không lấy lại được lệnh trong `history.tsv` đã bị dọn. Lệnh cũ vẫn
  còn trong `.zsh_history` nếu `SAVEHIST` đủ lớn, nhưng không có cwd và exit code.
- `wib query --period 90d` gọi tay cũng chỉ thấy `history.tsv` trong khoảng `WIB_PERIOD` đang cấu hình.

## Phases

| # | Phase | Effort | Phụ thuộc | Trạng thái |
|---|-------|--------|-----------|-----------|
| 1 | [Đọc `.zsh_history` từ cuối](phase-01-tail-read-zsh-history.md) | 3h | — | completed |
| 2 | [Tự dọn `history.tsv` theo WIB_PERIOD](phase-02-compact-wib-log.md) | 3h | — | completed |
| 3 | [Benchmark theo kích thước, cập nhật README](phase-03-bench-and-docs.md) | 1h | 1, 2 | completed |

Phase 1 và 2 sửa các file khác nhau (`zsh_history.rs` và `log_store.rs`), chỉ cùng chạm `main.rs` ở hai hàm
khác nhau (`load_entries` và `cmd_record`), nên có thể làm song song nếu cần.

## Acceptance criteria

- [x] p95 ở file 500.000 lệnh (400 ngày, `WIB_PERIOD=30d`) không quá 1,3 lần mốc chỉ chứa lệnh trong period:
      23,0 ms so với 21,3 ms (1,08 lần).
- [ ] p95 dưới 20 ms ở cùng file đó: **chưa đạt**, đo được 23,0 ms. File giả này có ~37.500 lệnh trong 30 ngày
      (1.250 lệnh/ngày), và phần còn lại là chi phí khớp chính các lệnh trong period, nằm ngoài phạm vi plan
      (không thêm cache hay index). Với mật độ thực tế hơn (500.000 lệnh trong 7 năm, ~6.000 lệnh trong period),
      p95 là 6,2 ms so với 40,8 ms trước thay đổi.
- [x] Bộ nhớ ở file 500.000 lệnh tương đương mốc trong period: 12,0 MB so với 11,8 MB.
- [x] Kết quả `wib query` giống hệt bản cũ: 205 lần so trên file giả và `~/.zsh_history` thật, 0 khác biệt.
- [x] Sau `wib record --period 30d`, `history.tsv` không còn dòng cũ hơn 31 ngày (test CLI).
- [x] 4 tiến trình × 200 lần `record` đồng thời trong lúc dọn: đủ 800 dòng, không dòng hỏng (test CLI).
- [x] `history.tsv` sau khi dọn vẫn quyền 0600 và giữ chủ sở hữu cũ.
- [x] `cargo test` (44), `cargo clippy --all-targets -- -D warnings`, `zsh tests/zle/run.zsh` (38) đều qua.
- [x] README đã bỏ đoạn `awk` dọn tay, cập nhật bảng số đo và hành vi giữ dữ liệu.

## Phát sinh khi review

- `WIB_PERIOD` cực lớn làm tràn phép nhân trong `parse_period`, khiến lần dọn xoá cả log. Đã sửa: period quá lớn
  bị từ chối, và `record` dùng phép trừ bão hoà.
- Dòng trong heredoc trông giống header có thể làm đọc ngược dừng sai chỗ. Đã sửa: bỏ qua dòng ngay sau một dòng
  kết thúc bằng `\`, hoặc khi không thấy được dòng trước.
- Không khoá được (filesystem không hỗ trợ, file khoá thuộc người khác) thì vẫn ghi lệnh nhưng không dọn.
- Dọn giữ nguyên chủ sở hữu của `history.tsv` (`fchown`), nên shell root dùng chung `$HOME` không làm hỏng quyền ghi.
- `scripts/gen-history.sh` được viết sớm ở phase 1 vì file giả đầu tiên (sinh tay) xếp ngược thứ tự thời gian.

## Non-goals

- Không sửa hay cắt `.zsh_history`.
- Không thêm daemon, cache hay index trên đĩa.
- Không đổi cách khớp, xếp hạng hay giao diện.
