# Handoffs — RFC 019, An opt-in `novalidate`

Companion to [RFC 019](../../accepted/019-optional-novalidate.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/019-optional-novalidate/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [The option, the attribute, the tests, the documentation](./01-native-validation-option.md) | D1–D4 | — | small |

**State.** Handoff 01 approved 2026-09-22 (`eb84632`).  **RFC 019 is complete**; it moves to `done/` at the release.

**The default must not change.**  With `native_validation` at its default,
the rendered markup is byte-identical to 0.9.0's, and a test pins it.
