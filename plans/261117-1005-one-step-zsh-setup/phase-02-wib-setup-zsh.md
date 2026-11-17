---
phase: 2
title: "wib setup zsh"
status: completed
priority: P1
effort: "3h"
dependencies: [1]
---

# Phase 2: `wib setup zsh`

## Goal

Người dùng cài xong chỉ cần một lệnh, không sửa `.zshrc` bằng tay. Lệnh này nói đúng bước tiếp theo cho terminal
hiện tại và cho Orca/IDE.

## Files to Create / Modify

- Modify: `src/setup.rs` (`zshrc_path`, `init_line`, `host_hint`, `run`)
- Modify: `src/main.rs` (`Some("setup")`, USAGE, doc comment đầu file, danh sách tên lệnh con ở dòng `wib -- …`)
- Modify: `shell/wib.zsh` (`_wib_keywords` thêm `setup`, cả comment ở dòng 120)
- Modify: `tests/cli.rs`

## Hành vi

`wib setup zsh` (đối số khác hoặc thiếu: exit 2 với usage).

1. Đích: `zshrc_path(ZDOTDIR)` = `$ZDOTDIR/.zshrc` khi `ZDOTDIR` khác rỗng, ngược lại `log_store::home()/.zshrc`.
2. Nếu đích tồn tại: `fs::metadata` (theo symlink) phải là file thường và thuộc `log_store::euid()`. Sai thì exit 2,
   không ghi, in khối cần thêm tay.
3. `init_line(text)`: số dòng (1-based) của dòng không phải comment đầu tiên chứa `wib init zsh` (cùng quy tắc
   `doctor::load_order` đang dùng; phase 3 đổi `load_order` sang gọi hàm này). Có rồi thì in
   `already set up: <đích>:<n>` rồi in bước kích hoạt, exit 0.
4. Khối ghi thêm (append, theo symlink; thêm `\n` trước nếu file không kết thúc bằng newline):

   ```zsh

   # >>> wib >>>
   export PATH="$HOME/.local/bin:$PATH"   # chỉ khi self_on_path() là None
   eval "$(wib init zsh)"
   # <<< wib <<<
   ```

   Dòng `PATH` dùng thư mục của `current_exe()`, thay tiền tố home bằng `$HOME`. Nếu đường dẫn chứa `"`, `$`, `` ` ``
   hoặc `\`, exit 2 và in khối cần thêm tay.
5. Đích chưa tồn tại: tạo mới (quyền theo umask).
6. In ra (stdout):

   ```text
   added wib to ~/.zshrc (line 87)
   this terminal: run  exec zsh
   <host_hint>
   ```

`host_hint(get)` nhận hàm đọc biến môi trường (để test thuần):

| Điều kiện | Câu in |
|---|---|
| `TERM_PROGRAM=vscode` | `VS Code / Cursor: quit the app fully and open it again; it keeps the shell environment from when it started` |
| `TERMINAL_EMULATOR=JetBrains-JediTerm` | `JetBrains IDE: quit the IDE fully and open it again; it keeps the shell environment from when it started` |
| khác | `terminals inside an app (Orca, an IDE): if a new one still lacks wib, quit that app fully and open it again` |

[UNVERIFIED] Cursor đặt `TERM_PROGRAM=vscode`; Orca không có biến nhận diện đã biết, nên rơi vào câu chung.

## Tasks & Steps

- [x] `setup.rs`: `zshrc_path`, `init_line`, `host_hint`, `block(path_line: Option<String>)`, `run(args)`.
      Test đơn vị: `init_line` bỏ qua comment; `host_hint` ba nhánh; `block` có/không có dòng PATH.
- [x] `main.rs`: dispatch `setup`, USAGE thêm `wib setup zsh   add wib to ~/.zshrc`, cập nhật dòng
      `Setup:` trong USAGE thành `run \`wib setup zsh\``, danh sách tên lệnh con thêm `setup`.
- [x] `wib.zsh`: `_wib_keywords` thêm `setup`.
- [x] `tests/cli.rs` (đặt `HOME` và `ZDOTDIR` vào thư mục sandbox):
  - [x] Chạy hai lần trên `.zshrc` có nội dung không kết thúc bằng newline: một khối, nội dung cũ giữ nguyên
        byte-for-byte, lần hai in `already set up`.
  - [x] `ZDOTDIR=<sb>/zdot`: ghi vào `<sb>/zdot/.zshrc`; `<sb>/.zshrc` không được tạo.
  - [x] `.zshrc` là symlink tới `<sb>/dotfiles/zshrc`: đích có khối, `.zshrc` vẫn là symlink.
  - [x] `.zshrc` là thư mục: exit 2, stdout/stderr chứa `eval "$(wib init zsh)"`.
  - [x] `PATH` không chứa wib → khối có `export PATH=`; `PATH` có symlink wib → không có.
  - [x] `.zshrc` có sẵn `eval "$(wib init zsh)"` (người dùng tự thêm): không ghi, in `already set up: …:<n>`.
  - [x] `wib setup` và `wib setup bash`: exit 2.
- [x] Kiểm tra bằng tay: chạy `zsh -i` với `ZDOTDIR` là thư mục vừa setup → wib nạp (`(( ${+_wib_loaded} ))`).

## Verification

`cargo test setup`, rồi `cargo test` và `zsh tests/zle/run.zsh`.

## Risks

- Không kiểm được "thuộc người dùng" khi chạy với `sudo`: `euid` là 0, file của người dùng thường sẽ bị từ chối
  (an toàn: không ghi, in khối cần thêm tay).
- Ghi giữa chừng thất bại (đĩa đầy): một lần `write_all` duy nhất; lần chạy sau thấy dòng init nếu đã ghi được.

## Rollback

Xoá lệnh `setup`. Người dùng xoá khối giữa hai marker trong `.zshrc`.
