---
phase: 1
title: "Đường dẫn binary ổn định và báo lỗi khi binary mất"
status: completed
priority: P1
effort: "2h"
dependencies: []
---

# Phase 1: Đường dẫn binary ổn định và báo lỗi khi binary mất

## Goal

Output của `wib init zsh` vẫn đúng sau `brew upgrade`, kể cả khi nó bị cache (evalcache, zinit, host replay
snapshot). Khi binary thật sự không chạy được, người dùng thấy một dòng giải thích thay vì wib biến mất âm thầm.

## Bối cảnh

<!-- Updated: Validation Session 1 - macOS không resolve symlink; lợi ích chính là Linux -->
- `cmd_init` (`src/main.rs:434`) ghi `std::env::current_exe()`.
  - **macOS (đã kiểm):** chạy qua symlink trong `PATH` thì `_wib_bin` đã là đường dẫn symlink, không phải Cellar.
    Hành vi này phải giữ nguyên.
  - **Linux [UNVERIFIED]:** `current_exe()` đọc `/proc/self/exe`, vốn đã resolve symlink, nên với Linuxbrew sẽ ra
    `.../Cellar/wib/<version>/bin/wib`. `brew cleanup` xoá thư mục này sau upgrade, làm output init bị cache chết.
  - `self_on_path` cũng được phase 2 dùng để quyết định có thêm dòng `PATH` hay không.
- `shell/wib.zsh:13`: `[[ -n ${_wib_bin-} && -x $_wib_bin ]] || return 0` thoát không báo gì.

## Files to Create / Modify

- Create: `src/setup.rs` (hàm `self_on_path`; các phase sau thêm hàm vào đây)
- Modify: `src/main.rs` (`mod setup;`, `cmd_init` dùng `setup::self_on_path`)
- Modify: `shell/wib.zsh` (dòng 13)
- Modify: `tests/cli.rs`

## Thiết kế

```rust
/// The `wib` on $PATH that is this binary, as $PATH spells it (a symlink stays a symlink).
pub fn self_on_path() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?.canonicalize().ok()?;
    std::env::split_paths(&std::env::var_os("PATH")?)
        .filter(|d| d.is_absolute())
        .map(|d| d.join("wib"))
        .find(|c| c.canonicalize().is_ok_and(|c| c == me))
}
```

`cmd_init`: `let exe = setup::self_on_path().map_or_else(std::env::current_exe, Ok)?;`. Giữ nguyên cách quote.
Chỉ chọn mục `PATH` trỏ tới **chính binary này**, nên khi có hai bản wib khác nhau trong `PATH`, shell vẫn dùng
đúng bản đã chạy `init` (giữ hành vi hiện tại).

`shell/wib.zsh`:

```zsh
[[ -n ${_wib_bin-} ]] || return 0
[[ -x $_wib_bin ]] || {
  print -ru2 -- "wib: cannot run $_wib_bin; if your shell caches \`wib init zsh\` output, clear that cache and open a new terminal"
  return 0
}
```

## Tasks & Steps

- [x] Tạo `src/setup.rs` với `self_on_path`; khai báo `mod setup;` trong `main.rs`.
- [x] Sửa `cmd_init` dùng `self_on_path`, fallback `current_exe`.
- [x] Sửa `shell/wib.zsh:13` như trên.
- [x] `tests/cli.rs`: tạo `bin/` trong sandbox với symlink `wib` → `env!("CARGO_BIN_EXE_wib")`; chạy
      `init zsh` với `PATH=<bin>:/usr/bin:/bin` → dòng đầu là `typeset -g _wib_bin='<bin>/wib'`.
- [x] `tests/cli.rs`: `PATH=/usr/bin:/bin` → dòng đầu vẫn là đường dẫn tuyệt đối (test cũ
      `init_zsh_embeds_binary_path` giữ nguyên).
- [x] `tests/cli.rs`: lấy output init, thay dòng đầu bằng `typeset -g _wib_bin='/nonexistent/wib'`, ghi ra file,
      chạy `zsh -f -i -c 'source <file>; print ok'` → stderr chứa `wib: cannot run /nonexistent/wib`, stdout
      chứa `ok`. Bỏ qua test khi máy không có `zsh` (giống cách các test khác xử lý, nếu có).

## Verification

`cargo test init_`, rồi `cargo test` và `zsh tests/zle/run.zsh`.

## Risks

- `PATH` có thư mục mạng chậm: `canonicalize` từng mục chỉ chạy một lần lúc shell khởi động; chấp nhận được.
- Mục `PATH` tương đối (`.`) bị bỏ qua để không ghi đường dẫn phụ thuộc thư mục hiện tại.

## Rollback

Trả `cmd_init` về `current_exe()`; bỏ thông báo stderr. Không có dữ liệu nào bị đổi.
