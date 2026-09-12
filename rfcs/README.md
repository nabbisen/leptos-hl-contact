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
| — | *(none yet)* | |

## Accepted

| ID | Title | Handoff |
|----|-------|---------|
| 004 | [Form token: rename, minimum age, cookie binding, client acquisition](./accepted/004-form-token.md) | [handoffs/004-form-token/](./handoffs/004-form-token/README.md) |
| 005 | [Challenge providers: Turnstile, hCaptcha, reCAPTCHA](./accepted/005-challenge-providers.md) | [handoffs/005-challenge-providers/](./handoffs/005-challenge-providers/README.md) |
| 006 | [Pre-delivery filter hook](./accepted/006-contact-filter.md) | [handoffs/006-contact-filter/](./handoffs/006-contact-filter/README.md) |

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.3.3 (commit `8d29d5a`) |
| 001 | [Milestone M1: green baseline](./done/001-m1-green-baseline.md) | 0.3.4 (2026-09-12) — [handoffs](./handoffs/001-m1-green-baseline/README.md) |
| 002 | [Form state model](./done/002-form-state-model.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/002-form-state-model/README.md) |
| 003 | [Error codes and localisable server messages](./done/003-error-codes.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/003-error-codes/README.md) |
| 007 | [One context closure for Axum](./done/007-one-context-closure.md) | 0.4.0 (2026-09-13) — [handoffs](./handoffs/007-one-context-closure/README.md) |

## Archive

| ID | Title | Reason |
|----|-------|--------|
| — | *(none yet)* | |

## Planned (not yet numbered)

Derived from the roadmap milestones; each becomes a numbered RFC in
`proposed/` when the owner approves the milestone.

| Roadmap item | Topic |
|--------------|-------|
| P-15 | Test strategy |
| P-20 | Multi-language label presets |
| P-22 / P-23 | HTTP-API delivery adapters and runtime portability |
| P-26 | Scheduled CI job for live challenge-vendor tests (owner-cost decision) |
