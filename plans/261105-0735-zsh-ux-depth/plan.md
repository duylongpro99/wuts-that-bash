---
title: "wib — đào sâu trải nghiệm cho người dùng zsh: secret guard, template, sửa lệnh gõ sai, doctor, unforget"
description: "Năm tính năng cho người dùng zsh hiện tại: không lưu và không gợi ý lệnh chứa secret, nhảy giữa các tham số hay đổi, gợi ý sửa tên chương trình gõ sai, tự chẩn đoán cài đặt, và bỏ quên một lệnh."
status: completed
priority: P1
effort: 27h
branch: feat/zsh-ux-depth
tags: [feature, rust, zsh, security, ux]
blockedBy: []
blocks: []
created: 2026-11-05
---

# wib — đào sâu trải nghiệm cho người dùng zsh

## Overview

Nguồn: `plans/reports/research-261105-1429-terminal-pain-points-next-features.md`. Người dùng chọn đào sâu giá
trị cho người dùng zsh hiện tại trước khi mở rộng sang shell khác.

1. **Secret guard:** lệnh chứa token, mật khẩu hay private key không được ghi vào log của wib, không bao giờ hiện
   trong gợi ý, không lộ qua tham số process của wib, và shell đã chạy lệnh đó được nhắc. `wib audit` tìm secret
   đã có trong history (in đã che) và dọn khỏi file của wib.
2. **Template:** Ctrl-] đưa con trỏ tới phần "hay thay đổi" tiếp theo (tag, port, tên pod), dựa trên các lần chạy
   trước có cùng dạng. Gõ ngay thì thay cả phần đó.
3. **Gợi ý sửa lệnh gõ sai:** sau "command not found" (exit 127), prompt kế tiếp hiện lệnh đã sửa tên chương
   trình; ↓ Enter để chạy. Đối số không bao giờ bị đổi.
4. **`wib doctor`:** kiểm tra cài đặt và nói cách sửa.
5. **`wib unforget`:** liệt kê (đã che) và bỏ quên một lệnh đã `forget`.

## Ràng buộc

- Không thêm dependency, không daemon, không mạng.
- Không làm chậm prompt: lệnh thành công không tạo thêm process; process đồng bộ chỉ chạy sau exit 127 hoặc khi
  người dùng bấm Ctrl-].
- Giữ nguyên hành vi phím hiện có; ↑, Tab, Enter, Ctrl-R chỉ đổi nghĩa khi một dòng đang được chọn.
- wib không bao giờ sửa `~/.zsh_history`.
- p95 của `wib query` trong `scripts/bench.sh` không tăng quá 10%; `suggest-fix` và `slots` p95 < 15 ms.

## Non-goals

bash/fish, Homebrew, đồng bộ nhiều máy, tính năng AI, tự chạy lệnh đã sửa, sửa đối số hay subcommand, đổi định
dạng `history.tsv`, đoán slot theo hình dạng khi không có lần chạy cùng dạng.

## Kiến trúc sau thay đổi

```text
 wib.zsh: _wib_run <sub> … (truyền WIB_ZSH_HISTORY/HISTFILE, WIB_DATA_DIR, WIB_PERIOD, WIB_SECRET_GUARD)
   ├─ precmd:  print -rn cmd | wib record --stdin --notice-id $$ … &!   ──► secret::detect → bỏ qua, ghi secret-notice.<pid>
   ├─ precmd:  exit 127? print -rn cmd | wib suggest-fix --stdin       ──► fix.rs: sửa token đầu
   ├─ line-init / pre-redraw: secret-notice.$$ → zle -M; _wib_fix → danh sách khi buffer rỗng
   ├─ _wib_fetch → wib query …                                          ──► rank.rs: sort rồi lọc secret lười
   └─ Ctrl-] → print -rn BUFFER | wib slots --stdin                     ──► slots.rs: vị trí hay đổi
 wib audit [--fix]   ──► audit.rs: quét .zsh_history + history.tsv + forget; mask; --fix chỉ file của wib
 wib doctor          ──► hàm zsh `wib` truyền _WIB_DOCTOR_STATE ──► doctor.rs
 wib unforget [cmd]  ──► log_store::unforget (bắt buộc giữ lock, rewrite_private)
```

## Phases

| # | Phase | Effort | Phụ thuộc | Trạng thái |
|---|-------|--------|-----------|-----------|
| 1 | [Secret guard: phát hiện, không ghi, không gợi ý, nhắc](./phase-01-secret-guard.md) | 7h | — | Completed |
| 2 | [`wib audit`: tìm và dọn secret đã có](./phase-02-wib-audit.md) | 4h | 1 | Completed |
| 3 | [Template: nhảy giữa các phần hay đổi](./phase-03-template-slots.md) | 6h | 1 | Completed |
| 4 | [Gợi ý sửa lệnh gõ sai tên chương trình](./phase-04-fix-suggestion.md) | 4h | 1 | Completed |
| 5 | [`wib unforget` và `wib doctor`](./phase-05-unforget-doctor.md) | 4h | 1, 2 | Completed |
| 6 | [Benchmark, README, site](./phase-06-bench-docs.md) | 2h | 1–5 | Completed |

Các phase cùng sửa `src/main.rs`, `shell/wib.zsh`, `tests/cli.rs`, `tests/zle/run.zsh`, nên làm tuần tự.

## Acceptance criteria

- [x] Chạy một lệnh có `Authorization: Bearer …`: không có dòng nào trong `history.tsv`; không bao giờ hiện trong
      danh sách (kể cả khi chỉ có trong `.zsh_history`); chuỗi secret không xuất hiện trong tham số của process
      wib nào; shell đó hiện dòng nhắc ở prompt kế tiếp hoặc phím đầu tiên; shell khác không hiện.
- [x] `wib audit` liệt kê secret trong `.zsh_history`, log và file forget; không chuỗi con ≥ 3 ký tự nào của giá
      trị secret (ngoài tiền tố công khai) được in; `--fix` không đổi một byte nào của `.zsh_history`.
- [x] Điền `docker run -p 8080:80 nginx:1.25` rồi Ctrl-]: con trỏ tới `8080:80`; gõ `9090:80` thay nguyên phần đó;
      Ctrl-] tiếp tới `nginx:1.25`. Trong vi insert mode, Ctrl-] trên dòng không có slot không đổi buffer.
- [x] `ehco hi` (exit 127) sau khi đã có `echo hi` thành công: prompt kế tiếp hiện `echo hi`; ↓ Enter chạy nó; gõ
      bất kỳ thì gợi ý biến mất; lệnh exit 1 không tạo gợi ý và không tạo process thêm.
- [x] `wib doctor` báo `warn` khi dòng `eval` nằm trước `source $ZSH/oh-my-zsh.sh`, và không có `fail` khi cài đúng.
- [x] `wib unforget` không tham số liệt kê lệnh đã quên đã che; `wib unforget '<lệnh>'` đưa nó trở lại gợi ý.
- [x] `cargo test`, `zsh tests/zle/run.zsh` đều qua; bench đạt ngưỡng ở phần Ràng buộc.

## Scope Challenge

- Dùng lại: `log_store::{forget, load_forgotten, escape, unescape, lock, private_dir, open_regular}` và
  `rank::{is_ok, wib_args}` (hiện là private, phase 1 đổi thành `pub(crate)`), `rank::RankContext`,
  `project::{scope, in_scope}`, `zsh_history::read_since`, `printable` trong `main.rs`, harness `Sandbox` trong
  `tests/cli.rs` và `start_shell` trong `tests/zle/run.zsh`.
- Phạm vi: đủ năm tính năng (HOLD SCOPE, không có `--yagni`).
- Độ phức tạp: khoảng 15 file, 5 module mới (mỗi module một lệnh con, không trait hay registry), 6 phase. Vượt
  ngưỡng 8 file / 3 phase vì có năm tính năng độc lập; mỗi phase vẫn nhỏ.

## Validation Log

### Session 1 — 2026-11-05

| Câu hỏi | Quyết định |
|---|---|
| Áp dụng các phát hiện red-team? | Áp dụng cả 16 |
| `wib audit --fix` sửa tới đâu? | Chỉ file của wib (`history.tsv`, `forget`); `.zsh_history` chỉ báo số dòng và cách xoá |
| Có công tắc tắt secret guard? | Có: `WIB_SECRET_GUARD=0` |
| Phím template? | Ctrl-] |
| Gợi ý sửa chạy khi nào? | Chỉ exit 127 |
| Ứng viên sửa? | Log của wib (exit thành công) + token đầu xuất hiện ≥ 3 lần trong `.zsh_history` |
| Không có lần chạy cùng dạng? | Không làm gì, hiện thông báo |

Hệ quả của "chỉ exit 127": phase 4 chỉ sửa tên chương trình; quy tắc sửa subcommand bị bỏ vì exit 127 nghĩa là
chính chương trình không tồn tại.

### Verification Results

- Claims checked: 18 (tên hàm, dòng code, binding, hành vi zsh); reviewer đã chạy thử `BUFFER` và `zle -M` trong
  `line-pre-redraw`, binding `^]` ở emacs/viins, và `$?` ở nhiều hook precmd.
- Verified: 15 | Failed: 3 (đã sửa: `^]` trong viins là `self-insert`; `load_entries` không lọc gì; các helper
  của `log_store`/`rank` là private) | Unverified: 0
- Tier: Full

### Whole-Plan Consistency Sweep

Đã đối chiếu lại plan.md và cả sáu phase sau khi áp dụng: `detect` → `detect_all` + `mask` ở phase 1, 2, 5;
`suggest-fix` chỉ exit 127 và chỉ token đầu ở plan.md và phase 4; phụ thuộc phase 3 = [1], phase 5 = [1, 2]; mọi
lời gọi binary từ zsh đi qua `_wib_run`; mọi lần ghi lại file đi qua `rewrite_private` có lock bắt buộc; bỏ kiểm
tra thứ tự `precmd_functions` ở doctor. Không còn mâu thuẫn.

## Red Team Review

Hai reviewer (Security + Assumption, Failure + Scope), 16 phát hiện sau khi gộp trùng, chấp nhận cả 16.

| # | Mức | Phát hiện | Áp dụng ở |
|---|---|---|---|
| 1 | High | Secret lộ qua tham số process (`ps`) của `record`/`suggest-fix`/`slots` | Phase 1, 3, 4: `--stdin` |
| 2 | High | `audit` chỉ che một secret, giữ 4 ký tự nên lộ mật khẩu ngắn | Phase 1 `detect_all`, phase 2 `mask` |
| 3 | High | `unforget` không tham số in secret đã quên | Phase 5 che bằng `audit::mask` |
| 4 | High | Mẫu tên biến khớp nhầm (`TOKENIZERS_…`), bỏ sót `curl -u`, `mysql -p…` | Phase 1 bảng mẫu |
| 5 | High | Test zle dùng `GITHUB_TOKEN` bị `HISTORY_IGNORE` của harness chặn | Phase 1 bước 8 |
| 6 | High | Một file notice chung cho mọi shell | Phase 1 `secret-notice.<pid>`, `zshexit` |
| 7 | High | Sửa lỗi bằng khoảng cách cả lệnh đổi mục tiêu lệnh nguy hiểm | Phase 4 chỉ sửa token đầu |
| 8 | High | Test zle phase 4 không thể qua; người mới cài không có gợi ý | Phase 4 `echo hi`/`ehco hi`, ứng viên ≥ 3 lần |
| 9 | High | `slots`/`suggest-fix` không truyền HISTFILE/period | Phase 1 `_wib_run`; phase 3 phụ thuộc phase 1 |
| 10 | Medium | Ctrl-] trong vi insert chèn byte `0x1d` khi không có slot | Phase 3 |
| 11 | Medium | Ghi lại file khi không lấy được lock làm mất dòng | Phase 1 `rewrite_private` bắt buộc lock |
| 12 | Medium | Quét secret cho mọi ứng viên làm chậm `query` | Phase 1 lọc lười sau sort |
| 13 | Medium | Đường đồng bộ `suggest-fix`/`slots` chưa có benchmark | Phase 4, 6 `bench.sh` |
| 14 | Medium | `suggest-fix` chạy sau mọi lỗi gây chậm prompt | Phase 4 chỉ exit 127 |
| 15 | Medium | Doctor kiểm tra thứ tự precmd là sai (mọi hook nhận cùng `$?`) | Phase 5 bỏ kiểm tra |
| 16 | Medium | Plan giả định helper private là public, `load_entries` có lọc | Phase 1, 3 |
