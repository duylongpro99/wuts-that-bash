---
phase: 3
title: "Rust: match, xếp hạng, CLI query"
status: pending
priority: P1
effort: 5h
dependencies: [2]
---

# Phase 3: Rust — match, xếp hạng, CLI query

## Goal
`wib query --period 30d --limit 8 -- <input>` in ra tối đa N lệnh, mỗi dòng một lệnh, đã dedup và xếp hạng: khớp tiền tố trước, khớp mờ sau.

## Files to Create / Modify
- Create: `src/matcher.rs`, `src/rank.rs`
- Modify: `src/main.rs` (thêm subcommand `query`)

## Tasks & Steps
1. Dedup theo `cmd` (so khớp chính xác sau khi trim), gom `count` và `last_ts`.
2. `matcher.rs` dùng smart-case (input toàn chữ thường thì không phân biệt hoa thường):
   - Tier 0: `cmd` bắt đầu bằng input.
   - Tier 1: subsequence. Cộng điểm khi ký tự khớp ở đầu từ (sau space, `-`, `/`, `_`, `.`), trừ điểm theo khoảng cách. `dcup` phải khớp `docker compose up` với điểm cao.
   - Không khớp thì loại.
3. `rank.rs`: frecency = `count` × trọng số độ mới (`<1h`: 4, `<1d`: 2, `<7d`: 1, còn lại: 0.5). Sắp xếp theo tier, rồi match score giảm dần, frecency giảm dần, `last_ts` giảm dần.
4. Loại lệnh giống hệt input, để không gợi ý lại chính thứ đang gõ.
5. Hợp đồng output: mỗi dòng một lệnh; ký tự xuống dòng trong lệnh nhiều dòng escape thành `\n` (và `\` thành `\\`) để zsh tách theo dòng an toàn. Phía zsh unescape khi điền BUFFER. Ghi hợp đồng này trong doc comment của `src/main.rs`.
6. Giữ nhanh: đọc file bằng `std::fs::read` một lần, không regex, không cấp phát thừa trong vòng match.

## Verification
- `cargo test` phủ: `dcup` khớp `docker compose up`; tiền tố xếp trên fuzzy; dedup; limit; smart-case; input 1 ký tự vẫn trả kết quả (ngưỡng 2 ký tự do zsh xử lý).
- Chạy tay với history thật: `time ./target/release/wib query -- doc`.
