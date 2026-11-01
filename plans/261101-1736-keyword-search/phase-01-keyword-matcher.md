---
phase: 1
title: "Matcher và xếp hạng từ khoá"
status: completed
priority: P2
effort: 2h
dependencies: []
---

# Phase 1: Matcher và xếp hạng từ khoá

## Goal
Có hàm xếp hạng nhận chuỗi từ khoá (`pull,master`) và trả các lệnh chứa đủ mọi từ khoá, theo thứ tự bất kỳ,
xếp theo chất lượng khớp rồi frecency.

## Files to Modify
- `src/matcher.rs`: thêm `KeywordPattern`.
- `src/rank.rs`: tách phần dedup + sort dùng chung; thêm `rank_keywords`.

## Tasks & Steps
1. `KeywordPattern::new(input)`: tách theo `,` và khoảng trắng, bỏ phần rỗng, fold từng từ khoá thành `Vec<char>`
   bằng `fold` hiện có. Smart case: chữ hoa ở bất kỳ từ khoá nào thì phân biệt hoa thường.
2. `KeywordPattern::is_empty()` và `matches(cmd) -> Option<Match>`:
   - Fold `cmd` thành `Vec<char>` một lần.
   - Với mỗi từ khoá, quét mọi vị trí xuất hiện; điểm vị trí: 3 nếu bắt đầu ở ranh giới (`is_boundary`) và
     kết thúc ở cuối lệnh hoặc trước một ký tự ranh giới, 2 nếu chỉ bắt đầu ở ranh giới, 1 nếu ở giữa từ.
     Lấy điểm cao nhất. Không xuất hiện thì trả `None`.
   - Trả `Match { tier: Tier::Prefix, score: tổng }` (một tier duy nhất trong chế độ này; ghi chú ngắn cạnh code).
   - Để biết "ký tự kết thúc từ", thêm hàm nhỏ dùng chung danh sách ký tự ranh giới với `is_boundary` (không
     chép lại danh sách).
3. `rank.rs`: đưa phần dedup, lọc, sort hiện có vào `fn rank_by<'a>(entries, now, limit, keep: impl Fn(&str) -> bool,
   matcher: impl Fn(&str) -> Option<Match>) -> Vec<&'a str>`. `rank` giữ chữ ký cũ và gọi `rank_by` với điều kiện
   `cmd != typed`.
4. `pub fn rank_keywords(entries, input, now, limit)`: dùng `KeywordPattern`, loại lệnh có từ đầu tiên là `wib`
   (`cmd == "wib"` hoặc bắt đầu bằng `wib` + khoảng trắng).

## Tests (`cargo test`)
- `matcher.rs`: `pull` khớp `git pull origin master`; `pull,master`, `master pull`, `master, pull` khớp;
  `pull,main` không khớp; `8080` khớp `-p8080:80`; `,` và `  ` rỗng → `is_empty`; smart case.
- Điểm: `pull` trong `git pull` > `pull` trong `git pull-request` > `ull` trong `git pull`.
- `rank.rs`: `rank_keywords` loại `wib pull,master` cũ; thứ tự theo điểm rồi frecency; `limit` được tôn trọng.
- Mọi test cũ của `matcher.rs` và `rank.rs` qua mà không sửa.

## Verification
`cargo test`, `cargo clippy --all-targets -- -D warnings`.

## Risk
- Lệnh rất dài với từ khoá xuất hiện nhiều lần: quét ngây thơ là O(độ dài lệnh × độ dài từ khoá) mỗi từ khoá,
  tương đương chi phí fuzzy hiện tại. Thêm test thời gian giống `long_commands_stay_fast`.

## Rollback
Xoá `KeywordPattern` và `rank_keywords`; `rank` giữ hành vi cũ.
