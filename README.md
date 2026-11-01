# wib — what is that bash?

Gợi ý lại các lệnh bạn đã dùng gần đây ngay khi gõ trên zsh. Gõ từ 2 ký tự trở lên, một danh sách
lệnh khớp (trong 30 ngày gần nhất, mặc định) hiện ngay dưới prompt và lọc lại theo từng phím.

```text
PROMPT> gi
› git push origin master
  git status
  git commit -m "fix bug"
```

- Khớp tiền tố trước, rồi khớp mờ theo chữ cái đầu (`dcup` → `docker compose up`).
- Xếp hạng theo số lần dùng và độ gần đây; lệnh trùng chỉ hiện một lần.
- Một binary Rust nhỏ, không daemon; mỗi lần tra cứu mất khoảng 5–8ms trên ~10k lệnh.
- Dữ liệu chỉ nằm trên máy bạn.

## Cài đặt

Cần zsh ≥ 5.3 và Rust toolchain.

```sh
cargo install --path .
```

Thêm vào **cuối** `~/.zshrc` (sau oh-my-zsh và các plugin khác):

```zsh
eval "$(wib init zsh)"
```

Mở terminal mới là dùng được. Lịch sử cũ trong `~/.zsh_history` có tác dụng ngay từ đầu.

## Phím tắt

Danh sách có hai chế độ: **đang gõ** (danh sách hiện, chưa chọn dòng nào) và **đang chọn** (một dòng được tô sáng).

| Phím | Khi đang gõ | Khi đang chọn |
|------|-------------|---------------|
| ↓ | Bắt đầu chọn, từ dòng đầu | Xuống dòng dưới (cuối thì quay về đầu) |
| ↑ | History của zsh như cũ | Lên dòng trên (dòng đầu thì quay về gõ) |
| Tab | Tab completion của zsh như cũ | Điền cả lệnh, chưa chạy |
| Shift+Tab | Điền từng từ của dòng đầu danh sách | Điền từng từ của dòng đang chọn |
| Space | Gõ dấu cách | Quay về gõ, không thêm dấu cách, danh sách vẫn hiện |
| Enter | Chạy dòng đang gõ | Điền lệnh đang chọn và chạy luôn |
| Esc / Ctrl-G | Ẩn danh sách đến khi bạn gõ tiếp | Như bên trái |

**Điền từng từ:** đang gõ `gi`, chọn `git push origin master`, bấm Shift+Tab bốn lần sẽ lần lượt
được `git` → `git push` → `git push origin` → `git push origin master`. Chuỗi trong ngoặc như
`"fix bug"` được điền trong một lần bấm.

Cmd+Tab không dùng được vì macOS giữ tổ hợp này để chuyển ứng dụng, nên điền từng từ dùng Shift+Tab.

Khi danh sách không hiện (chưa đủ ký tự, đã ẩn, hoặc đang duyệt history bằng ↑), mọi phím giữ đúng
hành vi cũ, kể cả các phím oh-my-zsh gán (ví dụ Shift+Tab lùi trong menu completion). Các phím ↑, Tab,
Space, Enter chỉ đổi nghĩa khi đang chọn một dòng, nên Ctrl-R, completion và history không bị ảnh hưởng.

**Esc hơi trễ?** zsh chờ `KEYTIMEOUT` (mặc định 0.4s) sau Esc để phân biệt với phím mũi tên. Có thể đặt
`KEYTIMEOUT=1` cho Esc phản hồi ngay, đổi lại kiểu bấm Esc rồi mới bấm phím (thay cho Option/Meta) sẽ không
còn hoạt động. Ctrl-G luôn ẩn danh sách ngay lập tức.

**vi mode** (`bindkey -v`): Esc vừa ẩn danh sách vừa chuyển sang command mode như bình thường.

## Cấu hình

Đặt biến môi trường trong `~/.zshrc`:

| Biến | Mặc định | Ý nghĩa |
|------|----------|---------|
| `WIB_PERIOD` | `30d` | Khoảng thời gian lấy lệnh: `12h`, `7d`, `2w`, `30d`, … |
| `WIB_LIMIT` | `8` | Số dòng tối đa trong danh sách |
| `WIB_MIN_CHARS` | `2` | Số ký tự tối thiểu trước khi hiện danh sách |
| `WIB_ZSH_HISTORY` | `$HISTFILE` | File history của zsh |
| `WIB_DATA_DIR` | `~/.local/share/wib` | Nơi lưu log riêng của wib |

## Dữ liệu được lưu ở đâu

- wib đọc `~/.zsh_history` (cần `setopt EXTENDED_HISTORY` để có thời điểm chạy lệnh; oh-my-zsh bật sẵn).
- Mỗi lệnh bạn chạy được ghi thêm vào `~/.local/share/wib/history.tsv` kèm thư mục và exit code.
  Log này không bị giới hạn bởi `SAVEHIST`.
- Lệnh bắt đầu bằng dấu cách và lệnh khớp `HISTORY_IGNORE` không được ghi. File log chỉ chủ tài khoản
  đọc được (quyền 0600).
- **Giới hạn:** lệnh bị loại bằng hàm `zshaddhistory` tự viết vẫn được wib ghi lại, vì zsh không cho biết
  hàm đó đã loại lệnh nào. Nếu bạn lọc lệnh nhạy cảm theo cách này, hãy chuyển sang `HISTORY_IGNORE`.

## Dùng cùng plugin khác

- **zsh-autosuggestions:** gợi ý xám vẫn hiện; bấm → chỉ nhận gợi ý đó, danh sách của wib không bị dán vào dòng lệnh.
- **powerlevel10k / oh-my-zsh:** đặt dòng `eval` sau khi các plugin đã nạp để wib ghi nhớ đúng phím gốc.

## Giới hạn đã biết

- Bấm Ctrl-C khi danh sách đang hiện sẽ để lại danh sách trong scrollback (giống danh sách completion của zsh).
- Ngay sau khi gọi lại một lệnh bằng ↑/↓ hoặc Ctrl-R, danh sách không hiện để ↓ vẫn duyệt history; gõ thêm ký tự là danh sách hiện lại.

## Dùng từ dòng lệnh

```sh
wib query --period 7d -- dcup     # in các lệnh khớp, mỗi dòng một lệnh
wib query -0 -- git               # phân tách bằng NUL, giữ nguyên lệnh nhiều dòng
```

## Gỡ cài đặt

Xoá dòng `eval "$(wib init zsh)"` khỏi `~/.zshrc`, rồi:

```sh
cargo uninstall wib
rm -rf ~/.local/share/wib
```

## Phát triển

```sh
cargo test                                   # unit + CLI tests
cargo install --path . --root .bin
zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"    # lái zsh thật qua zpty: phím, Ctrl-R, vi mode, autosuggestions
zsh scripts/bench.sh "$PWD/.bin/bin/wib"     # đo p50/p95 trên bản copy của ~/.zsh_history
```
