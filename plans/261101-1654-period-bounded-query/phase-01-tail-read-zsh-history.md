---
phase: 1
title: "Đọc .zsh_history từ cuối"
status: completed
priority: P2
effort: 3h
dependencies: []
---

# Phase 1: Đọc `.zsh_history` từ cuối

## Goal
`wib query` chỉ đọc phần cuối của `.zsh_history` chứa các lệnh trong period, nên thời gian đọc và bộ nhớ
không phụ thuộc vào phần cũ của file.

## Files to Modify
- `src/zsh_history.rs`: thêm hàm đọc từ cuối, giữ nguyên `parse_since` làm bộ parse.
- `src/main.rs`: `load_entries` gọi hàm mới thay cho `std::fs::read` + `parse_since`.

## Tasks & Steps
1. Thêm `pub fn read_since(path: &Path, cutoff: i64) -> io::Result<Vec<Entry>>` và một hàm nội bộ nhận
   kích thước khối ban đầu để test được ranh giới khối với khối nhỏ.
2. Thuật toán:
   - Mở file, lấy độ dài. Nếu độ dài không quá khối ban đầu (256 KB) thì đọc hết như cũ.
   - Ngược lại đọc khối cuối `[len - chunk, len)`. Bỏ phần trước dấu `\n` đầu tiên (dòng bị cắt dở).
   - Tìm dòng header đầu tiên trong khối (dòng khớp `parse_header`). Nếu không có, hoặc header đó có
     `ts >= cutoff - 86_400`, thì nhân đôi `chunk` và đọc lại; dừng khi `chunk` phủ cả file.
   - Khi header đầu khối cũ hơn `cutoff - 86_400`: gọi `parse_since(&bytes[vị_trí_header..], cutoff)`.
3. Byte `\n` không bao giờ là byte thứ hai của cặp metafied (zsh chỉ metafy byte 0x00 và 0x83–0x9f, mã hoá
   thành `b ^ 0x20`), nên cắt tại `\n` không làm hỏng ký tự non-ASCII. Ghi chú ngắn điều này cạnh code.
4. Dòng tiếp nối của lệnh nhiều dòng có thể trông giống header. Vì chỉ bắt đầu parse từ header cũ hơn mốc
   trừ 1 ngày, dòng giả header (nếu có) chỉ ảnh hưởng tới lệnh đã ngoài period. Giữ hành vi này, không thêm
   xử lý riêng.
5. `main.rs::load_entries`: thay `std::fs::read(zsh_history_path())` + `parse_since` bằng `read_since`.
   Lỗi đọc file vẫn trả danh sách rỗng như hiện tại.

## Tests (`cargo test`)
- Kết quả `read_since` với khối nhỏ (ví dụ 64 byte) giống hệt `parse_since(toàn bộ file)` trên:
  `tests/fixtures/zsh_history_sample`, một file có lệnh nhiều dòng nằm vắt qua ranh giới khối, một file có
  ký tự non-ASCII đã metafied, một file có timestamp lệch thứ tự trong vòng 1 ngày.
- File rỗng, file không có header nào, file nhỏ hơn khối ban đầu, `cutoff` cũ hơn mọi lệnh (phải đọc hết file).
- Test CLI hiện có trong `tests/cli.rs` vẫn qua mà không sửa.

## Verification
- `cargo test`, `cargo clippy -- -D warnings`.
- So kết quả `wib query -0` trên `~/.zsh_history` thật trước và sau thay đổi với khoảng 50 input ngẫu nhiên:
  phải giống hệt nhau.

## Risk
- Timestamp lệch thứ tự hơn 1 ngày (một lệnh chạy liền hơn 1 ngày với `sharehistory`) có thể làm sót lệnh đó.
  Lệnh như vậy hiếm và vẫn xuất hiện trong `history.tsv` nếu shell có nạp wib.

## Rollback
Trả `load_entries` về dùng `std::fs::read` + `parse_since`; `parse_since` không bị đổi.
