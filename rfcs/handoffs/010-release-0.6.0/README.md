# Handoffs — RFC 010, Release 0.6.0

Companion to [RFC 010](../../done/010-release-0.6.0.md).  Shared rules,
gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md).
Review requests go to `.git-exclude/review-request/010-release-0.6.0/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Remove the deprecated 0.4 names](./01-remove-deprecated-names.md) | D1 | — | medium |
| 02 | [Email addresses a form can reply to](./02-email-syntax.md) | D2 | 01 | small |
| 03 | [Delivery error text is logged](./03-delivery-error-text.md) | D3 | 01 | small |

**Order.**
- **01 first.**  It touches `server.rs`, the server suite and the
  traceability table, which the others also touch.
- **02 and 03 after it.**  They can land in either order, one review
  request each.
- **RFC 009.**  Its handoff comes after 03, once the owner accepts the RFC.

## State

All three handoffs approved 2026-09-13 (01 `291e06c`, 02 `ebc37bb`, 03 `37fcb1f`).  RFC 010 shipped in 0.6.0 (tag `0.6.0`).

## Rules added for this release

- **The suites.**  The server suite (`tests/server`) and browser suite
  (`tests/browser`) are part of the gates.  Run the browser suite locally
  with the runner at the lock's wasm-bindgen version (see Testing).
- **Traceability.**  Every test you add, rename or remove updates its row in
  the traceability table in the same commit.
- **CHANGELOG.**  Entries go under `[Unreleased]` in `### Removed`,
  `### Changed`, `### Added` or `### Documentation`.  A breaking change gets
  a migration line.
- **Deliberate breaks.**  Report each one you run, with its transcript.
  Make them locally and never commit them.
