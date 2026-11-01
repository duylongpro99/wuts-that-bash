# Brainstorm: tìm lệnh theo từ khoá (`wib pull,master`)

## Hiện trạng (đã kiểm tra trên binary `.bin/bin/wib` với history giả)

| Input cho `wib query` | Kết quả |
|---|---|
| `pull` | `git pull origin master`, `docker pull nginx` |
| `pull master` | `git pull origin master` (khớp theo thứ tự) |
| `master pull` | không có gì (sai thứ tự) |
| `pull,master` | không có gì (dấu phẩy bị coi là ký tự cần khớp) |
| `wib pull` (chạy như lệnh) | `unknown command 'pull'`, exit 2 |

Gõ `wib pull` tại prompt cũng không ra gì trong danh sách live, vì buffer `wib pull` được khớp như lệnh bắt đầu bằng `wib`.

## Contract

**Outcome:** Người dùng chỉ nhớ vài từ trong lệnh cũ gõ `wib pull` hoặc `wib pull,master` (thứ tự bất kỳ)
và thấy các lệnh liên quan nhất, xếp hạng tốt nhất trước:
- Tại prompt: khi buffer bắt đầu bằng `wib `, danh sách live chuyển sang chế độ từ khoá; ↓ rồi Enter thay cả
  dòng bằng lệnh đã chọn và chạy; Tab điền lệnh; Shift+Tab điền từng từ như hiện nay.
- Chạy thẳng (Enter khi chưa chọn dòng, hoặc từ script): `wib pull,master` in danh sách ra stdout, mỗi dòng một lệnh.

**Quyết định của người dùng (2026-11-01):**
- Bề mặt: live list + CLI.
- Dấu cách tương đương dấu phẩy: `wib pull master` = `wib pull,master` = `wib master,pull`.

**Constraints:**
- Giữ nguyên hành vi khớp hiện tại cho mọi buffer không bắt đầu bằng `wib `.
- Không phá các subcommand `query`, `record`, `init`, `-h`, `-V`. Từ khoá trùng tên subcommand (ví dụ tìm chữ
  `init`) phải có đường thoát: `wib -- init` hoặc `wib query --keywords -- init`.
- Không daemon, không index; giữ ngân sách tốc độ hiện tại (p95 ~5–9 ms ở ~7.500 lệnh trong period).
- Phạm vi dữ liệu vẫn là `WIB_PERIOD`; kết quả tối đa `WIB_LIMIT`.
- Không gợi ý chính các lệnh `wib ...` đã chạy trước đó.

**Non-goals:** khớp mờ chính tả (typo), regex, toán tử OR/NOT, tìm theo thư mục hay exit code, tìm ngoài
`WIB_PERIOD`, giao diện chọn toàn màn hình kiểu fzf.

**Acceptance criteria:**
1. `wib pull`, `wib pull,master`, `wib master pull`, `wib 'master, pull'` đều trả `git pull origin master`; `docker pull nginx`
   chỉ có ở `wib pull`.
2. Mọi từ khoá phải xuất hiện (AND); thiếu một từ thì lệnh bị loại.
3. Lệnh có từ khoá khớp trọn một từ / đầu từ xếp trên lệnh chỉ khớp ở giữa từ; cùng mức thì theo frecency như hiện nay.
4. Gõ `wib pul` tại prompt hiện danh sách live; ↓ + Enter chạy đúng lệnh đã chọn (không chạy `wib ...`).
   Test zpty trong `tests/zle/run.zsh` phủ trường hợp này.
5. Buffer không có tiền tố `wib ` cho kết quả giống hệt trước (unit test matcher/rank cũ vẫn xanh).
6. `wib -- query` tìm chữ `query` thay vì gọi subcommand.
7. `scripts/bench.sh` không chậm hơn đáng kể so với số đo trong README.
8. README cập nhật mục dùng từ khoá.

## Lựa chọn cách khớp từng từ khoá

| Cách | Giả định chính | Hỏng đầu tiên khi |
|---|---|---|
| A. Chỉ đầu từ (dùng `is_boundary` hiện có) | Người dùng nhớ phần đầu của từ | Từ khoá nằm giữa token, ví dụ `8080` trong `-p8080:80` |
| B. Substring ở bất kỳ đâu | Từ khoá đủ dài để ít nhiễu | Từ khoá ngắn (`ma`) khớp tràn lan |
| **C. Substring, nhưng đầu từ / trọn từ xếp trên (khuyến nghị)** | Xếp hạng đủ đẩy nhiễu xuống dưới | Period có rất nhiều lệnh chứa từ khoá ngắn ở giữa từ; vẫn chỉ đẩy xuống, không lọc |

Khuyến nghị C: người dùng chủ động tìm nên ưu tiên không bỏ sót (lý do của tính năng là "không nhớ rõ"),
còn nhiễu được xử lý bằng xếp hạng. Điểm mỗi lệnh = tổng điểm từng từ khoá (trọn từ > đầu từ > giữa từ),
rồi frecency, rồi độ gần đây.

**Trade-offs:**
- A rẻ nhất và nhất quán với triết lý chống nhiễu của matcher hiện tại, nhưng bỏ sót đúng loại trường hợp
  "chỉ nhớ một mẩu".
- C tốn thêm một lượt `contains` cho mỗi từ khoá (rẻ, tuyến tính), không đổi cấu trúc dữ liệu.

**Better approaches:** none — hướng được chọn là hướng người dùng yêu cầu; lựa chọn C chỉ thêm tầng xếp hạng.

## Hình dạng thay đổi dự kiến

- `src/matcher.rs`: thêm `KeywordPattern` (tách theo `,` và khoảng trắng, smart case như `Pattern`).
- `src/rank.rs`: cho `rank` nhận matcher qua tham số hoặc enum, giữ dedup/frecency dùng chung.
- `src/main.rs`: lệnh không phải subcommand (hoặc sau `--`) → chế độ từ khoá in ra stdout; `query` nhận cờ chế độ
  từ khoá cho widget dùng `-0`.
- `shell/wib.zsh`: buffer khớp `wib *` → gọi chế độ từ khoá với phần sau `wib `; chọn dòng thì thay cả buffer.
- Tests: unit cho matcher/rank, `tests/cli.rs`, `tests/zle/run.zsh`.

## Câu hỏi còn mở

- Có loại các lệnh `wib ...` khỏi mọi kết quả (kể cả chế độ thường) hay chỉ trong chế độ từ khoá? Đề xuất: chỉ trong chế độ từ khoá.
