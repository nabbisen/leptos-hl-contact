# Roadmap

This file is the planning baseline for `leptos-hl-contact`.  The roadmap
and its milestones are defined jointly by the project owner and the
architect; the owner holds final approval.  Items marked **proposed** are
architect recommendations awaiting owner approval.  Design work for
scheduled items is carried out through RFCs (see [`rfcs/README.md`](./rfcs/README.md));
RFC numbers are assigned only when the RFC file is created.

Release tags use the form `X.Y.Z` (no `v` prefix).

---

## Released

### 0.1.0 — MVP

- [x] `ContactForm` component
- [x] `submit_contact` server function
- [x] `ContactInput` model with server-side validation
- [x] Honeypot field
- [x] `ContactDelivery` trait
- [x] `NoopDelivery`
- [x] `LettreSmtpDelivery`
- [x] Class and labels injection
- [x] `ContactFormOptions`
- [x] Basic documentation
- [x] Axum example
- [x] Apache-2.0 licence

### 0.2.x / 0.3.x — Near-term hardening

- [x] Per-field validation errors (`ContactFieldErrors`) — *see P-02: the client-side rendering of these errors is currently broken*
- [x] `axum-helpers` feature: `delivery_context_fn`, `provide_contact_delivery`
- [x] CSRF guidance and example middleware
- [x] Rate limit guide (tower-governor)
- [x] Cloudflare Turnstile integration guide
- [x] CSRF token helper (`csrf` feature) — HMAC-SHA256 stateless tokens — *see P-12*
- [x] Rate limit integration example (`examples/axum-with-security`)
- [x] `ContactServerPolicy` server-side enforcement
- [x] Credential redaction in `Debug`, fail-closed `csrf`, PII-free logs
- [x] RFC lifecycle policy (RFC-000, 5-folder variant)

### 0.3.4 — Milestone M1 "green baseline" (2026-09-12)

- [x] CI gates green with a matched 1.91 toolchain; first green run in the project's history
- [x] Examples start on Axum 0.8 and are compiled in CI; security example serves with connection info
- [x] Per-field validation errors render beside their field
- [x] Character-based limits; `MESSAGE_MAX_LEN` ceiling enforced
- [x] Release records and crate rustdoc corrected
- [x] RFC 001 → `rfcs/done/`

---

## Planned — proposed milestones (awaiting owner approval)

Source: architect baseline review on 2026-09-12 of commit `8d29d5a` (tag
`0.3.3`).  "Verified" means observed by running the toolchain or confirmed
by reading crate and framework source; "inferred" means a conclusion from
code reading that has not yet been reproduced by a test and must be
confirmed during implementation.

Documents produced from this review: [Requirements](./docs/src/development/requirements.md)
and [External Design](./docs/src/development/external-design.md).

### 0.4.0 — Milestone M2 (2026-09-13)

- [x] Form built once: the anti-automation token survives a failed submission (P-10, P-11)
- [x] Focus moves to the first invalid field (P-16)
- [x] Success page confirms with or without JavaScript (P-13)
- [x] Error codes on the wire; every message localisable (P-14, P-29)
- [x] One context closure for Axum; the manual server-fn route was never reached (P-28)
- [x] The security example ships a WASM client, so hydration is testable
- [x] RFC 002, RFC 003, RFC 007 → `rfcs/done/`
- [x] Ships with no known issues listed — a first for the project

### M1 — Green baseline — **released as 0.3.4 on 2026-09-12** (tag `0.3.4`, crates.io)

Tracked by [RFC 001](./rfcs/done/001-m1-green-baseline.md); handoffs in
[`rfcs/handoffs/001-m1-green-baseline/`](./rfcs/handoffs/001-m1-green-baseline/README.md).

No public API change.  Goal: CI gates green, documentation truthful,
per-field errors visible.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-01 | Make CI gates green — **done** in handoff 001-01, approved 2026-09-12 (`e37e836`, `e8512f2`, `b98980d`, `ee769aa`).  Also found and fixed: the CI workflow had never run a matched clippy (Debian `cargo-1.91` without `clippy-1.91`); CI now installs 1.91 via `dtolnay/rust-toolchain`; run 34688023060 is the first green run in the project's history | **High** | fix | verified |
| P-02 | Per-field validation errors never rendered on the client (component matched the framework's `Display` text) — **done** in handoff 001-03, approved 2026-09-12 (`eaceab5`): the component matches the `Args` variant via `ContactFieldErrors::from_server_fn_error`; no-JS transcript and both error-routing checks reproduced by the architect | **High** | fix + test | verified |
| P-03 | Documentation drift sweep — **done**: book and README on 2026-09-12 (docs restructure); crate rustdoc and header comments in handoff 001-05, approved 2026-09-12 (`0c93749`, `34c2bd2`) | **High** | docs | verified |
| P-04 | `ContactServerPolicy.max_message_len` compared bytes while the validator counts characters — **done** in handoff 001-04, approved 2026-09-12 (`17f2aff`): `ContactServerPolicy::check` counts characters | Medium | fix + test | verified |
| P-05 | Release-record hygiene — **done** for the CHANGELOG (every tagged version dated, 0.2.1 and 0.2.3 added, 0.2.2 corrected) in handoff 001-05, approved 2026-09-12; example crate versions move to 0.3.3 in handoff 001-02 | Medium | docs | verified |
| P-06 | RFC directory scaffolding per RFC-000 5-folder variant: `rfcs/README.md` index and state folders | Medium | governance | done 2026-09-12 with this roadmap update |
| P-07 | 4 000-character ceiling documented but not enforced — **done** in handoff 001-04: `MESSAGE_MAX_LEN` single definition, options and policy clamped; also fixed the `ssr`-without-`csrf` warning and added a feature-combination clippy step plus `-D warnings` on the examples job | Low | fix | verified |
| P-08 | `docs/book.toml` `git-repository-icon` rejected by mdbook 0.5 in every tried form — **done 2026-09-12**: key removed, default icon used; book builds | Low | docs | verified |
| P-09 | Examples panicked at startup on the Axum 0.7 wildcard syntax — **done** in handoff 001-02, approved 2026-09-12 (`16a32a9`, `e15ede3`); examples now compiled by a CI matrix job; `axum-with-security` additionally served with connection info so the rate limiter's peer-address fallback works (it had answered 500 to every header-less request) | **High** | fix + CI | verified |

### M2 — Form robustness and anti-abuse redesign (proposed release: 0.4.0, minor) — **complete**: RFC 002, RFC 003 and RFC 007 implemented and approved on `main` 2026-09-13 all handoffs approved.  Ready for a 0.4.0 release-readiness pass on the owner's word

Behavioural or API changes.  Each item needs an accepted RFC before a
handoff is written.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-10 | Form subtree rebuilt on every action-value change — **done** in handoff 002-02 (`d97e078`): the form is built once and only error and success regions react.  Typed input was never lost (tachys rebuilds in place); it is now a regression guard | **High** | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser by the architect |
| P-11 | Hidden token emptied by the client-side rebuild — **done** in handoff 002-02: the token survives any number of failed submissions (SSR-then-hydrate case).  The client-side-navigation case moves to RFC 004 | **High** | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser by the architect |
| P-12 | Anti-forgery token redesign: the token is not bound to the visitor and is replayable within its TTL, so it does not prevent cross-site forgery on its own; decide between cookie binding, session binding, or repositioning it as an anti-automation "form token" with the Origin check as the documented CSRF control | **High** | RFC | verified by design reading |
| P-13 | No success confirmation without JavaScript — **done** in handoff 002-03 (`fa3a970`): `ContactSuccessRedirect` + `axum_helpers::success_redirect`; every successful submission goes to the configured page in both modes; unconfigured case unchanged and documented | Medium | [RFC 002](./rfcs/done/002-form-state-model.md) | no-JS 302 reproduced by the architect |
| P-27 | ~~Per-field errors do not render under hydration~~ — **withdrawn 2026-09-12**: reported by handoff 002-01, not reproducible by the architect on a clean client build (decode path verified natively, instrumented and clean hydrated runs both render the field error); attributed to a stale WASM bundle.  Handoff 02 begins with a clean rebuild and a regression check | — | withdrawn | verified in the browser |
| P-14 | Server-composed messages were English and unlocalisable — **done** in RFC 003 handoff 01 (`aaca97c`), approved 2026-09-13: codes on the wire, `ContactErrorLabels` on the client; verified with a Japanese label set on both the hydrated and no-JS paths | Medium | [RFC 003](./rfcs/done/003-error-codes.md) | verified by the architect |
| P-15 | Test strategy: integration tests for `submit_contact` (CSRF fail-closed, policy, honeypot, delivery error), component render tests, no-JS flow; current server tests only check string sentinels | Medium | RFC / handoff | verified.  Required cases recorded so far: the 0.4 `csrf_token` argument still being accepted (a compatibility promise with a removal date), and the component rendering an error code end to end |
| P-16 | Focus management after a failed submission — **done** in handoff 002-02: `focus_first_error` option, default on | Low | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser |
| P-28 | "Two context sites" guidance was wrong — **done** in RFC 007 handoff 01 (`804b0db`), approved 2026-09-13 with one correction: the manual route is gone from both examples and nine pages; the context closure is the single site | **High** | [RFC 007](./rfcs/done/007-one-context-closure.md) | verified by the architect |

### M3 — Anti-abuse (proposed release: 0.5.0, minor) — **authorized 2026-09-12; in progress from 2026-09-13**

Owner decisions of 2026-09-12: theme approved and positioned | P-29 | Blank required field rendered the length message — **done** in RFC 003 handoff 02 (`5c61968`, `ea534b4`), approved 2026-09-13; the field mapping is now exhaustive over `ContactField`, verified by probe | Medium | [RFC 003](./rfcs/done/003-error-codes.md) handoff 02 | verified by the architect |
| P-30 | Cookie tossing from a subdomain defeats binding — **closed for the common case** in RFC 004 handoff 02 (`fe7e027`), approved 2026-09-13: `__Host-` prefix at the defaults, threat-model row T17; `secure: false` or a non-root path keeps the weaker guarantee, documented; Origin validation remains the control | **High** | [RFC 004](./rfcs/accepted/004-form-token.md) handoff 02 | verified by the architect |
directly
after M2; first-release providers are Cloudflare Turnstile, hCaptcha,
reCAPTCHA v2 and v3 (not Enterprise); when a challenge is enabled,
no-JavaScript submissions are rejected with a `<noscript>` message,
fail-closed, with an explicit opt-in to accept them under honeypot-only
protection; an HTTP client dependency is accepted behind a
`challenge-http` feature.

All three RFCs (004, 005, 006) were accepted on 2026-09-12; handoffs exist for each.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-12 | Form token redesign — **done** in RFC 004 handoffs 01–03, all approved 2026-09-13 (final `10a12a8`): rename with aliases, two-second minimum age, cookie binding with a per-browser nonce and `__Host-` prefix, client-side acquisition and loop-proof refresh (off by default) | **High** | [RFC 004](./rfcs/accepted/004-form-token.md) | verified by the architect |
| P-21 | Challenge providers — **done** in RFC 005 handoffs 01–03, all approved 2026-09-13 (final `02cf7d7`): server decision table, widget with explicit rendering, HTTP verifiers with redirects disabled, verified against the real vendor endpoints | **High** | [RFC 005](./rfcs/accepted/005-challenge-providers.md) | verified by the architect |
| P-25 | Pre-delivery filter hook — implemented in RFC 006 handoff 01 (`5e2e406`), conditionally approved 2026-09-13 on C1 (P-31) and a wording fix | Medium | [RFC 006](./rfcs/accepted/006-contact-filter.md) | verified by the architect |

### M4 — Reach (p| P-31 | **Regression shipped in 0.4.0:** with a success page configured, a honeypot hit returned success without the redirect, so a bot could tell it was caught (FR-ABUSE-09); `SilentDrop` would have inherited it.  Found in review of RFC 006 handoff 01; corrected there by applying the redirect on every successful outcome.  Threat model T18.  **Owner decision 2026-09-13: called out as a security fix in the 0.5.0 release notes** (CHANGELOG `### Security`, added with C1) | **High** | RFC 006 handoff 01, C1 | reproduced by the architect |
roposed release: 0.6.x)

| ID | Item | Priority | Kind |
|----|------|----------|------|
| P-20 | Multi-language label presets (GUI rule requires i18n; depends on P-14 for server messages) | Medium | RFC |
| P-22 | HTTP-API delivery adapters: Resend, SendGrid, AWS SES (existing Future items) | TBD | RFC per adapter |
| P-23 | Cloudflare Workers compatibility: `lettre` with tokio and native TLS cannot run on Workers; requires a fetch-based delivery adapter and a runtime-neutral core; owner decision on target platforms | TBD (owner decision) | RFC |
| P-26 | Scheduled CI job running the `#[ignore]` live tests against challenge-vendor test keys; deferred by the owner on 2026-09-12 because it carries a cost; needs its own RFC | TBD (owner) | RFC |
| P-24 | Dependency and CI hygiene: consider `subtle` for constant-time comparison; CI matrix on MSRV 1.85 plus stable instead of Debian `rustc-1.91` only.  Lock-file facts recorded 2026-09-12 (dev team): `rand` 0.8.6 beside transitive 0.9.4 — the bump to 0.9 is folded into RFC 004 handoff 01; `sha2` 0.10.9 beside our 0.11.0 comes only from Leptos' `wasm_split_macros` proc-macro and is not ours to unify | Low | task |

### Future (unscheduled)

- [ ] Database persistence adapter
- [ ] Queue-based delivery adapter (also addresses bounded delivery time)
- [ ] Advanced slot / render prop API (submit button, success/error slots)
- [ ] mdBook-published documentation site

### Not planned for now

- GUI admin panel for managing submissions
- Attachment / file upload support
- Complex form builder UI

---

## Decisions required from the owner

1. Approve or amend the M1 / M2 / M3 structure and the proposed priorities.
2. ~~Direction for P-12 (anti-forgery token).~~ Decided 2026-09-12 in RFC 004:
   rename to form token, minimum age 2 s, opt-in cookie binding, client
   acquisition; hidden field renamed in 0.5.0.
3. Target platforms (P-23): is Cloudflare Workers in scope for this crate?
4. ~~Whether a bundled Turnstile adapter (P-21) is in scope.~~ Decided 2026-09-12: in scope, see M3.
5. Version numbers: the architect proposes 0.3.4 for M1 and 0.4.0 for M2; the
   owner decides release timing and numbering.
