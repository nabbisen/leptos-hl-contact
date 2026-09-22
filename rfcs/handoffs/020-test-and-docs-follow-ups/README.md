# Handoffs — RFC 020, Test follow-ups, and translations as contributed examples

Companion to [RFC 020](../../accepted/020-test-and-docs-follow-ups.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/020-test-and-docs-follow-ups/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [The mutation follow-up tests](./01-mutation-follow-up-tests.md) | D1 | — | medium |
| 02 | [Translations as contributed examples, and two documentation lines](./02-localization-and-docs.md) | D2, D3 | — | small |
| 03 | [A flaky release gate](./03-flaky-browser-gate.md) | D4 (amendment) | — | small |

**State.** Handoffs 01 (`bfeeaea`) and 02 (`e67086e`) approved 2026-09-22.  Handoff 03 approved 2026-09-22 (`4ed68ed`): the browser gate waits for the bodies it asserts on — ten runs by the dev team and eight by the architect, all green.  **RFC 020 is complete**; it moves to `done/` at the release.  Its review added NFR-SEC-07 (a bounded external response) to the requirements; the matching traceability row is carried into RFC 018 handoff 04.

**No production code changes in either handoff.**  If a test cannot be
written without one, stop and report: that is a finding, not a licence.
