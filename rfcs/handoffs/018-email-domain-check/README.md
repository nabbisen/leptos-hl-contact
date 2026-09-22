# Handoffs — RFC 018, An opt-in check that the email domain can receive mail

Companion to [RFC 018](../../accepted/018-email-domain-check.md).  Shared
rules, gates and review-request format:
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md), plus
the release rules in
[`../010-release-0.6.0/README.md`](../010-release-0.6.0/README.md#rules-added-for-this-release).
Review requests go to `.git-exclude/review-request/018-email-domain-check/`.

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Step 0 spike: DNS over HTTPS through the shared transport](./01-step0-spike.md) | Step 0 | — | small (report only) |
| 02 | [The lookup, and the decision table](./02-lookup-and-decision.md) | D1, D4, A1–A4 | 01 reviewed | large |
| 03 | [The pipeline step, and what the visitor is told](./03-pipeline-and-error.md) | D2, D3 | 02 | medium |
| 04 | [Documentation, privacy, traceability](./04-docs-privacy-records.md) | D5, D6 | 03 | medium |

**State.** Handoffs 02 (`b8d0d1c`) and 03 (`691a1e3`) conditionally approved 2026-09-22, one r2 covering both: **C1** the lookup must check the domain's own characters rather than trust a caller's validation, and **C2** the pipeline step must skip rather than `unreachable!` when an address has no `@`.  Handoff 04 (`b73eb7a`) approved, live tests run and passing.  Handoff 01 (spike) approved 2026-09-22: DoH stands, and the RFC is amended (A1–A4: a GET on both targets, three refusal shapes, fixture choice, tolerant parsing).  Handoffs 02–04 written 2026-09-22.

## Rules for this RFC

- **Never refuse a submission because our lookup failed.**  SERVFAIL, a
  timeout, a transport error or an unparsable answer all **accept**, with a
  `warn`.  Only the domain itself saying "no mail here" refuses.
- **An address is personal data.**  The lookup sends the **domain** only:
  never the local part, never the whole address.  No log line carries either
  — a domain can identify a small employer.
- **No probe mail, ever.**  This feature never sends anything to the
  visitor's address.
- **No default resolver.**  The site names the endpoint; the crate ships
  none.
- **No cache.**  A store keyed by domain is personal data by another name.
