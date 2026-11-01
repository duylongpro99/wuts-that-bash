---
phase: 3
title: "Benchmark theo kích thước, cập nhật README"
status: completed
priority: P2
effort: 1h
dependencies: [1, 2]
---

# Phase 3: Benchmark theo kích thước, cập nhật README

## Goal
Có số đo lặp lại được chứng minh thời gian tra không còn tăng theo tổng số lệnh, và README phản ánh hành vi mới.

## Files to Create / Modify
- Create: `scripts/gen-history.sh`: sinh file `.zsh_history` giả với N lệnh trải đều trên D ngày (zsh + awk,
  không phụ thuộc Python), seed cố định để lặp lại được.
- Modify: `README.md`, mục "Tốc độ và bộ nhớ", "Dữ liệu được lưu ở đâu", "Giới hạn đã biết", và mục "Phát triển".

## Tasks & Steps
1. `scripts/gen-history.sh <N> <days> <out>`: mỗi dòng `: <ts>:0;<lệnh>` với lệnh ghép từ vài từ thông dụng.
2. Đo bằng `scripts/bench.sh` (100 lần mỗi cỡ) và `/usr/bin/time -l` cho bộ nhớ, trước và sau thay đổi:
   - 10.000, 100.000, 500.000 lệnh trên 400 ngày, `--period 30d`.
   - Mốc so sánh: file chỉ chứa phần lệnh trong 30 ngày của file 500.000 lệnh.
3. README:
   - Thay bảng số đo bằng số mới; giải thích wib đọc `.zsh_history` từ cuối và dừng ở mốc period.
   - Bỏ đoạn `awk` dọn tay và câu "chưa tự dọn".
   - "Dữ liệu được lưu ở đâu": ghi rõ `history.tsv` chỉ giữ lệnh trong `WIB_PERIOD` (cộng tối đa 1 ngày);
     tăng `WIB_PERIOD` về sau không lấy lại được lệnh đã bị dọn.
   - "Giới hạn đã biết": bỏ dòng về `history.tsv` không tự dọn.
   - "Phát triển": thêm cách chạy `gen-history.sh` cùng `bench.sh`.

## Verification
- Đối chiếu số đo với acceptance criteria trong `plan.md` (p95 ở 500.000 lệnh không quá 1,3 lần mốc so sánh
  và dưới 20 ms; bộ nhớ chênh dưới 2 MB).
- Mọi số trong README khớp với kết quả vừa đo; liên kết nội bộ trong README còn đúng.

## Rollback
Hoàn tác README; script benchmark không ảnh hưởng runtime.
