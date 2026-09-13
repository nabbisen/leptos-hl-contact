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

- [x] Per-field validation errors (`ContactFieldErrors`) — rendering on the client fixed in 0.3.4 (P-02)
- [x] `axum-helpers` feature: `delivery_context_fn`, `provide_contact_delivery`
- [x] CSRF guidance and example middleware
- [x] Rate limit guide (tower-governor)
- [x] Cloudflare Turnstile integration guide — superseded by challenge providers in 0.5.0
- [x] CSRF token helper (`csrf` feature) — renamed form token in 0.5.0 (P-12)
- [x] Rate limit integration example (`examples/axum-with-security`)
- [x] `ContactServerPolicy` server-side enforcement
- [x] Credential redaction in `Debug`, fail-closed `csrf`, PII-free logs
- [x] RFC lifecycle policy (RFC-000, 5-folder variant)

### 0.3.4 — Milestone M1, green baseline (2026-09-12)

- [x] CI gates green with a matched 1.91 toolchain; first green run in the project's history
- [x] Examples start on Axum 0.8 and are compiled in CI; security example serves with connection info
- [x] Per-field validation errors render beside their field
- [x] Character-based limits; `MESSAGE_MAX_LEN` ceiling enforced
- [x] Release records and crate rustdoc corrected
- [x] RFC 001 → `rfcs/done/`

### 0.4.0 — Milestone M2, form robustness (2026-09-13)

- [x] Form built once: the anti-automation token survives a failed submission (P-10, P-11)
- [x] Focus moves to the first invalid field (P-16)
- [x] Success page confirms with or without JavaScript (P-13)
- [x] Error codes on the wire; every message localisable (P-14, P-29)
- [x] One context closure for Axum; the manual server-fn route was never reached (P-28)
- [x] The security example ships a WASM client, so hydration is testable
- [x] RFC 002, RFC 003, RFC 007 → `rfcs/done/`
- [x] Ships with no known issues listed — a first for the project

### 0.5.0 — Milestone M3, anti-abuse (2026-09-13)

- [x] Form token: renamed from `csrf` with aliases, two-second minimum age, cookie binding with `__Host-` prefix, client-side acquisition and refresh (P-12, P-30)
- [x] Challenge providers: Turnstile, hCaptcha, reCAPTCHA v2/v3, fail-closed, verified against real vendor endpoints (P-21)
- [x] Pre-delivery filter hook and the "which layer decides what" table (P-25)
- [x] Security fix: a honeypot hit was observable with a success page configured, since 0.4.0 (P-31)
- [x] RFC 004, RFC 005, RFC 006 → `rfcs/done/`

---

## Current

### M4 — Trust what shipped — **theme authorized 2026-09-13**

Owner decisions 2026-09-13: M4 starts with the test strategy (done).  M4
ships as **0.6.0**, with the removal of the deprecated 0.4 names promised in
0.5.0 (P-37), stricter email syntax (P-34), the delivery-error rule (P-33)
and a bound on delivery time (P-32).  RFC 010 tracks the first three; RFC
009 carries P-32 and awaits design decisions.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-15 | Test strategy: a server integration suite built as the integrator's router, browser tests for hydrate-only logic, requirement-to-test traceability.  Required cases recorded by reviews: the 0.4 `csrf_token` field still accepted; the component rendering an error code end to end; the field-error / banner routing pair; the one-context-closure routing property; every successful outcome applying the success redirect | **High** | [RFC 008](./rfcs/accepted/008-test-strategy.md) (accepted; browser tests on every push, mutation run once per milestone) | **done** 2026-09-13, handoffs 01–04 approved: 25 server integration tests through the documented router (`7520763`, `1bceb59`); 9 browser tests in headless Chrome, CI job on every push (`2a7b901`, `c3f3fba`); all 102 MUST requirements traced to their tests (`45fc00c`, `c3f3fba`).  RFC 008 moves to `rfcs/done/` with the next release |
| P-33 | Tell `ContactDelivery` implementers that their error text is logged and must not contain the submission (FR-OBS-02 with FR-OBS-03); the built-in SMTP adapter already complies | Low — **approved** 2026-09-13 | [RFC 010](./rfcs/accepted/010-release-0.6.0.md) D3 — **done** (`37fcb1f`), ships in 0.6.0 | found in the RFC 008 handoff 01 review |
| P-32 | Bounded delivery time: lettre's async transport times out the TCP connect only, so a stalled relay holds the request indefinitely; custom backends have no bound (FR-DEL-08, threat T15) | Medium — **approved** 2026-09-13 | [RFC 009](./rfcs/proposed/009-delivery-time-bound.md) (proposed; three owner decisions) | open since the baseline |
| P-34 | Stricter email syntax: reject address literals (`a@[127.0.0.1]`) and single-label domains (`abc@bar`), which `validator` 0.20 accepts; enforce the 254-character limit on the server (FR-VAL-02) | Medium — **approved** 2026-09-13 | [RFC 010](./rfcs/accepted/010-release-0.6.0.md) D2 — **done** (`ebc37bb`), ships in 0.6.0 | owner question 2026-09-13; limit found in the RFC 008 handoff 04 review |
| P-37 | Remove the deprecated 0.4 names: feature `csrf`, module `csrf`, the `csrf_token` field (FR-CFG-01) | **High** — promised in the 0.5.0 CHANGELOG | [RFC 010](./rfcs/accepted/010-release-0.6.0.md) D1 — **done** (`291e06c`), ships in 0.6.0 | 0.5.0 migration notes |

---

## Unscheduled — proposed candidates for later milestones

| ID | Item | Priority | Kind |
|----|------|----------|------|
| P-20 | Multi-language label presets (GUI rule requires i18n) | Medium | RFC |
| P-22 | HTTP-API delivery adapters: Resend, SendGrid, AWS SES | TBD | RFC per adapter |
| P-23 | Cloudflare Workers compatibility: `lettre` with tokio and native TLS cannot run on Workers; needs a fetch-based delivery adapter and a runtime-neutral core | TBD (owner decision on target platforms) | RFC |
| P-24 | Dependency and CI hygiene: consider `subtle` for constant-time comparison; CI on MSRV 1.85 plus stable; pin GitHub Actions by commit SHA rather than tag (the workflow uses `actions/checkout`, `dtolnay/rust-toolchain` and, since RFC 008, `taiki-e/install-action`).  `sha2` 0.10.9 beside our 0.11.0 comes only from a Leptos proc-macro and is not ours to unify; transitive advisories in Leptos's own dependencies (`paste`, `proc-macro-error2`, `anyhow`) remain as of 0.5.0 | Low | task |
| P-26 | Scheduled CI job running the `#[ignore]` live vendor tests; deferred by the owner on 2026-09-12 because it carries a cost | TBD (owner) | RFC |
| P-35 | Opt-in mail-domain check: a DNS lookup for a mail exchanger (falling back to an address record, RFC 5321 §5.1) behind a feature, time-bounded, reported as an error under the email field so a visitor can fix a typo; accepts with a `warn` log when DNS does not answer.  Catches non-existent domains, not non-existent mailboxes | Low — **approved** 2026-09-13, less prioritized | RFC |
| P-36 | Messages the site cannot translate — **on hold** (owner, 2026-09-13): the app team upgrades after the anti-abuse releases, and the report is re-checked on the upgraded version; no questions sent, no schedule, no RFC.  Since 0.4.0 `labels.errors` covers every server message under a field.  Candidate gaps: the browser's own validation pop-ups from `required`, `type="email"` and `maxlength`, which no label reaches; one text per rule, not per field; the Customization page mentions `errors` only as a link | TBD | pending facts |

### Future

- [ ] Confirm-before-forward: the submitter receives a link and the operator receives the message only after it is clicked, which removes fake addresses.  Needs integrator-supplied storage (the crate must not persist, FR-OBS-04) and abuse controls on mail sent to arbitrary addresses.  Recorded only (owner, 2026-09-13).  An unconfirmed notice to the submitter is **not** recommended: a missing mailbox bounces later, to the site, and the form becomes a way to mail anyone
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

1. RFC 009 (P-32): the SMTP default deadline (30 s recommended), a distinct `delivery_timeout` code, and a public `DeliveryTimeout` wrapper.
2. P-23: is Cloudflare Workers a target platform for this crate?

---

## Milestone record

Kept for traceability.  Each item's final state, the commit that closed it,
and how it was verified.  Source of the original list: the architect's
baseline review of `0.3.3` (commit `8d29d5a`) on 2026-09-12, with the
[Requirements](./docs/src/development/requirements.md) and
[External Design](./docs/src/development/external-design.md) it produced.

### M1 — Green baseline → 0.3.4

Tracked by [RFC 001](./rfcs/done/001-m1-green-baseline.md).

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-01 | Make CI gates green — **done** in handoff 001-01 (`e37e836`, `e8512f2`, `b98980d`, `ee769aa`).  Also fixed: the CI workflow had never run a matched clippy; CI now installs 1.91 via `dtolnay/rust-toolchain` | **High** | fix | verified |
| P-02 | Per-field validation errors never rendered on the client — **done** in handoff 001-03 (`eaceab5`): the component matches the `Args` variant | **High** | fix + test | verified |
| P-03 | Documentation drift sweep — **done**: book and README; crate rustdoc in handoff 001-05 (`0c93749`, `34c2bd2`) | **High** | docs | verified |
| P-04 | Server policy compared bytes, validator counts characters — **done** in handoff 001-04 (`17f2aff`) | Medium | fix + test | verified |
| P-05 | Release-record hygiene — **done** in handoff 001-05 and 001-02 | Medium | docs | verified |
| P-06 | RFC directory scaffolding per RFC-000 5-folder variant — **done** 2026-09-12 | Medium | governance | done |
| P-07 | 4 000-character ceiling documented but not enforced — **done** in handoff 001-04 | Low | fix | verified |
| P-08 | `docs/book.toml` icon rejected by mdbook 0.5 — **done**: key removed | Low | docs | verified |
| P-09 | Examples panicked on the Axum 0.7 wildcard syntax — **done** in handoff 001-02 (`16a32a9`, `e15ede3`); examples compiled in CI | **High** | fix + CI | verified |

### M2 — Form robustness → 0.4.0

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-10 | Form subtree rebuilt on every action-value change — **done** in handoff 002-02 (`d97e078`); typed input was never lost, now a regression guard | **High** | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser |
| P-11 | Hidden token emptied by the client-side rebuild — **done** in handoff 002-02 | **High** | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser |
| P-13 | No success confirmation without JavaScript — **done** in handoff 002-03 (`fa3a970`) | Medium | [RFC 002](./rfcs/done/002-form-state-model.md) | no-JS 302 reproduced |
| P-14 | Server-composed messages were English — **done** in RFC 003 handoff 01 (`aaca97c`) | Medium | [RFC 003](./rfcs/done/003-error-codes.md) | verified with Japanese labels |
| P-16 | Focus management after a failed submission — **done** in handoff 002-02 | Low | [RFC 002](./rfcs/done/002-form-state-model.md) | verified in the browser |
| P-27 | ~~Per-field errors do not render under hydration~~ — **withdrawn**: not reproducible on a clean client build; a stale WASM bundle | — | withdrawn | verified in the browser |
| P-28 | "Two context sites" guidance was wrong — **done** in RFC 007 handoff 01 (`804b0db`) | **High** | [RFC 007](./rfcs/done/007-one-context-closure.md) | verified |
| P-29 | Blank required field rendered the length message — **done** in RFC 003 handoff 02 (`5c61968`, `ea534b4`) | Medium | [RFC 003](./rfcs/done/003-error-codes.md) | verified by probe |

### M3 — Anti-abuse → 0.5.0

Owner decisions of 2026-09-12: providers Turnstile, hCaptcha, reCAPTCHA v2
and v3 (not Enterprise); no-JavaScript submissions rejected fail-closed with a
`<noscript>` message when a challenge is enabled, with an explicit opt-in;
HTTP client behind a `challenge-http` feature.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-12 | Form token redesign — **done** in RFC 004 handoffs 01–03 (final `10a12a8`) | **High** | [RFC 004](./rfcs/done/004-form-token.md) | verified |
| P-21 | Challenge providers — **done** in RFC 005 handoffs 01–03 (final `02cf7d7`) | **High** | [RFC 005](./rfcs/done/005-challenge-providers.md) | verified against real vendor endpoints |
| P-25 | Pre-delivery filter hook — **done** in RFC 006 handoff 01 (`5e2e406`, `67c1ac1`) | Medium | [RFC 006](./rfcs/done/006-contact-filter.md) | verified |
| P-30 | Cookie tossing from a subdomain defeats binding — **closed for the common case** in RFC 004 handoff 02 (`fe7e027`): `__Host-` prefix at the defaults, threat T17 | **High** | [RFC 004](./rfcs/done/004-form-token.md) | verified |
| P-31 | **Regression shipped in 0.4.0:** with a success page configured, a honeypot hit was distinguishable from success — **fixed** in RFC 006 handoff 01 C1 (`67c1ac1`), threat T18.  Called out as a security fix in the 0.5.0 release notes; no formal advisory (owner, 2026-09-13: no production use yet beyond an internal team) | **High** | RFC 006 handoff 01 | fix reproduced |

### Past decisions

- 2026-09-12: milestones M1–M3 and their priorities approved.
- 2026-09-12: anti-forgery token direction set in RFC 004; bundled challenge providers in scope.
- 2026-09-12, 2026-09-13: versions 0.3.4, 0.4.0 and 0.5.0 approved and released.
- 2026-09-13: M4 theme approved, test strategy first.
- 2026-09-13: RFC 008 — browser tests in CI on every push; mutation testing once per milestone before the release candidate, informational.
- 2026-09-13: email — stricter syntax approved (P-34); opt-in mail-domain check approved at lower priority (P-35); confirm-before-forward recorded only.  Untranslatable messages (P-36) wait for the app team's confirmation.
- 2026-09-13: RFC 008 complete.  M4 ships as 0.6.0: the 0.4 names removed (P-37), P-34, P-33 (approved) and P-32 (approved; RFC 009).  RFC 010 accepted for the first three.
