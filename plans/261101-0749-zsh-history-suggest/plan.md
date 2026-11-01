---
title: "wib — gợi ý lệnh từ history zsh khi gõ"
description: "Binary Rust + widget ZLE hiện danh sách lệnh đã dùng trong period (7d/30d) dưới prompt khi gõ ≥2 ký tự."
status: in-progress
priority: P1
effort: 3.5d
tags: [feature, cli, zsh, rust]
blockedBy: []
blocks: []
created: 2026-11-01
---

# wib — gợi ý lệnh từ history zsh khi gõ

## Overview

`wib` (what-is-that-bash) gồm một binary Rust và một đoạn script zsh. Khi gõ ≥2 ký tự trên prompt,
danh sách lệnh đã dùng trong period cấu hình được (mặc định 30d) tự hiện dưới prompt và lọc lại
sau mỗi phím. Gõ tiếp không bị chặn. Cài đặt: `cargo install --path .` và thêm `eval "$(wib init zsh)"` vào `.zshrc`.

## Bảng phím

Danh sách có 3 trạng thái: **Typing** (danh sách hiện, chưa chọn dòng nào), **Navigating** (đang chọn một dòng),
**Hidden** (đã tắt bằng Esc, hiện lại khi buffer thay đổi).

| Phím | Typing | Navigating |
|------|--------|-----------|
| ↓ | Vào Navigating, chọn dòng đầu | Xuống dòng dưới (cuối danh sách thì quay về đầu) |
| ↑ | History của zsh như cũ | Lên dòng trên (dòng đầu thì về Typing) |
| Tab | Tab completion của zsh như cũ | Điền **cả lệnh** đang chọn vào buffer, về Typing |
| Shift+Tab | Điền **từ kế tiếp** của dòng đầu danh sách | Điền **từ kế tiếp** của dòng đang chọn |
| Space | Chèn dấu cách như thường | Thoát Navigating về Typing, **không** chèn dấu cách, danh sách giữ nguyên |
| Enter | Chạy dòng đang gõ như thường | Điền lệnh đang chọn và **chạy luôn** |
| Esc / Ctrl-G | Tắt danh sách (Hidden) | Thoát Navigating và tắt danh sách (Hidden) |
| Ký tự khác | Gõ bình thường, lọc lại | Về Typing, chèn ký tự, lọc lại |

Cmd+Tab không dùng được vì macOS chặn để chuyển ứng dụng, nên điền từng từ dùng Shift+Tab (`^[[Z`).

**Điền từng từ (Shift+Tab).** Lệnh đích được khoá khi bấm lần đầu và giữ nguyên qua các lần bấm tiếp theo,
miễn là buffer vẫn là tiền tố của lệnh đích (danh sách lọc lại không làm đổi đích). Ví dụ đang gõ `gi`,
đích `git push origin master`: lần 1 → `git`, lần 2 → `git push`, lần 3 → `git push origin`, lần 4 → `git push origin master`.
- Nếu buffer đang dở một từ (`gi`), lần bấm đầu hoàn thành từ đó (`git`).
- Ranh giới từ theo cách zsh tách token (`${(z)}`), nên chuỗi trong ngoặc như `"fix bug"` được điền một lần. Khoảng trắng gốc của lệnh được giữ nguyên.
- Nếu buffer không phải tiền tố của đích (khớp mờ, ví dụ `dcup` → `docker compose up`), lần bấm đầu thay buffer bằng từ đầu tiên của đích (`docker`).
- Khi buffer đã bằng đích, Shift+Tab không làm gì.

## Kiến trúc

```text
 phím gõ ──► ZLE hook (line-pre-redraw) ──► wib query --period 30d -- "$BUFFER"
                                                │  đọc ~/.zsh_history (chỉ lệnh cũ hơn log riêng)
                                                │  + ~/.local/share/wib/history.tsv (log riêng)
                                                │  lọc period → dedup → match → frecency
                                                ▼
                    POSTDISPLAY (danh sách dưới prompt)  ◄── stdout: mỗi dòng 1 lệnh
 precmd ──► wib record --ts --cwd --exit -- "$cmd"   (chạy nền; bỏ lệnh bắt đầu bằng dấu cách)
```

Quyết định chính:

- **Không daemon, không cache ở bản đầu.** Mỗi query đọc và parse lại file (~1MB). Chỉ thêm cache nhị phân theo mtime nếu benchmark ở phase 5 không đạt p95 < 10ms.
- **Vẽ bằng `POSTDISPLAY`** (cơ chế zsh-autosuggestions đang dùng) thay vì ghi escape sequence trực tiếp, để ZLE tự lo redraw và resize. Phase 1 kiểm chứng điều này; phương án dự phòng là `zle -M`.
- **Mặc định không có dòng nào được chọn.** Ở trạng thái Typing, Enter, Tab và ↑ giữ nguyên hành vi của zsh; chỉ khi bấm ↓ vào Navigating thì các phím mới đổi nghĩa theo bảng phím.
- **Không đếm trùng giữa hai nguồn.** Chỉ lấy entry `.zsh_history` có timestamp nhỏ hơn entry đầu tiên của log riêng.

## Phases

| # | Phase | Effort | Phụ thuộc | Status |
|---|-------|--------|-----------|--------|
| 1 | [Spike vẽ danh sách trong ZLE](./phase-01-zle-render-spike.md) | 3h | — | Done (gộp vào phase 4, kiểm bằng zpty) |
| 2 | [Rust: parse history + log riêng](./phase-02-rust-history-store.md) | 5h | — | Done |
| 3 | [Rust: match, xếp hạng, CLI query](./phase-03-rust-query-engine.md) | 5h | 2 | Done |
| 4 | [Tích hợp ZLE + `wib init zsh`](./phase-04-zle-integration.md) | 8h | 1, 3 | Done |
| 5 | [Test tích hợp, benchmark, cài đặt, README](./phase-05-verify-install-docs.md) | 5h | 4 | In progress (chờ dùng thử thật) |

Phase 1 và phase 2 chạy song song được vì không dùng chung file.

## Success Criteria

- [x] AC1: gõ `doc` hiện ≤ N (mặc định 8) lệnh trong period, hiển thị < 50ms.
- [x] AC2: ↓/↑ di chuyển trong danh sách; Tab điền cả lệnh đang chọn mà không chạy; Enter điền và chạy lệnh đang chọn.
- [x] AC8: Space khi đang Navigating trả về Typing, buffer không đổi, danh sách vẫn hiện.
- [x] AC9: Esc/Ctrl-G tắt danh sách; gõ ký tự tiếp theo thì danh sách hiện lại.
- [x] AC10: gõ `gi`, chọn `git push origin master`, bấm Shift+Tab 4 lần → buffer lần lượt là `git`, `git push`, `git push origin`, `git push origin master`.
- [x] AC11: Shift+Tab với `git commit -m "fix bug"` điền `"fix bug"` trong một lần bấm.
- [x] AC3: `WIB_PERIOD=7d` và `30d` cho kết quả khác nhau trên cùng dữ liệu.
- [x] AC4: lệnh vừa chạy có trong gợi ý ở prompt kế tiếp.
- [x] AC5: p95 của `wib query` (tính cả thời gian spawn process) < 10ms trên ~11k entry thật.
- [x] AC6: `dcup` khớp `docker compose up`.
- [x] AC7: buffer 1 ký tự không hiện danh sách.
- [x] Ctrl-R, ↑ (khi menu đóng) và Tab completion (khi menu đóng) hoạt động như cũ.
- [x] Lệnh bắt đầu bằng dấu cách không bị ghi vào log riêng.

Bằng chứng (2026-11-01): `cargo test` 25 unit + 7 CLI pass; `zsh tests/zle/run.zsh` 38/38 pass (emacs, vi mode,
zsh-autosuggestions, nhánh không có `memo=`); benchmark trên 10.072 entry thật: p50 4,6–5,1ms, p95 5,8–8,6ms
(khoảng 3ms là chi phí khởi chạy process).

Còn lại: dùng thử 1 ngày trên terminal thật (resize, iTerm2, powerlevel10k) — phase 5 bước 5.

## Non-goals

bash/fish, sync nhiều máy, gợi ý bằng AI, thay Tab completion, daemon.

## Rủi ro

| Rủi ro | Giảm thiểu |
|--------|-----------|
| Danh sách nháy, lệch khi resize, hoặc đụng plugin khác | Làm spike ở phase 1 trước; dùng POSTDISPLAY; dự phòng `zle -M` |
| Esc trong zsh bị trễ theo `KEYTIMEOUT` (mặc định 0.4s) vì ↓ và Shift+Tab cũng bắt đầu bằng ESC | Bind cả Esc và Ctrl-G; README ghi chú `KEYTIMEOUT=1`; spike kiểm tra ↓ và Shift+Tab vẫn nhận đúng khi `KEYTIMEOUT=1` |
| Bind Space toàn cục làm chậm hoặc phá `magic-space` / abbreviation của plugin khác | Widget Space chỉ chặn khi đang Navigating, còn lại gọi lại widget gốc đã lưu |
| `.zsh_history` mã hoá ký tự non-ASCII kiểu "metafied" | Unmetafy trong parser (byte 0x83 → byte kế tiếp XOR 0x20), có test |
| Spawn process mỗi phím chậm trên máy yếu | Benchmark ở phase 5; nếu không đạt thì thêm cache theo mtime |
| Repo chưa là git repo | Chạy `git init` ở đầu phase 2 nếu muốn commit theo phase |

<!-- slug: zsh-history-suggest -->
