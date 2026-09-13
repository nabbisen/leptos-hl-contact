# Handoff 03 — Requirement-to-test traceability

**RFC.** [008](../../accepted/008-test-strategy.md), D5.
**Requirements.** NFR-TEST-02.

## Purpose

Make it checkable that tests are derived from the specification, and visible
which MUST requirements have no test.

## Change scope

`docs/src/development/testing.md` only, plus doc comments on integration and
browser tests that lack a requirement ID.

## Required implementation

1. A table in `testing.md`: every **MUST** row of the Requirements
   Specification (FR-*, NFR-*) → the test names that cover it, by layer.  A
   requirement with no automated test is listed with **none** and one line
   saying how it is verified instead (review, documentation, not testable).
2. Every test in `tests/server/` and `tests/browser/` carries at least one
   requirement or threat ID in its doc comment.
3. A short paragraph on how the table is maintained: a handoff that adds or
   changes a MUST requirement's behaviour updates its row.

## Acceptance criteria

- Every MUST requirement appears exactly once (the review request shows the
  count of MUST rows in the specification and in the table).
- The list of "none" rows is in the review request, for the architect to
  decide whether each needs a test.
- `mdbook build` clean.

## Prohibited shortcuts

Generating the table from test names alone (it must start from the
specification); marking a requirement covered by a test that does not assert
it.

## Required evidence

The MUST-row counts; the "none" list; `mdbook build`.
