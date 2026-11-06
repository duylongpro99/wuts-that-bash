---
phase: 5
title: "wib unforget và wib doctor"
status: completed
priority: P2
effort: "4h"
dependencies: [1, 2]
---

# Phase 5: `wib unforget` và `wib doctor`

## Goal

Người dùng tự gỡ một lệnh đã quên mà không sửa file tay và không lộ secret, và tự tìm ra vì sao wib không chạy đúng.

## Files to Create / Modify

- Create: `src/doctor.rs`
- Modify: `src/log_store.rs` (`unforget(dir, cmd) -> io::Result<bool>`: bắt buộc giữ `lock`, ghi qua `rewrite_private`)
- Modify: `src/rank.rs` (loại cả `wib unforget …` khỏi gợi ý, như `wib forget …`)
- Modify: `src/main.rs` (lệnh con `unforget`, `doctor`, USAGE, doc comment đầu file)
- Modify: `shell/wib.zsh` (`_wib_keywords` thêm `unforget|doctor`; hàm `wib` truyền trạng thái shell cho `doctor`)
- Modify: `tests/cli.rs`, `tests/zle/run.zsh`

## `wib unforget`

- Không tham số: in danh sách đã quên, đánh số, **đã che** bằng `audit::mask` (phase 2) rồi `printable`. Rỗng thì
  in `nothing forgotten`.
- `wib unforget '<lệnh>'` (hoặc `wib unforget -- …`): khớp đúng văn bản thật (chưa che) sau khi trim. In
  `unforgot: <đã che>` hoặc `not forgotten: <đã che>` (exit 1). Không lấy được khoá thì báo lỗi, không ghi.

## `wib doctor`

```zsh
wib() {
  if [[ $1 == doctor ]]; then
    _WIB_DOCTOR_STATE="$(_wib_doctor_state)" _wib_run "$@"
  else
    "$_wib_bin" "$@"
  fi
}
```

`_wib_doctor_state` in các dòng `key=value`: `zsh_version`, `ZDOTDIR`, `extended_history`, `share_history`,
`inc_append_history`, `HISTFILE`, `KEYTIMEOUT`, widget đang gán cho `^[[B ^[OB ^[[Z ^[ ^G ^]` trong `main`, giá
trị (kể cả chưa export) của `WIB_PERIOD WIB_LIMIT WIB_MIN_CHARS WIB_ZSH_HISTORY WIB_DATA_DIR WIB_SECRET_GUARD`.

Mỗi mục in `ok`, `warn`, `info` hoặc `fail`, kèm dòng `fix:` khi không ok. Exit 1 khi có `fail`.

| Kiểm tra | Mức khi sai | Cách sửa gợi ý |
|---|---|---|
| Không có `_WIB_DOCTOR_STATE` (chạy ngoài shell đã nạp wib) | warn | Thêm `eval "$(wib init zsh)"` vào cuối `.zshrc`, mở terminal mới |
| `${ZDOTDIR:-$HOME}/.zshrc` (ZDOTDIR lấy từ trạng thái shell): dòng `wib init zsh` đứng trước dòng nạp plugin (`oh-my-zsh.sh`, `zinit`, `antidote load`, `antigen apply`, `zplug load`, `source …zsh-autosuggestions…`, `source …zsh-syntax-highlighting…`); bỏ qua dòng comment | warn | Chuyển dòng `eval` xuống sau dòng đó |
| zsh < 5.3 | fail | |
| `extended_history` tắt, hoặc 20 dòng cuối của history không có dạng `: <ts>:<d>;` | fail | `setopt EXTENDED_HISTORY` |
| Phím ↓ / Shift+Tab / Esc / Ctrl-G / Ctrl-] trong `main` không còn trỏ tới widget `_wib_*` | warn | Nêu widget đang chiếm phím; nạp wib sau plugin đó |
| Thư mục dữ liệu: không thuộc người dùng, group/other ghi được, log hoặc forget không phải 0600, là symlink | fail | Lệnh `chmod`/`chown` cụ thể |
| `WIB_PERIOD`, `WIB_LIMIT`, `WIB_MIN_CHARS` không hợp lệ | fail | Ví dụ giá trị hợp lệ |
| `KEYTIMEOUT` > 10 | info | `KEYTIMEOUT=1` nếu Esc chậm |
| Một lần `rank` trong process > 50 ms | warn | Giảm `WIB_PERIOD`; in số lệnh trong period |
| `WIB_SECRET_GUARD=0` | info | Nhắc rằng secret guard đang tắt |
| `secret::detect_all` thấy secret trong history | warn | Chạy `wib audit` (chỉ in số lượng, không in lệnh) |

Không có kiểm tra thứ tự `precmd_functions`: mọi hook precmd đều nhận cùng `$?` (reviewer đã thử).

## Tasks & Steps

- [x] `log_store::unforget` + test đơn vị (xoá đúng một dòng, giữ các dòng khác byte-for-byte, lệnh nhiều dòng đã
       escape, file không tồn tại → `false`).
- [x] `main.rs`: `cmd_unforget`; `rank.rs`: loại `wib unforget …`.
- [x] `doctor.rs`: mỗi kiểm tra là hàm thuần nhận đầu vào (nội dung `.zshrc`, map trạng thái, metadata) để test
       không cần shell thật; `run()` ghép lại và in.
- [x] `wib.zsh`: hàm `wib`, `_wib_doctor_state`; `_wib_keywords` thêm `unforget|doctor`.
- [x] `tests/cli.rs`: forget rồi unforget → lệnh trở lại trong `query`; `forget` một lệnh có token rồi `unforget`
       không tham số → stdout không chứa token; `doctor` với `.zshrc` giả có `eval` trước `source $ZSH/oh-my-zsh.sh`
       → `warn` về thứ tự; data dir mode 0777 → `fail`, exit 1; `forget` và `unforget` song song không mất dòng.
- [x] `tests/zle/run.zsh`: trong shell đã nạp wib, `wib doctor` không có dòng `fail` và báo phím đã gán đúng.

## Verification

- `cargo test doctor log_store` và `cargo test --test cli unforget doctor`
- `zsh tests/zle/run.zsh "$PWD/.bin/bin/wib"`

## Rủi ro

- **Hàm `wib` che binary trên PATH:** chỉ trong shell tương tác đã nạp wib, và gọi đúng binary đã `init`. Gọi bản
  khác bằng `command wib`. Ghi trong README.
- **Thêm tên lệnh con:** `audit`, `doctor`, `unforget`, `slots`, `suggest-fix` không còn là tìm từ khoá; dùng
  `wib -- doctor`. Ghi trong README.
