---
phase: 2
title: "Tự dọn history.tsv theo WIB_PERIOD"
status: completed
priority: P2
effort: 3h
dependencies: []
---

# Phase 2: Tự dọn `history.tsv` theo `WIB_PERIOD`

## Goal
`history.tsv` chỉ giữ lệnh trong khoảng `WIB_PERIOD` cộng tối đa 1 ngày, nên kích thước file không tăng theo
thời gian sử dụng. Không mất dòng khi nhiều shell ghi cùng lúc.

## Files to Modify
- `src/log_store.rs`: khoá chung, dọn file.
- `src/main.rs`: `cmd_record` nhận thêm `--period`, gọi dọn sau khi ghi.
- `shell/wib.zsh`: `_wib_precmd` truyền `--period "${WIB_PERIOD:-30d}"`.
- `tests/cli.rs`: test CLI cho dọn file và ghi đồng thời.

## Tasks & Steps
1. `log_store.rs`: thêm hàm mở `history.lock` (quyền 0600, trong thư mục 0700 sẵn có) và giữ khoá độc quyền
   bằng `File::lock()`. Khoá tự nhả khi file bị đóng.
2. `append` lấy khoá trước khi ghi. Giữ nguyên logic sửa dòng bị cắt dở và siết quyền 0600.
3. Thêm `pub fn compact_if_stale(dir, cutoff) -> io::Result<()>`, gọi khi đang giữ khoá:
   - Đọc tối đa 4 KB đầu file, parse dòng đầu. Nếu `ts >= cutoff - 86_400` thì thoát ngay (đường đi thường gặp).
   - Ngược lại đọc cả file, giữ các dòng có `ts >= cutoff` **ở dạng nguyên văn** (không parse rồi format lại,
     để không đổi escape), bỏ dòng hỏng.
   - Ghi vào `history.tsv.tmp` với quyền 0600, `sync_all`, rồi `rename` đè `history.tsv`.
   - Nếu ghi hoặc rename lỗi: xoá file tmp, giữ nguyên file cũ, trả lỗi.
4. `main.rs::cmd_record`: thêm `--period` vào danh sách flag. Sau khi ghi thành công, nếu có `--period` hợp lệ
   thì gọi `compact_if_stale` trong cùng lần giữ khoá. Lỗi dọn bị bỏ qua (lệnh đã được ghi, lần sau dọn lại).
   `--period` sai định dạng không làm `record` thất bại.
5. `shell/wib.zsh::_wib_precmd`: thêm `--period "${WIB_PERIOD:-30d}"` vào lệnh `record`.
6. Cập nhật USAGE trong `main.rs` cho `record`.

## Tests
- Unit (`log_store.rs`): dọn bỏ dòng cũ, giữ dòng mới nguyên văn (kể cả dòng có `\t`, `\n` đã escape); dòng đầu
  còn mới thì file không bị viết lại (so mtime hoặc inode); file sau dọn có quyền 0600; không còn file tmp.
- CLI (`tests/cli.rs`):
  - Ghi 3 lệnh với `--ts` 40 ngày trước, rồi 1 lệnh hiện tại với `--period 30d`: file chỉ còn lệnh mới.
  - `record` không có `--period` thì không dọn.
  - `record --period 7x` vẫn ghi lệnh, exit 0, không dọn.
  - 4 tiến trình, mỗi tiến trình ghi 200 lệnh với `--period 30d`, file có sẵn dòng cũ để buộc dọn: cuối cùng
    có đúng 800 dòng mới, mỗi dòng parse được.
- `zsh tests/zle/run.zsh`: vẫn qua.

## Verification
- `cargo test`, `cargo clippy -- -D warnings`, `zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`.
- Thử thật: đặt `WIB_PERIOD=1h`, chạy vài lệnh, xem `history.tsv` sau một lệnh tiếp theo.

## Risk
- Người dùng tăng `WIB_PERIOD` sau khi đã dọn thì không lấy lại được dữ liệu cũ của `history.tsv`.
  Đây là đánh đổi đã được chấp nhận; ghi rõ trong README.
- Một shell cũ (đã nạp script trước khi cập nhật) gọi `record` không có `--period`: vẫn ghi đúng, chỉ không dọn.

## Rollback
Bỏ lời gọi `compact_if_stale` trong `cmd_record`. Khoá trong `append` có thể giữ lại vì vô hại.
