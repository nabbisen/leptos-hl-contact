# Handoffs — RFC 012, The honeypot without an inline style

Companion to [RFC 012](../../accepted/012-honeypot-without-inline-style.md).
Shared rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/012-honeypot-without-inline-style/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Class hook and inline-style opt-out](./01-class-and-opt-out.md) | D1–D3 | — | small |

**State.** Handoff 01 approved 2026-09-15 (`7a45d69`).  RFC 012 moves to `done/` at the 0.7.0 release.

**It comes before RFC 011's handoffs.**  It is small, and landing it first
keeps the shared files (`testing.md`, `CHANGELOG.md`) free of three-way
edits.
