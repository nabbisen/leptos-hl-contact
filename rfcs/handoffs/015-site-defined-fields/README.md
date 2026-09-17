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
| 02 | Definition, validation, errors | D1, D2 | 01 reviewed | written after 01 |
| 03 | Server, rendering, L2 and L3 tests | D3, D5 | 02 | written after 01 |
| 04 | Delivery, SMTP body, privacy, docs, records | D4, D6 | 03 | written after 01 |

**State.** Handoff 01 written 2026-09-17.

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
