# Handoffs — RFC 015, Fields defined by the site, bounded

Companion to [RFC 015](../../accepted/015-site-defined-fields.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/015-site-defined-fields/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Step 0 spike: a map argument through `server_fn`](./01-step0-spike.md) | Step 0 | RFC 016 handoff 01 | small (report only) |
| 02 | [Definition, validation, errors](./02-definition-and-validation.md) | D1, D2, A1–A3, A5 | 01 reviewed | medium |
| 03 | [Server and component](./03-server-and-rendering.md) | D3, D5, A1, A3, A4, A6, A7 | 02 | large |
| 04 | [SMTP body, documentation, traceability](./04-delivery-docs-records.md) | D4, D6, D7 | 03 | medium |

**State.** Handoff 01 (spike) reviewed 2026-09-17: the map shape stands; RFC amended (A1–A7).  Handoffs 02–04 written 2026-09-17.  Handoffs 02 (`d07a69e`), 03 (`8d146fd`, `41a59ca`) and 04 (`90849a5`) approved 2026-09-22.  **RFC 015 is complete**; it moves to `done/` at the 0.8.0 release.

## Rules for this RFC

- **The bounds in RFC 015 are requirements.**
  - **The values:** 4 fields at most, three kinds, fixed placement, and
    reserved keys.
  - **No widening:** do not widen one, even for a test convenience.
- **Field values are personal data.**
  - **Never** put one in a log, an error, a test assertion message, or a
    review request.
  - **In tests,** use obviously fake values.
- **No field key from a request** is ever logged.  Only keys from the
  site's definition may appear in logs.
