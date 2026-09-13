# Handoffs — RFC 008, Test strategy

Companion to [RFC 008](../../accepted/008-test-strategy.md).  Shared rules,
gates and review-request format: [`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md).
Review requests go to `.git-exclude/review-request/008-test-strategy/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Server integration suite](./01-server-integration.md) | D2, D3 | — | large |
| 02 | [Browser tests and their CI job](./02-browser-tests.md) | D4, D6 | 01's `support` conventions only | medium |
| 03 | [Requirement-to-test traceability](./03-traceability.md) | D5 | 01 and 02 merged | small |
| 04 | [Close the cheap coverage gaps](./04-coverage-gaps.md) | D5 follow-up | 01–03 done; added at the 03 review | small |

01 starts with a spike and reports its result before writing the matrix.
02 may start once 01's spike has passed.  03 comes last because it maps the
tests the other two create.

Rules that apply to all three:

- Tests assert **observable behaviour from the specification**, never an
  implementation detail.  A test that would still pass with the feature
  deleted is a defect (the mutation run before each release candidate is
  there to find them).
- No test contacts the network beyond `127.0.0.1`, reads real secrets, or
  depends on wall-clock timing without a controllable clock.
- Any temporary instrumentation is removed before commit; report the grep.
- Report `cargo test --all-features` wall time before and after the handoff.
