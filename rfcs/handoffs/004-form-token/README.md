# Handoffs — RFC 004, Form token

Companion to [RFC 004](../../accepted/004-form-token.md).  Shared rules,
gates, and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md).
Review requests go to `.git-exclude/review-request/004-form-token/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Rename with aliases; minimum age](./01-rename-and-minimum-age.md) | D1, D2, D5 | RFC 003 handoff (codes) | medium |
| 02 | [Cookie binding in axum-helpers](./02-cookie-binding.md) | D3 | 01 | medium |
| 03 | [Client-side acquisition and refresh](./03-client-acquisition.md) | D4 | 01, 02 | medium |

Strictly in order; each touches `form_token.rs` and `server.rs`.
