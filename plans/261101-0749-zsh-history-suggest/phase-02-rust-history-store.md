---
phase: 2
title: "Rust: parse history + log riêng"
status: pending
priority: P1
effort: 5h
dependencies: []
---

# Phase 2: Rust — parse history + log riêng

## Goal
Crate `wib` đọc được `~/.zsh_history` và log riêng thành danh sách `Entry { ts, cmd, cwd: Option, exit: Option }`, đồng thời ghi được entry mới vào log riêng.

## Files to Create / Modify
- Create: `Cargo.toml` (binary `wib`, edition 2021; release profile `lto = true`, `codegen-units = 1`; chỉ dùng std trừ khi có lý do rõ)
- Create: `src/main.rs` (dispatch subcommand thủ công, không cần clap)
- Create: `src/entry.rs`, `src/zsh_history.rs`, `src/log_store.rs`
- Create: `tests/fixtures/zsh_history_sample` (có lệnh nhiều dòng, ký tự non-ASCII đã metafied, dòng hỏng)
- Create: `.gitignore` (thư mục build của cargo)

## Tasks & Steps
1. `zsh_history.rs`: đọc bytes, unmetafy (0x83 + b → b ^ 0x20), tách entry theo dòng bắt đầu bằng `: <ts>:<dur>;`. Dòng kết thúc bằng `\` thì nối với dòng sau (lệnh nhiều dòng). Dòng không có header hoặc dòng hỏng thì bỏ qua, không panic.
2. `log_store.rs`: file `${XDG_DATA_HOME:-~/.local/share}/wib/history.tsv`, mỗi dòng `ts\texit\tcwd\tcmd`, escape `\\`, `\t`, `\n`. Ghi bằng `OpenOptions::append` với một lần `write_all`. Tự tạo thư mục nếu chưa có.
3. Subcommand `wib record --ts <unix> --exit <code> --cwd <path> -- <cmd>`: bỏ qua nếu cmd rỗng hoặc bắt đầu bằng dấu cách; trim khoảng trắng cuối.
4. Hàm `load_all(period)`: đọc log riêng để lấy `min_ts`; chỉ lấy entry `.zsh_history` có `ts < min_ts` (lấy hết nếu log rỗng); lọc `ts >= now - period`. Cho phép ghi đè đường dẫn qua `WIB_ZSH_HISTORY` và `WIB_DATA_DIR` để test.
5. Parse period: `12h`, `7d`, `2w`, `30d`. Sai định dạng thì báo lỗi rõ ràng và exit 2.

## Verification
- `cargo test` phủ: unmetafy, lệnh nhiều dòng, dòng hỏng, escape TSV round-trip, quy tắc không đếm trùng, lọc period, bỏ lệnh bắt đầu bằng dấu cách.
- `cargo clippy -- -D warnings` sạch.
