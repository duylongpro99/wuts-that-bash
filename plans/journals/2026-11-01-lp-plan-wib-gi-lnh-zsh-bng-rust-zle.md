---
title: "Lập plan wib: gợi ý lệnh zsh bằng Rust + ZLE"
date: 2026-11-01
summary: Brainstorm và lập plan 5 phase cho tool gợi ý history zsh khi gõ; chưa có code
---

# Lập plan wib: gợi ý lệnh zsh bằng Rust + ZLE

## What happened
- Brainstorm nhu cầu: gõ lại lệnh cũ nhưng không nhớ cú pháp, lệnh đã bị đẩy sâu trong history.
- Kiểm tra máy: zsh 5.9, `extendedhistory` bật (có timestamp), ~11k entry từ 10/2024, SAVEHIST=10000; chưa cài fzf/atuin/zsh-autosuggestions.
- User chọn: khớp tiền tố + fuzzy, danh sách tự hiện khi gõ ≥2 ký tự; đổi ngôn ngữ từ Go sang Rust.
- Lập plan `plans/261101-0749-zsh-history-suggest/` (5 phase, ~3 ngày công); `ak plan validate` OK.

## Decision
- Binary Rust `wib` + script zsh nạp bằng `eval "$(wib init zsh)"`; không daemon, không cache ở bản đầu.
- Vẽ danh sách bằng `POSTDISPLAY`; phase 1 làm spike để kiểm chứng, dự phòng `zle -M`.
- Ghi log riêng qua preexec/precmd (ts, cwd, exit code) vì `.zsh_history` thiếu cwd và exit code; chỉ import entry `.zsh_history` cũ hơn log riêng để không đếm trùng.

## Next steps
- Chạy `/ak-cook` để làm phase 1 (spike ZLE) và phase 2 (parse history) song song.
- Benchmark p95 < 10ms ở phase 5; chỉ thêm cache nếu không đạt.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
