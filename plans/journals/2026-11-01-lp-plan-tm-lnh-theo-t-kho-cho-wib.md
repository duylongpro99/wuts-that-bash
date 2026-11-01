---
title: Lập plan tìm lệnh theo từ khoá cho wib
date: 2026-11-01
summary: "Plan 261101-1736-keyword-search: wib pull,master tìm lệnh cũ theo từ khoá, cả live list và CLI"
---

# Lập plan tìm lệnh theo từ khoá cho wib

## What happened
Kiểm tra binary hiện tại: `pull` đã khớp `git pull origin master` trong danh sách live, nhưng `master pull` (sai thứ tự)
và `pull,master` không khớp gì; `wib pull` báo `unknown command 'pull'`. Đã lập plan `plans/261101-1736-keyword-search/`.

## Decision
- Chế độ từ khoá khi buffer bắt đầu bằng `wib ` (live list) và khi chạy `wib <kw>` (in ra stdout).
- Dấu cách = dấu phẩy; AND, thứ tự bất kỳ.
- Từ khoá khớp như chuỗi con ở bất kỳ đâu; xếp hạng trọn từ > đầu từ > giữa từ, rồi frecency.
- `wib` là tên tool, không phải từ khoá; lệnh `wib ...` cũ bị loại trong chế độ từ khoá.
- zsh nhận diện tiền tố và gọi `wib query --keywords`; chế độ thường không đổi.

## Next steps
`/ak-cook plans/261101-1736-keyword-search/plan.md` trên nhánh `feat/keyword-search`.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
