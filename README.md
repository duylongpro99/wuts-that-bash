# wib — what is that bash?

Gợi ý lại các lệnh bạn đã dùng gần đây ngay khi gõ trên zsh. Gõ từ 2 ký tự trở lên, một danh sách
lệnh khớp (trong 30 ngày gần nhất, mặc định) hiện ngay dưới prompt và lọc lại theo từng phím.

```text
PROMPT> gi
› git push origin master
  git status
  git commit -m "fix bug"
```

- Khớp tiền tố trước, rồi khớp mờ theo đầu từ (`dcup` → `docker compose up`, `gi st` → `git status`).
  Mỗi đoạn bạn gõ phải là phần đầu của một từ trong lệnh; chữ cái nằm rải rác giữa các từ không được tính.
- Xếp hạng theo số lần dùng và độ gần đây; lệnh trùng chỉ hiện một lần.
- Một binary Rust nhỏ, không daemon; mỗi lần tra cứu mất khoảng 5–8ms trên ~10k lệnh.
- Dữ liệu chỉ nằm trên máy bạn.

## Vì sao chọn wib

- **Gợi ý tự hiện, không cần gọi.** Ctrl-R, fzf hay atuin bắt bạn nhớ ra là cần tìm rồi bấm phím mở
  màn hình tìm kiếm. wib hiện danh sách ngay khi bạn gõ và lọc lại sau từng phím.
- **Thấy nhiều lựa chọn cùng lúc.** zsh-autosuggestions chỉ đưa một gợi ý xám; wib hiện tối đa 8 dòng
  (đổi được qua `WIB_LIMIT`), nên bạn nhận ra đúng lệnh khi nhìn thấy nó, kể cả khi không nhớ cách viết.
- **Gõ ít mà vẫn ra đúng lệnh.** Khớp theo đầu từ (`dcup`, `gi st`), và Shift+Tab điền từng từ để lấy
  lệnh cũ làm khung rồi sửa phần cuối.
- **Không phá thói quen đang có.** ↑, Tab, Enter, Ctrl-R giữ nguyên hành vi; phím chỉ đổi nghĩa khi bạn
  bấm ↓ để chọn một dòng. Chạy cùng oh-my-zsh, powerlevel10k, zsh-autosuggestions và vi mode. Lịch sử
  cũ trong `~/.zsh_history` dùng được ngay.
- **Nhanh, nhẹ, riêng tư.** Một binary, không daemon, không database server, không cloud hay telemetry.
  Log để quyền 0600 và bỏ qua lệnh khớp `HISTORY_IGNORE`. Gỡ ra chỉ cần xoá một dòng và một thư mục.

| | Cách gọi | Số gợi ý | Đồng bộ nhiều máy |
|---|---|---|---|
| **wib** | Tự hiện khi gõ | Nhiều dòng, ngay dưới prompt | Không, chỉ local |
| zsh-autosuggestions | Tự hiện khi gõ | Một gợi ý xám | Không |
| Ctrl-R / fzf | Bấm phím để mở | Danh sách tìm kiếm | Không |
| atuin | Bấm phím để mở | Danh sách tìm kiếm | Có |

## Cài đặt

Cần zsh ≥ 5.3 và Rust ≥ 1.89.

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
  Log này không bị giới hạn bởi `SAVEHIST`, nhưng chỉ giữ lệnh trong `WIB_PERIOD`: khi lệnh cũ nhất quá
  `WIB_PERIOD` thêm một ngày, wib bỏ các lệnh cũ hơn `WIB_PERIOD` (khoảng một lần mỗi ngày). Vì vậy tăng
  `WIB_PERIOD` về sau không lấy lại được các lệnh đã bị bỏ khỏi log này; lệnh cũ chỉ còn trong
  `~/.zsh_history` nếu `SAVEHIST` đủ lớn, và không có thư mục, exit code.
- wib không bao giờ sửa `~/.zsh_history`.
- Lệnh bắt đầu bằng dấu cách và lệnh khớp `HISTORY_IGNORE` không được ghi. File log chỉ chủ tài khoản
  đọc được (quyền 0600).
- **Giới hạn:** lệnh bị loại bằng hàm `zshaddhistory` tự viết vẫn được wib ghi lại, vì zsh không cho biết
  hàm đó đã loại lệnh nào. Nếu bạn lọc lệnh nhạy cảm theo cách này, hãy chuyển sang `HISTORY_IGNORE`.

## Tốc độ và bộ nhớ

**Vì sao nhanh**

- Mỗi lần buffer thay đổi, zsh gọi `wib query` một lần. Gõ lại cùng nội dung hoặc chỉ di chuyển trên
  danh sách thì không gọi lại.
- wib đọc thẳng file history rồi trả kết quả, không có bước khởi động hay nạp index. Binary build với
  LTO, không phụ thuộc thư viện ngoài.
- wib đọc `~/.zsh_history` từ cuối file lên và dừng khi gặp lệnh cũ hơn `WIB_PERIOD`, nên phần cũ của file
  không bao giờ được đọc. `history.tsv` tự dọn theo `WIB_PERIOD`. Thời gian tra vì thế phụ thuộc vào số lệnh
  trong `WIB_PERIOD`, không tăng theo số năm bạn dùng.
- Khi khớp, wib so tiền tố trước vì đây là trường hợp phổ biến. Phép khớp mờ chỉ chạy khi các chữ cái
  của input có xuất hiện theo đúng thứ tự trong lệnh, nên phần lớn lệnh bị loại sau một lượt quét.
  Chi phí chấm điểm tỉ lệ với độ dài input nhân độ dài lệnh, nên lệnh rất dài cũng không làm chậm.
- Ghi lệnh vào log (`wib record`) chạy nền sau khi lệnh kết thúc, nên không làm prompt hiện chậm.

**Vì sao không rò rỉ bộ nhớ**

wib không có daemon hay tiến trình chạy thường trực. Mỗi lần tra cứu là một tiến trình riêng, chạy xong
thì thoát và hệ điều hành thu hồi toàn bộ bộ nhớ. Vì vậy mở terminal nhiều ngày hay gõ hàng nghìn phím
cũng không làm bộ nhớ tăng dần. Shell zsh chỉ giữ tối đa `WIB_LIMIT` dòng gợi ý của lần tra gần nhất.

**Số đo** (`scripts/bench.sh`, macOS, `WIB_PERIOD=30d`, gồm cả thời gian khởi động tiến trình, 100 lần tra
mỗi file; file history giả sinh bằng `scripts/gen-history.sh`):

| File history | Lệnh trong 30 ngày | p95 | Bộ nhớ tối đa mỗi lần tra |
|---|---|---|---|
| 10.000 lệnh trong 400 ngày | ~750 | 5,4 ms | ~2 MB |
| 100.000 lệnh trong 400 ngày | ~7.500 | 8,8 ms | ~3,5 MB |
| 500.000 lệnh trong 7 năm (200 lệnh/ngày) | ~6.000 | 6,9 ms | ~3,3 MB |
| 500.000 lệnh trong 400 ngày (1.250 lệnh/ngày) | ~37.500 | 22 ms | ~12 MB |

Ba dòng cuối cho thấy thời gian tra đi theo số lệnh trong period: file 500.000 lệnh trải trên 7 năm còn
nhanh hơn file 100.000 lệnh trải trên 400 ngày. Chỉ khi bạn gõ hàng chục nghìn lệnh trong một period mới
thấy hơi trễ; khi đó hãy giảm `WIB_PERIOD`.

## Dùng cùng plugin khác

- **zsh-autosuggestions:** gợi ý xám vẫn hiện; bấm → chỉ nhận gợi ý đó, danh sách của wib không bị dán vào dòng lệnh.
- **powerlevel10k / oh-my-zsh:** đặt dòng `eval` sau khi các plugin đã nạp để wib ghi nhớ đúng phím gốc.

## Giới hạn đã biết

- Chỉ hỗ trợ zsh, chưa có bash hay fish.
- Cài đặt cần Rust toolchain; chưa có Homebrew hay binary dựng sẵn.
- Không đồng bộ lịch sử giữa các máy.
- wib coi `~/.zsh_history` được ghi theo thứ tự thời gian (cho phép lệch tới một ngày, như khi `sharehistory`
  ghi một lệnh chạy lâu sau các lệnh mới hơn). Nếu bạn ghép file history từ máy khác vào cuối, các lệnh mới
  hơn nằm phía trước đoạn ghép có thể không được gợi ý.
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
zsh scripts/gen-history.sh 500000 400 /tmp/h  # sinh file history giả: 500k lệnh trong 400 ngày
zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 100
```
