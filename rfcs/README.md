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
| 011 | [Cloudflare Workers as a supported server target](./done/011-cloudflare-workers.md) | 0.7.0 (2026-09-16) — [handoffs](./handoffs/011-cloudflare-workers/README.md) |
| 012 | [The honeypot without an inline style](./done/012-honeypot-without-inline-style.md) | 0.7.0 (2026-09-16) — [handoffs](./handoffs/012-honeypot-without-inline-style/README.md) |
| 013 | [Corrections from the first production report on Cloudflare Workers](./done/013-production-report-corrections.md) | 0.8.0 (2026-09-22) — [handoffs](./handoffs/013-production-report-corrections/README.md) |
| 014 | [A size option for the challenge widget](./done/014-challenge-widget-size.md) | 0.8.0 (2026-09-22) — [handoffs](./handoffs/014-challenge-widget-size/README.md) |
| 015 | [Fields defined by the site, bounded](./done/015-site-defined-fields.md) | 0.8.0 (2026-09-22) — [handoffs](./handoffs/015-site-defined-fields/README.md) |
| 016 | [A true MSRV, pinned CI actions, and constant-time comparison from the crypto crates](./done/016-msrv-and-ci-hygiene.md) | 0.8.0 (2026-09-22) — [handoffs](./handoffs/016-msrv-and-ci-hygiene/README.md) |
| 017 | [Delivery through an email HTTP API](./done/017-http-api-delivery.md) | 0.9.0 (2026-09-22) — [handoffs](./handoffs/017-http-api-delivery/README.md) |
| 018 | [An opt-in check that the email domain can receive mail](./done/018-email-domain-check.md) | 0.10.0 (2026-09-22) — [handoffs](./handoffs/018-email-domain-check/README.md) |
| 019 | [An opt-in `novalidate`](./done/019-optional-novalidate.md) | 0.10.0 (2026-09-22) — [handoffs](./handoffs/019-optional-novalidate/README.md) |
| 020 | [Test follow-ups, and translations as contributed examples](./done/020-test-and-docs-follow-ups.md) | 0.10.0 (2026-09-22) — [handoffs](./handoffs/020-test-and-docs-follow-ups/README.md) |

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
| P-22 | HTTP-API delivery adapters |
| P-26 | Scheduled CI job for live challenge-vendor tests (owner-cost decision) |
