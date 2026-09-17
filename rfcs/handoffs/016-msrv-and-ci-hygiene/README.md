# Handoffs — RFC 016, A true MSRV, pinned CI actions, and constant-time comparison from the crypto crates

Companion to [RFC 016](../../accepted/016-msrv-and-ci-hygiene.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/016-msrv-and-ci-hygiene/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [MSRV 1.88 and its CI job, actions pinned with Dependabot, token comparison through `hmac`](./01-msrv-actions-and-token-compare.md) | D1–D3 | — | small |

**State.** Handoff 01 written 2026-09-17.

**Order in M6.**  This RFC comes first: it is independent, and the MSRV job
then guards RFCs 014 and 015 as they land.
