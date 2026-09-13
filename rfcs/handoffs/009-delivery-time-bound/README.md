# Handoffs — RFC 009, A bound on delivery time

Companion to [RFC 009](../../accepted/009-delivery-time-bound.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in [`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release)
(both suites in the gates, traceability kept true, CHANGELOG sections,
deliberate breaks reported).
Review requests go to `.git-exclude/review-request/009-delivery-time-bound/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Delivery deadline, SMTP default, timeout code](./01-delivery-deadline.md) | D1–D5 | RFC 010 handoffs (approved) | medium |

**State.** Handoff 01 approved 2026-09-13 (`c1785db`).  RFC 009 moves to `done/` at the 0.6.0 release.

One handoff: the wrapper, the error, the code, the label and the SMTP
default only make sense together, and a partial state would ship a code no
server sends or a deadline no client can describe.
