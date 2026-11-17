---
phase: 3
title: "Doctor: .zshrc có dòng init nhưng shell chưa nạp"
status: completed
priority: P2
effort: "1.5h"
dependencies: [2]
---

# Phase 3: Doctor nhận ra shell chưa nạp wib dù `.zshrc` đã có dòng init

## Goal

Trong terminal Orca/IDE không có wib, `wib doctor` (lúc này chạy thẳng binary, không qua hàm `wib` của shell) nói
đúng nguyên nhân và cách sửa, thay vì khuyên thêm dòng đã có.

## Bối cảnh

- `doctor::loaded` (`src/doctor.rs:55`) hiện luôn khuyên "add `eval …` at the end of ~/.zshrc" khi không có
  `_WIB_DOCTOR_STATE`.
- `doctor::run` (`src/doctor.rs:243`) đã đọc `.zshrc` cho `load_order`.

## Files to Create / Modify

- Modify: `src/doctor.rs` (`loaded`, `load_order`, `run`, test)
- Modify: `tests/cli.rs`

## Thiết kế

`run()` đọc `.zshrc` một lần qua `setup::zshrc_path`, tính `setup::init_line(text)` một lần, truyền cho cả
`loaded` và `load_order`. `load_order` bỏ vòng `find` riêng, dùng `init_line`.

```rust
pub fn loaded(state: Option<&State>, init_at: Option<usize>, rc: &str, hint: &str) -> Check
```

| `state` | `init_at` | Kết quả |
|---|---|---|
| Some | — | `ok  wib is loaded in this shell` |
| None | Some(n) | `warn  {rc}:{n} loads wib, but this shell has not loaded it` — fix: `run \`exec zsh\`; {hint}` |
| None | None | `warn  no \`wib init zsh\` line in {rc}` — fix: `run \`wib setup zsh\`, then \`exec zsh\`` |

`hint` = `setup::host_hint(|k| std::env::var(k).ok())`.

## Tasks & Steps

- [x] Đổi chữ ký `loaded`, `load_order`; cập nhật `run`.
- [x] Test đơn vị trong `doctor.rs`: ba hàng của bảng; test `load_order` cũ vẫn qua.
- [x] `tests/cli.rs`: sandbox có `.zshrc` chứa dòng init, chạy `doctor` với `TERM_PROGRAM=vscode` và không có
      `_WIB_DOCTOR_STATE` → stdout có `has not loaded it` và `quit the app fully`; `.zshrc` rỗng → có
      `wib setup zsh`.
- [x] `zsh tests/zle/run.zsh`: test doctor hiện có (shell đã nạp) vẫn không có `fail`.

## Verification

`cargo test doctor`, rồi `cargo test` và `zsh tests/zle/run.zsh`.

## Risks

Shell đã nạp wib nhưng chạy `command wib doctor` (bỏ qua hàm): sẽ báo "has not loaded" sai. Đây là hành vi hiện
có của `loaded`, chỉ đổi lời; chấp nhận.

## Rollback

Trả `loaded` về chữ ký cũ.
