---
phase: 1
title: "Secret guard: phát hiện, không ghi, không gợi ý, nhắc"
status: completed
priority: P1
effort: "7h"
dependencies: []
---

# Phase 1: Secret guard

## Goal

Lệnh chứa secret không vào `history.tsv`, không bao giờ hiện trong gợi ý, không lộ qua tham số process của
wib, và người dùng được nhắc một lần trong đúng shell đã chạy lệnh đó.

## Files to Create / Modify

- Create: `src/secret.rs`
- Modify: `src/main.rs` (khai báo module; `cmd_record` đọc lệnh từ stdin với `--stdin`, gọi `secret::detect`,
  nhận `--notice-id`; tôn trọng `WIB_SECRET_GUARD=0`)
- Modify: `src/rank.rs` (lọc secret lười sau khi sort; đổi `is_ok`, `wib_args` thành `pub(crate)` cho phase 4–5)
- Modify: `src/log_store.rs` (`write_secret_notice`, dọn notice cũ; đổi `escape`, `unescape`, `lock`,
  `private_dir`, `open_regular` thành `pub(crate)`; tách `rewrite_private` từ `compact_if_stale`)
- Modify: `shell/wib.zsh` (helper `_wib_run`; record qua stdin; đọc notice của chính shell; `zshexit` dọn notice)
- Modify: `tests/cli.rs`, `tests/zle/run.zsh`

## Thiết kế `secret.rs`

- `pub fn detect_all(cmd: &str) -> Vec<Finding>` và `pub fn detect(cmd) -> Option<Finding>` (= phần tử đầu).
  `Finding { kind: Kind, span: Range<usize> }`, `span` là byte range của **giá trị** secret (char boundary).
- `Kind::label()` cho thông báo; `Kind::public_prefix()` trả tiền tố công khai (`ghp_`, `AKIA`, `sk_live_`, …)
  cho loại token có tiền tố đã biết, `None` cho mọi loại khác. Dùng ở phase 2 và 5 để che.
- Không dùng regex, chỉ quét ký tự; thoát sớm khi chuỗi không có `=`, `_`, `-`, `:` hay `@`.
- `WIB_SECRET_GUARD=0` (đã chốt) tắt lọc ở record, rank, slots và suggest-fix; `audit` vẫn dùng `detect_all`.

| Mẫu | Khớp | Không khớp (test bắt buộc) |
|---|---|---|
| Gán biến | Tên tách theo `_` có một phần là `TOKEN`, `SECRET`, `PASSWORD`, `PASSWD`, `APIKEY`, `CREDENTIALS`, hoặc cặp `API_KEY`, `ACCESS_KEY`, `PRIVATE_KEY` (`GITHUB_TOKEN=…`, `DB_PASSWORD=…`, `AWS_SECRET_ACCESS_KEY=…`) | `TOKENIZERS_PARALLELISM=false`, `PASSWORD_STORE_DIR=~/x`, `TOKEN="$(op read …)"`, `TOKEN=$GITHUB_TOKEN`, `SECRET_FILE=/run/s` |
| Giá trị bỏ qua | | Rỗng; bắt đầu bằng `$`, `"$`, `'$`, `~/`, `/`; `true`/`false`; số thuần ≤ 5 chữ số |
| Cờ mật khẩu | `--password=x`, `--password x`, `--token x`, `--secret x`, `--api-key x`, `--client-secret x` | `--password-file x`, `--token $T` |
| Cờ ngắn theo chương trình | `mysql -pS3cret` (dính liền), `sshpass -p pw`, `docker login -p pw`, `htpasswd -b f u pw` | `mysql -p` (hỏi mật khẩu), `docker run -p 8080:80`, `ssh -p 22` |
| User:pass | `curl -u user:pass`, `curl --user user:pass`, `wget --password x`, URL `scheme://user:pass@host` | `curl -u user`, `https://example.com/@user`, `git@github.com:org/repo` |
| Header | `Authorization: Bearer x`, `Authorization: Basic x`, `X-Api-Key: x` (không phân biệt hoa thường) | `Authorization: $AUTH` |
| Token có tiền tố | `ghp_ gho_ ghu_ ghs_ ghr_ github_pat_ glpat- xoxb- xoxp- sk_live_ rk_live_`, `sk-`+≥20, `AKIA`+16, `AIza`+35, ở đầu một từ | `task-sk-1234567890abcdefghij`, `AKIA` ít hơn 16 ký tự |
| JWT, private key | `eyJ….eyJ….…`, `-----BEGIN … PRIVATE KEY-----` | |

## Tasks & Steps

- [x] Trước khi sửa code, đo gốc: `cargo install --path . --root .bin`, `zsh scripts/gen-history.sh 100000 400 /tmp/h`,
       `zsh scripts/bench.sh "$PWD/.bin/bin/wib" /tmp/h 100` và thêm đối số `keywords`. Lưu p50/p95 vào
       `plans/261105-0735-zsh-ux-depth/reports/bench-before.txt` (phase 6 so sánh với số này).
- [x] `src/secret.rs` theo bảng trên; test đơn vị: mỗi ô "Khớp" và "Không khớp" một ca, lệnh có hai secret trả
       hai `Finding`, ký tự nhiều byte quanh secret.
- [x] `log_store`: đổi các helper sang `pub(crate)`. `rewrite_private(dir, name, bytes, _held: &File)` bắt buộc
       nhận guard của `lock` (không giữ khoá thì không gọi được), file tạm riêng `{name}.tmp`, giữ nguyên
       `create_new` + `fchown` + `rename` hiện có. `compact_if_stale` gọi lại helper này.
- [x] `cmd_record`: thêm `--stdin` (đọc toàn bộ stdin làm lệnh; cách truyền qua đối số giữ làm dự phòng) và
       `--notice-id <id>` (chỉ nhận chữ số). Khi guard bật và `detect` có kết quả: không `append`, ghi
       `<data>/secret-notice.<id>` (0600 qua `private_dir` + `open_regular`, nội dung chỉ là nhãn), và xoá các
       `secret-notice.*` cũ hơn một ngày.
- [x] `rank.rs`: giữ nguyên sort; sau khi sort, duyệt lần lượt và bỏ lệnh có secret cho tới khi đủ `limit`
       (gọi `detect` lười). Đầu ra giống hệt khi không có secret.
- [x] `wib.zsh`: helper `_wib_run <subcommand> [args…]` truyền `WIB_ZSH_HISTORY=${WIB_ZSH_HISTORY:-${HISTFILE:-…}}`,
       `WIB_DATA_DIR`, `WIB_PERIOD`, `WIB_SECRET_GUARD` (giá trị trong shell, kể cả chưa export) rồi chạy
       `"$_wib_bin"`. `_wib_fetch` và `_wib_precmd` dùng helper này. Record:
       `print -rn -- "$_wib_cmd" | _wib_run record --stdin --notice-id $$ --ts … --exit … --cwd "$PWD" --period … &!`.
- [x] `wib.zsh`: `zmodload -F zsh/files b:zf_rm`. `_wib_secret_notice` kiểm tra `[[ -e $dir/secret-notice.$$ ]]`
       (thư mục tính giống `log_store::data_dir` tại `src/log_store.rs:23`, từ cùng các biến mà `_wib_run`
       truyền), đọc nhãn, `zf_rm -f`, rồi `zle -M "wib: the last command looks like it contains a <label>; wib did
       not save it. It is still in your zsh history: run 'wib audit'."`. Gọi trong `_wib_line_init` và
       `_wib_pre_redraw`. Hook `zshexit` xoá file của `$$`.
- [x] `tests/cli.rs`: record qua `--stdin` lệnh có secret → log không có dòng nào, `secret-notice.<id>` tồn tại mode
       0600; `WIB_SECRET_GUARD=0` → được ghi; `query` không trả lệnh có secret dù có trong `.zsh_history`; lệnh
       thường vẫn được ghi; notice cũ hơn một ngày bị dọn.
- [x] `tests/zle/run.zsh`: **không đổi** `HISTORY_IGNORE='(*TOKEN*)'` ở dòng 40. Dùng secret không khớp `*TOKEN*`,
       ví dụ `curl -s -H 'Authorization: Bearer <chuỗi giả 40 ký tự>' localhost:1` → notice hiện ở prompt kế tiếp
       hoặc phím đầu tiên; gõ `cur` không thấy lệnh đó. Thêm ca: lệnh bị `HISTORY_IGNORE` bỏ qua thì không có
       notice. Thêm ca hai shell: shell B không thấy notice của shell A. Kiểm tra stdin: thay `_wib_bin` bằng
       wrapper ghi `$@` vào file, khẳng định file không chứa chuỗi secret.

## Verification

- `cargo test secret log_store rank` và `cargo test --test cli`
- `cargo install --path . --root .bin && zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`

## Rủi ro

- **Báo nhầm:** giảm bằng tách tên biến theo `_`, danh sách "Không khớp" bắt buộc, và `WIB_SECRET_GUARD=0`.
- **Bỏ sót:** README nói rõ đây là lưới an toàn; `HISTORY_IGNORE`, dòng bắt đầu bằng khoảng trắng và
  `wib forget` vẫn là lớp bảo vệ khác.
- **`wib query` vẫn nhận buffer qua đối số** khi đang gõ. Buffer lúc gõ chưa phải lệnh đã chạy; phase này chỉ
  chuyển sang stdin cho các đường truyền lệnh đã chạy (record, suggest-fix) và slots.
