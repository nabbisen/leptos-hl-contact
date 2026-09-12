# Handoffs — RFC 002, Form state model

Companion execution documents for [RFC 002](../../done/002-form-state-model.md).
The shared rules, gates, and review-request format in
[`../001-m1-green-baseline/README.md`](../001-m1-green-baseline/README.md)
apply unchanged; review requests go to `.git-exclude/review-request/002-form-state/<NN>-<slug>.md`.

## Prerequisite

All five RFC 001 handoffs reviewed **Approved**.  Handoff 02 below uses
`ContactFieldErrors::from_server_fn_error` from RFC 001 handoff 03 and the
example route fix from RFC 001 handoff 02.

## Units and order

| # | Handoff | RFC design | Depends on | Size |
|---|---------|------------|------------|------|
| 01 | [Hydrated example](./01-hydrated-example.md) | D4 | RFC 001 | medium |
| 02 | [Build once, react in place, focus](./02-state-model-and-focus.md) | D1, D2 | 01 (for evidence) | medium |
| 03 | [Success redirect](./03-success-redirect.md) | D3 | 01, 02 | small |

01 and 02 may be developed in parallel; 02's browser evidence needs 01.
