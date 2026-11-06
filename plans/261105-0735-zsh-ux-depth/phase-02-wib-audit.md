---
phase: 2
title: "wib audit: tìm và dọn secret đã có"
status: completed
priority: P1
effort: "4h"
dependencies: [1]
---

# Phase 2: `wib audit`

## Goal

Người dùng biết chính xác secret nào đang nằm trong history (file, số dòng, loại) mà không có ký tự nào của
secret bị in ra, và dọn được chúng khỏi file của wib.

## Files to Create / Modify

- Create: `src/audit.rs` (gồm `pub fn mask(cmd: &str, findings: &[Finding]) -> String`, dùng lại ở phase 5)
- Modify: `src/main.rs` (lệnh con `audit`, USAGE)
- Modify: `src/zsh_history.rs` (`pub fn parse_with_lines(raw: &[u8]) -> Vec<(usize, Entry)>`)
- Modify: `shell/wib.zsh` (`_wib_keywords`: thêm `audit`)
- Modify: `tests/cli.rs`

## Hành vi

```text
$ wib audit
~/.zsh_history:1834   GitHub token     export GITHUB_TOKEN=ghp_…
~/.zsh_history:2210   password in URL  psql postgres://app:…@db.local/app
~/.local/share/wib/history.tsv:12   password flag   mysql -u root -p…
3 commands look like they contain secrets. Rotate them, then run `wib audit --fix`.
```

- Quét **toàn bộ** `.zsh_history` (không giới hạn `WIB_PERIOD`), `history.tsv` và file `forget`. Luôn dùng
  `detect_all`, kể cả khi `WIB_SECRET_GUARD=0`.
- **Che:** mọi `Finding` trong lệnh đều được che. Loại có `public_prefix()` giữ đúng tiền tố đó (`ghp_…`);
  mọi loại khác thay toàn bộ giá trị bằng `…`. Sau khi che mới đi qua `printable`.
- Exit code: 0 khi không thấy gì (in `no secrets found`), 1 khi có.
- `--fix` (đã chốt): chỉ sửa file của wib. Xoá dòng có secret khỏi `history.tsv` và `forget` khi đang giữ khoá
  (`lock` thất bại thì báo lỗi `cannot lock …`, không ghi), ghi qua `rewrite_private`. Không bao giờ sửa
  `~/.zsh_history`; với file đó in số dòng và hướng dẫn: đóng các shell khác, xoá dòng bằng trình soạn thảo,
  rồi mở terminal mới (zsh đang chạy giữ history trong bộ nhớ và có thể ghi lại).

## Tasks & Steps

- [x] `zsh_history::parse_with_lines` dùng lại logic tách header của `parse`; test với lệnh nhiều dòng và file đã metafy.
- [x] `audit::mask`; test: hai secret trong một lệnh (`AWS_ACCESS_KEY_ID=… AWS_SECRET_ACCESS_KEY=…`), secret ≤ 4
       ký tự (`--password abcd`), mật khẩu trong URL, ký tự nhiều byte. Khẳng định: không chuỗi con nào dài ≥ 3
       ký tự của giá trị secret (ngoài tiền tố công khai) xuất hiện trong kết quả.
- [x] `audit::run(fix: bool) -> Result<bool, String>`: thu thập `(nguồn, dòng, findings, lệnh)` từ ba nguồn, in đã che.
- [x] `--fix` như trên; in số dòng đã xoá ở mỗi file.
- [x] `main.rs`: `Some("audit") => cmd_audit(..)`; `ExitCode::from(1)` khi có phát hiện (khác mã lỗi 2).
- [x] `wib.zsh`: `_wib_keywords` coi `audit` là lệnh con.
- [x] `tests/cli.rs`: hai lệnh secret + một lệnh thường → hai dòng, stdout không chứa giá trị secret, exit 1;
       history sạch → exit 0; `--fix` xoá đúng dòng trong `history.tsv` và `forget`, giữ mode 0600, `.zsh_history`
       không đổi một byte; chạy `record` song song với `audit --fix` (theo mẫu test đồng thời có sẵn trong
       `tests/cli.rs`) không mất dòng nào.

## Verification

- `cargo test audit zsh_history` và `cargo test --test cli audit`
