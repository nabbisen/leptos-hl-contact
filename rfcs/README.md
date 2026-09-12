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
| 002 | [Form state model](./proposed/002-form-state-model.md) | High (M2) |

## Accepted

| ID | Title | Handoff |
|----|-------|---------|
| 001 | [Milestone M1: green baseline](./accepted/001-m1-green-baseline.md) | [handoffs/001-m1-green-baseline/](./handoffs/001-m1-green-baseline/README.md) |

## Implemented

| ID | Title | Shipped in |
|----|-------|------------|
| 000 | [RFC lifecycle policy](./done/000-rfc-lifecycle-policy.md) | 0.3.3 (commit `8d29d5a`) |

## Archive

| ID | Title | Reason |
|----|-------|--------|
| — | *(none yet)* | |

## Planned (not yet numbered)

Derived from the roadmap milestones; each becomes a numbered RFC in
`proposed/` when the owner approves the milestone.

| Roadmap item | Topic |
|--------------|-------|
| P-14 | Error codes and localisable server-originated messages (RFC 003) |
| P-12 | Anti-forgery token design plus minimum age and client-side token acquisition (RFC 004) |
| P-15 | Test strategy |
| P-20 | Multi-language label presets |
| P-21 | Challenge providers: Turnstile, hCaptcha, reCAPTCHA v2/v3 (RFC 005) |
| P-25 | Pre-delivery filter hook (RFC 006) |
| P-22 / P-23 | HTTP-API delivery adapters and runtime portability |
