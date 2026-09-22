# Handoffs — RFC 017, Delivery through an email HTTP API

Companion to [RFC 017](../../accepted/017-http-api-delivery.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/017-http-api-delivery/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [A private HTTP module, shared with `challenge-http`; the body builder moved](./01-shared-http-and-body.md) | D1 | — | medium |
| 02 | [The Resend adapter: config, request, errors, tests](./02-resend-adapter.md) | D1a, D2–D4, D6 | 01 | large |
| 03 | [Workers, documentation, records, the live test](./03-workers-docs-and-records.md) | D5–D7 | 02 | medium |

**State.** Handoffs written 2026-09-22.  Handoff 01 approved 2026-09-22 (`603330d`); its three follow-ups landed in handoff 02 (§1a).  Handoff 02 approved 2026-09-22 at r2 (`aa79c55`, CI fix `42ddf8a`, r2 `0d28fde`): the adapter, the `https` warning on both override methods, a cap that bounds the native read and states its wasm32 limit, and one shared subject composer.

## Rules for this RFC

- **An API key is a secret.**  It never appears in a log, an error, a test
  assertion message, a review request, or `Debug` output.
- **A submission's content is personal data.**  The visitor's address, the
  subject, the message and every site-field value stay out of logs and
  errors, including anything the provider sends back.
- **Never pass the provider's error text through.**  It can echo what the
  visitor typed.  Status codes only.
- **No behaviour change to `challenge-http`.**  Handoff 01 is a refactor:
  the existing unit, worker and live tests are the gate, unchanged.
- **Fail closed.**  A missing key, sender or recipient is a `Configuration`
  error at delivery time, never a silent success.
