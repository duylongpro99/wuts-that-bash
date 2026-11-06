# Research: các vấn đề thường gặp khi gõ lệnh trên terminal → đề xuất tính năng tiếp theo cho wib

Ngày: 2026-11-05 · Phạm vi: web research + đối chiếu với README và `src/` hiện tại (v0.1.0).

## Kết luận

wib đã giải tốt bài toán "nhớ lại lệnh đã chạy" (gợi ý khi gõ, fuzzy theo đầu từ, xếp hạng theo
project, đẩy lệnh lỗi xuống dưới, `forget`). Các nỗi đau lớn còn lại mà người dùng hay nhắc tới và wib
có thể giải mà vẫn giữ triết lý local, không daemon:

1. **Bí mật (token, mật khẩu) lọt vào history**: wib có thể phát hiện và cảnh báo ngay khi ghi log.
2. **Lệnh dài cần sửa vài tham số**: biến lệnh cũ thành template có placeholder để nhảy giữa các tham số.
3. **Gõ sai lệnh**: wib đã biết lệnh nào từng thất bại, nên có thể gợi ý bản đúng mà bạn từng chạy thành công.
4. **Chỉ hỗ trợ zsh**: bash và fish là rào cản cài đặt lớn nhất.
5. **History không chia sẻ giữa các tab và bị giới hạn**: log của wib vốn đã chung giữa các tab, có thể nói rõ và mở rộng thêm.

## Bằng chứng tìm được

| Nỗi đau | Nguồn | Độ tin cậy |
|---|---|---|
| History là nhóm bực bội lớn thứ hai (42 phiếu): không chia sẻ giữa tab (16), giới hạn quá ngắn, mất khi terminal crash, không biết cách tìm, lệnh gõ sai bị kẹt trong history | Khảo sát của Julia Evans, 2/2025 (người trả lời đa số dùng terminal trên 21 năm) | Cao |
| Quên các lệnh phức tạp, ít dùng; alias khó tìm, khó tham số hoá, khó chia sẻ | Blog Warp 2022 (vendor) | Trung bình |
| Cách giải phổ biến hiện nay: bấm ↑ liên tục, `history \| grep`, copy từ ghi chú; history trộn lẫn mọi project gây nhiễu | Bài viết của developer (dev.to, peerlist) | Trung bình |
| API key, bearer token, connection string bị ghi vào `.zsh_history`; có tool riêng (shg) quét history tìm secret | Blog vrypan 7/2026, Kunal Ganglani 2026, Acunetix/Invicti | Cao |
| Đối thủ cạnh tranh bằng: lọc theo thư mục và exit status (McFly, hishtory), lưu thời lượng chạy (hishtory), lệnh nhiều dòng và đồng bộ máy (Atuin), MCP server cho AI agent (Suvadu) | Các bài so sánh, alternativeto | Trung bình |
| Atuin: người dùng muốn ↑ lọc theo thư mục còn Ctrl-R tìm toàn cục; hỗ trợ bash qua ble.sh cần nhiều công sức | Atuin issue #51, PR #2234, docs | Cao |
| McFly bị phàn nàn lag khi history khoảng 6K bản ghi | Bài review dev.to | Thấp (một nguồn) |
| zsh-autosuggestions: phần lớn lỗi đến từ xung đột plugin, thứ tự `source`, shell option (`KSH_ARRAYS`), màu sắc theme | GitHub issues #14, #483, #756 | Cao |
| thefuck nổi tiếng vì sửa lệnh gõ sai sau khi chạy (typo, `git main/master`, `cd` sai) | PyPI, docs | Cao |
| Cheatsheet có tham số (navi, pet): placeholder để điền giá trị, gợi ý giá trị động | navi docs, Keyhole | Cao |

## Đề xuất tính năng, xếp theo ưu tiên

### P1: Cảnh báo secret khi ghi log (`wib` secret guard)
- **Vấn đề:** lệnh chứa token/mật khẩu bị lưu vào history và sau đó được wib gợi ý lại trên màn hình (có thể lộ khi chia sẻ màn hình hoặc demo).
- **Đề xuất:** trong `wib record` (`src/log_store.rs` `should_record`), nhận diện các mẫu phổ biến (`*_TOKEN=`, `*_KEY=`, `Authorization: Bearer`, `--password`, URL có `user:pass@`, tiền tố như `ghp_`, `sk-`, `AKIA`). Lệnh khớp sẽ không được ghi vào log, không được gợi ý, và in một dòng nhắc kèm cách xoá khỏi `~/.zsh_history`. Thêm `wib audit` để quét history hiện có.
- **Lý do ưu tiên:** đúng với định vị "private by design", chi phí thấp, khác biệt rõ với zsh-autosuggestions.

### P1: Template có placeholder khi điền lệnh
- **Vấn đề:** người dùng chạy lại cùng một lệnh dài và chỉ đổi tag, port, tên pod.
- **Đề xuất:** khi điền một lệnh từ danh sách, bấm một phím (ví dụ Ctrl-]) để nhảy con trỏ tới token "biến đổi" tiếp theo, tức là các token khác nhau giữa những lần chạy cùng dạng lệnh (`kubectl logs <pod>`, `docker run -p <port>`). Tính bằng cách so sánh các lệnh cùng tiền tố trong log, không cần người dùng viết cheatsheet.
- **Lý do:** nối tiếp tự nhiên của Shift+Tab điền từng từ; giải đúng nỗi đau "alias khó tham số hoá" mà không cần file cấu hình như navi.

### P2: Gợi ý sửa lệnh lỗi dựa trên history của chính bạn
- **Vấn đề:** gõ sai (`gti status`, `git push origin main` trên repo dùng `master`).
- **Đề xuất:** sau một lệnh exit khác 0 và lệnh đó chưa từng thành công, `precmd` hiển thị một dòng "Ý bạn là: `git status`?" lấy từ lệnh thành công gần nhất có khoảng cách chỉnh sửa nhỏ, ưu tiên cùng project. ↓ Enter để chạy. Dữ liệu `exit` và `cwd` đã có sẵn trong `Entry`.
- **Lý do:** thefuck cần quy tắc viết tay và chạy chậm (Python); wib có sẵn dữ liệu thực tế của người dùng.

### P2: Hỗ trợ bash và fish
- **Vấn đề:** README ghi rõ "Not for (yet): bash and fish users". Đây là rào cản cài đặt lớn nhất.
- **Đề xuất:** làm fish trước (fish có cơ chế binding và hook dễ hơn bash), sau đó bash qua ble.sh. Lõi Rust (`query`, `record`) giữ nguyên, chỉ cần thêm `wib init fish` và đọc định dạng `fish_history`.
- **Rủi ro:** bash không có cơ chế vẽ danh sách dưới prompt như ZLE; Atuin cũng phải dựa vào ble.sh.

### P3: Lưu thời lượng chạy và hiển thị thông tin ngữ cảnh
- Ghi thêm thời gian chạy vào `history.tsv` (cần đổi định dạng log có tương thích ngược), để có thể đánh dấu lệnh chạy lâu và tránh vô tình chạy lại lệnh nặng.
- Có thể hiển thị mờ bên phải mỗi dòng gợi ý: "3 ngày trước · ~/api".

### P3: Lệnh nhiều dòng trong danh sách gợi ý
- `wib query -0` đã giữ được lệnh nhiều dòng; cần xác minh widget hiển thị và điền đúng (Atuin được chọn hơn McFly chủ yếu vì điểm này).

### P3: MCP server hoặc `wib query --json` cho AI agent
- Suvadu đã dùng điểm này làm khác biệt. Cho phép coding agent hỏi "lệnh nào dùng để chạy test trong project này" từ history local. Nên cân nhắc kỹ về quyền riêng tư; chỉ làm sau khi có secret guard.

### Việc nhỏ, ít rủi ro
- **`wib doctor`:** kiểm tra thứ tự dòng `eval` trong `.zshrc`, `EXTENDED_HISTORY`, `KEYTIMEOUT`, xung đột binding. Phần lớn issue của zsh-autosuggestions là cấu hình, nên tool tự chẩn đoán sẽ giảm số issue phải hỗ trợ.
- **`wib unforget`:** README nói hiện phải sửa file bằng tay.
- **Homebrew formula:** đang nằm trong "Known limitations".

## Không nên làm (ít nhất lúc này)
- **Đồng bộ nhiều máy:** Atuin đã làm tốt, và tính năng này trái với định vị "100% local, không cloud".
- **Sinh lệnh bằng AI từ ngôn ngữ tự nhiên:** cần mạng hoặc model, trái với "no daemon, no telemetry", và đã có nhiều tool khác làm.

## Câu hỏi còn bỏ ngỏ
- Ưu tiên nào quan trọng hơn với bạn: mở rộng người dùng (bash/fish, Homebrew) hay đào sâu giá trị cho người dùng zsh (secret guard, template)?
- Bạn có dữ liệu feedback từ đợt ra mắt Product Hunt không? Nếu có thì nên dùng để kiểm chứng bảng ưu tiên trên.
- Chưa có số liệu xếp hạng issue theo reaction của Atuin; nếu cần, có thể lấy bằng `gh` để bổ sung.

## Nguồn
- https://jvns.ca/blog/2025/02/05/some-terminal-frustrations/
- https://www.warp.dev/blog/using-workflows-and-commands-dev-to-remember-commands-we-often-forget
- https://dev.to/itsmimakhtar/ever-caught-yourself-pressing-arrow-up-key-over-and-over-3n80
- https://blog.vrypan.net/2026/07/13/260713-shg-the-shell-guard/
- https://www.kunalganglani.com/blog/prevent-api-key-leaks-shell-history
- https://www.acunetix.com/vulnerabilities/360/zsh-history-file-detected/
- https://dev.to/nickytonline/i-switched-shell-history-tools-heres-why-m6h
- https://suvadu-web.pages.dev/blog/suvadu-vs-atuin-mcfly-hstr
- https://alternativeto.net/software/ddworken-hishtory
- https://github.com/atuinsh/atuin/issues/51
- https://github.com/atuinsh/atuin/pull/2234
- https://docs.atuin.sh/configuration/key-binding
- https://github.com/zsh-users/zsh-autosuggestions/issues/14
- https://www.github.com/zsh-users/zsh-autosuggestions/issues/483
- https://www.github.com/zsh-users/zsh-autosuggestions/issues/756
- https://pypi.org/project/thefuck
- https://keyholesoftware.com/?p=133851
