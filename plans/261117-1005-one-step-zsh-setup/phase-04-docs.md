---
phase: 4
title: "README, caveats Homebrew, site, changelog"
status: completed
priority: P2
effort: "1.5h"
dependencies: [1, 2, 3]
---

# Phase 4: README, caveats Homebrew, site, changelog

## Goal

Mọi hướng dẫn cài đặt dùng `wib setup zsh`, và người dùng Orca/IDE biết cần khởi động lại app.

## Files to Modify

- `README.md`: Installation (dòng 112–151), Upgrading (153–163), Doctor (304), Known limitations (401),
  Command-line usage (416), Uninstall (442).
- `scripts/homebrew-formula.sh`: `caveats`.
- `site/index.html`: bước cài (khoảng dòng 912 và 965–967).
- `site/changelog.html`: mục Unreleased (dòng 96).

## Nội dung

- **Installation:** sau mỗi cách cài, một lệnh: `wib setup zsh` (tarball: `~/.local/bin/wib setup zsh`, nối vào
  cuối one-liner hiện có). Giữ đoạn "hoặc tự thêm vào cuối `~/.zshrc`" làm lựa chọn thay thế. Sau đó: `exec zsh`.
- **Terminal trong app:** một đoạn ngắn: Orca, VS Code, Cursor, JetBrains giữ môi trường shell từ lúc app khởi
  động; nếu terminal mới trong app vẫn không có wib, thoát hẳn app rồi mở lại. `wib doctor` sẽ báo trường hợp này.
- **Upgrading:** nói rõ shell dùng đường dẫn `wib` trong `PATH`, nên output init bị cache vẫn đúng sau upgrade.
- **Uninstall:** xoá khối giữa `# >>> wib >>>` và `# <<< wib <<<`.
- **Caveats Homebrew:**

  ```text
  Run this once, then open a new terminal (or run `exec zsh`):
    wib setup zsh
  Terminals inside an app (Orca, VS Code, Cursor, JetBrains) need that app restarted.
  ```

- **Changelog Unreleased:** Added `wib setup zsh`; Changed: `wib init zsh` dùng đường dẫn trong `PATH`, báo lỗi
  khi binary mất; doctor nhận ra shell chưa nạp wib.

## Tasks & Steps

- [x] Sửa README các mục trên; câu đầy đủ, giữ giọng văn hiện có.
- [x] Sửa `caveats`; chạy `scripts/homebrew-formula.sh 0.2.0 <dir có .sha256 giả hợp lệ>` và kiểm output là Ruby
      hợp lệ (`ruby -c` nếu có).
- [x] Sửa `site/index.html`, `site/changelog.html`.
- [ ] Mở bằng trình duyệt kiểm hiển thị (chưa làm; chỉ thêm thẻ `p`, `code`, `ul` theo mẫu có sẵn).
- [x] `grep -rn 'echo .eval' README.md site/index.html` không còn hướng dẫn `echo … >> ~/.zshrc` là cách chính.

## Verification

`cargo test` (USAGE không đổi ngoài phase 2), kiểm tra link nội bộ README (`#installation`, `#uninstall`).

## Rollback

`git revert` các file tài liệu.
