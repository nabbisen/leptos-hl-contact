# leptos-hl-contact RFCs

This directory follows [RFC 000 — RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md)
in its **5-folder variant**: `proposed/` (under review), `accepted/` (review
complete; implementer may start), `done/` (shipped), `archive/` (withdrawn or
superseded).  The folder is the source of truth for an RFC's state.  Optional
implementation companions live under `handoffs/NNN-slug/` and inherit the
state of their RFC.

Numbers are assigned when an RFC file is created and are never reused.
Planned topics that do not yet have a file are listed in
[`ROADMAP.md`](../ROADMAP.md) with `P-` identifiers.

## Proposed

| ID | Title | Priority |
|----|-------|----------|
| — | *(none)* | |

## Accepted

| ID | Title | Handoff |
|----|-------|---------|
| — | *(none)* | |

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.3.3 (commit `8d29d5a`) |
| 001 | [Milestone M1: green baseline](./done/001-m1-green-baseline.md) | 0.3.4 (2026-09-12) — [handoffs](./handoffs/001-m1-green-baseline/README.md) |
| 002 | [Form state model](./done/002-form-state-model.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/002-form-state-model/README.md) |
| 003 | [Error codes and localisable server messages](./done/003-error-codes.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/003-error-codes/README.md) |
| 007 | [One context closure for Axum](./done/007-one-context-closure.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/007-one-context-closure/README.md) |
| 004 | [Form token: rename, minimum age, cookie binding, client acquisition](./done/004-form-token.md) | 0.5.0 (2026-09-13) — [handoffs](./handoffs/004-form-token/README.md) |
| 005 | [Challenge providers: Turnstile, hCaptcha, reCAPTCHA](./done/005-challenge-providers.md) | 0.5.0 (2026-09-13) — [handoffs](./handoffs/005-challenge-providers/README.md) |
| 006 | [Pre-delivery filter hook](./done/006-contact-filter.md) | 0.5.0 (2026-09-13) — [handoffs](./handoffs/006-contact-filter/README.md) |
| 008 | [Test strategy](./done/008-test-strategy.md) | 0.6.0 (2026-09-13) — [handoffs](./handoffs/008-test-strategy/README.md) |
| 009 | [A bound on delivery time](./done/009-delivery-time-bound.md) | 0.6.0 (2026-09-13) — [handoffs](./handoffs/009-delivery-time-bound/README.md) |
| 010 | [Release 0.6.0: removals, email syntax, delivery-error rule](./done/010-release-0.6.0.md) | 0.6.0 (2026-09-13) — [handoffs](./handoffs/010-release-0.6.0/README.md) |

## Archive

| ID | Title | Reason |
|----|-------|--------|
| — | *(none yet)* | |

## Planned (not yet numbered)

Derived from the roadmap milestones; each becomes a numbered RFC in
`proposed/` when the owner approves the milestone.

| Roadmap item | Topic |
|--------------|-------|
| P-20 | Multi-language label presets |
| P-22 / P-23 | HTTP-API delivery adapters and runtime portability |
| P-26 | Scheduled CI job for live challenge-vendor tests (owner-cost decision) |
