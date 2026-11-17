---
title: "wib — cài một bước và báo rõ khi shell chưa nạp wib"
description: "Thêm `wib setup zsh` tự ghi .zshrc, ghi đường dẫn binary ổn định trong `wib init zsh`, báo lỗi thay vì im lặng khi binary mất, và để `wib doctor` nhận ra terminal do Orca/IDE mở bằng môi trường cũ."
status: completed
priority: P1
effort: 8h
branch: feat/one-step-zsh-setup
tags: [feature, rust, zsh, install, ux]
blockedBy: []
blocks: []
created: 2026-11-17
---

# wib — cài một bước và báo rõ khi shell chưa nạp wib

## Overview

Nguồn: phiên `/ak-advise` ngày 2026-11-17 (đã xác nhận phần khung lại). Trên một số máy, `.zshrc` đã có dòng
`eval "$(wib init zsh)"` nhưng terminal mở trong Orca hoặc IDE (VS Code, Cursor, JetBrains) vẫn không có wib cho
tới khi chạy `exec zsh`. Không chương trình cài đặt nào nạp được wib vào shell đang chạy hay vào môi trường mà host
đã cache, nên mục tiêu là:

1. **`wib setup zsh`:** một lệnh ghi dòng init vào `${ZDOTDIR:-$HOME}/.zshrc` (idempotent, có marker), thêm `PATH`
   khi cần, rồi in đúng bước kích hoạt cho terminal hiện tại và cho Orca/IDE.
2. **Đường dẫn ổn định:** `wib init zsh` ghi đường dẫn `wib` trong `PATH` thay vì đường dẫn đã resolve trong
   `Cellar/<version>`, để output init bị cache không chết sau `brew upgrade`. macOS đã làm đúng (đã kiểm); thay
   đổi này phục vụ Linux, nơi `current_exe()` resolve symlink [UNVERIFIED].
3. **Không im lặng:** `shell/wib.zsh` in một dòng lên stderr khi `_wib_bin` không chạy được.
4. **Doctor:** khi `.zshrc` có dòng init nhưng shell hiện tại chưa nạp wib, `wib doctor` cảnh báo và chỉ cách sửa
   theo host.
5. **Tài liệu:** README, caveats Homebrew, site và changelog dùng `wib setup zsh`.

## Ràng buộc

- Không thêm dependency, không daemon, không mạng.
- Chỉ ghi đúng một file: `.zshrc` đích của người dùng (theo symlink). Không ghi file không thuộc người dùng.
- Dòng `eval "$(wib init zsh)"` người dùng đã tự thêm vẫn chạy như cũ; `setup` không sửa hay di chuyển dòng có sẵn.
- Không làm chậm prompt: `wib init zsh` chỉ thêm một lần quét `PATH` lúc shell khởi động.

## Non-goals

Nạp wib vào shell đang chạy hoặc môi trường host đã cache; bash/fish; plugin riêng cho Orca hay VS Code; ghi
`.zshrc` từ `post_install` của Homebrew; daemon; tự di chuyển dòng init đặt sai chỗ; phát hành phiên bản mới.

## Kiến trúc sau thay đổi

```text
 cài (brew / tarball / cargo) ──► wib setup zsh ──► setup.rs
                                     ├─ zshrc_path(ZDOTDIR) → đọc, init_line() đã có? → báo, không ghi
                                     ├─ self_on_path() không có? → thêm export PATH vào khối
                                     ├─ append khối "# >>> wib >>> … # <<< wib <<<"
                                     └─ in: exec zsh + host_hint(TERM_PROGRAM, TERMINAL_EMULATOR)
 shell mới ──► eval "$(wib init zsh)" ──► cmd_init: _wib_bin = self_on_path() ?? current_exe()
                                     └─ wib.zsh: _wib_bin không chạy được → stderr một dòng
 wib doctor (shell chưa nạp) ──► doctor::loaded(state, init_line, host) → warn + cách sửa theo host
```

`setup.rs` giữ các hàm dùng chung (`zshrc_path`, `init_line`, `self_on_path`, `host_hint`); `doctor.rs` và
`cmd_init` gọi lại, không tự viết lại logic.

## Phases

| # | Phase | Effort | Phụ thuộc | Trạng thái |
|---|-------|--------|-----------|-----------|
| 1 | [Đường dẫn binary ổn định và báo lỗi khi binary mất](./phase-01-stable-binary-path.md) | 2h | — | Completed |
| 2 | [`wib setup zsh`](./phase-02-wib-setup-zsh.md) | 3h | 1 | Completed |
| 3 | [Doctor: `.zshrc` có dòng init nhưng shell chưa nạp](./phase-03-doctor-not-loaded.md) | 1.5h | 2 | Completed |
| 4 | [README, caveats Homebrew, site, changelog](./phase-04-docs.md) | 1.5h | 1–3 | Completed |

Các phase cùng sửa `src/main.rs`, `tests/cli.rs` và dùng `src/setup.rs`, nên làm tuần tự.

## Acceptance criteria

- [x] `wib setup zsh` hai lần liên tiếp trên `.zshrc` rỗng: `grep -c 'wib init zsh'` bằng 1; lần hai in "already".
- [x] `ZDOTDIR=<dir>`: khối được ghi vào `<dir>/.zshrc`, `~/.zshrc` không đổi.
- [x] `.zshrc` là symlink: file đích được nối thêm, symlink vẫn là symlink.
- [x] `.zshrc` không thuộc người dùng hoặc là thư mục: exit 2, không ghi byte nào, in các dòng cần thêm tay.
- [x] `wib` không có trong `PATH`: khối có dòng `export PATH=…` trước dòng `eval`; có trong `PATH` thì không.
- [x] `wib init zsh` khi `PATH` có symlink `wib` trỏ tới binary: `_wib_bin` là đường dẫn symlink đó, không phải
      đường dẫn đã resolve.
- [x] Nạp init với `_wib_bin` trỏ tới file không tồn tại: stderr có `wib: cannot run`, shell vẫn khởi động.
- [x] `wib doctor` chạy ngoài shell đã nạp, `.zshrc` có dòng init, `TERM_PROGRAM=vscode`: dòng `warn` nêu
      `.zshrc:<n>` và cách sửa nhắc tới việc thoát hẳn app. Không có dòng init: cách sửa là `wib setup zsh`.
- [x] README, caveats trong `scripts/homebrew-formula.sh`, `site/index.html` và `site/changelog.html` dùng
      `wib setup zsh`.
- [x] `cargo test` và `zsh tests/zle/run.zsh` đều qua.

## Cần từ người dùng (không chặn)

Khi có máy lỗi, chạy trong terminal Orca/IDE (trước `exec zsh`):
`ps -o args= -p $$; echo "opts=$- ZDOTDIR=$ZDOTDIR"; env | grep -iE '^(VSCODE|ORCA|TERM_PROGRAM|TERMINAL_EMULATOR)'`.
Kết quả cho biết biến môi trường nào nhận diện được Orca để thêm vào `host_hint`. Hiện Orca dùng thông báo chung.

## Scope Challenge

- Dùng lại: `doctor::{loaded, load_order, Check, bad, ok}`, `log_store::{home, euid}`, harness `Sandbox` và
  `wib_env` trong `tests/cli.rs`, `start_shell` trong `tests/zle/run.zsh`.
- Phạm vi: HOLD SCOPE (không có `--yagni`); đủ năm mục ở Overview.
- Độ phức tạp: 1 module mới (`src/setup.rs`), khoảng 9 file sửa, 4 phase. Không trait, registry hay config mới;
  `host_hint` là một `match` trên hai biến môi trường.

## Validation Log

### Session 1 — 2026-11-17

| Câu hỏi | Quyết định |
|---|---|
| macOS đã ghi đường dẫn symlink; giữ phần đường dẫn ổn định? | Giữ, phục vụ Linux/Linuxbrew |
| `setup` thêm `export PATH=…` khi wib không có trong `PATH`? | Có, trong khối có marker |
| `.zshrc` không thuộc người dùng hoặc không phải file thường? | Từ chối (exit 2), in khối cần thêm tay |
| Sửa số dòng `doctor::loaded` và bỏ clippy khỏi cổng kiểm tra? | Sửa cả hai |

### Verification Results

- Claims checked: 14 (`cmd_init` tại `src/main.rs:434`, `shell/wib.zsh:13`, `_wib_keywords` dòng 120–125,
  `doctor::{loaded, load_order, run}`, `log_store::{home, euid}`, harness `Sandbox`/`wib_env`, `start_shell`,
  dòng site/README, job `homebrew` trong `release.yml`, hành vi `current_exe()` qua symlink trên macOS).
- Verified: 11 | Failed: 3 (đã sửa sau khi người dùng xác nhận) | Unverified: 2 (Linux `/proc/self/exe` resolve
  symlink; Cursor đặt `TERM_PROGRAM=vscode`)
- Tier: Standard
- Failures:
  - `current_exe()` trên macOS trả về đường dẫn Cellar: sai. Chạy qua symlink trong `PATH` cho ra
    `_wib_bin='<dir>/wib'` (đường dẫn symlink).
  - `doctor::loaded` ở `src/doctor.rs:55`, không phải 56.
  - Dự án không dùng clippy làm cổng kiểm tra (không có trong `.github/` hay README).

### Whole-Plan Consistency Sweep

Đã đọc lại plan.md và bốn phase sau khi áp dụng: lý do của đường dẫn ổn định ghi "Linux" ở Overview và phase 1;
không phase nào còn dùng clippy làm cổng kiểm tra; dòng `PATH` và quy tắc từ chối ở phase 2 khớp acceptance criteria;
phase 3 dùng `setup::{zshrc_path, init_line, host_hint}` đúng như phase 2 định nghĩa. Không còn mâu thuẫn.
