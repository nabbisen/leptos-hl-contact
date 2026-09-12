# Handoffs — RFC 005, Challenge providers

Companion to [RFC 005](../../accepted/005-challenge-providers.md) as
amended at acceptance.  Shared rules, gates, and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md).
Review requests go to `.git-exclude/review-request/005-challenge/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Server side: trait, decision table, arguments](./01-server-verification.md) | D2, D3 | RFC 003 handoff, RFC 004 handoff 01 | medium |
| 02 | [Component: widget, scripts, no-JS](./02-component-widget.md) | D1 | RFC 002 handoff 02 | medium |
| 03 | [HTTP verifiers, example, documentation](./03-http-verifiers-and-docs.md) | D4, D5, D6 | 01, 02 | medium |

01 and 02 may proceed in parallel.
