# Requirements Specification

> **Document status.** Draft 11, 2026-09-13, against release `0.5.0`.
> First drafted against baseline `0.3.3` (commit `8d29d5a`).  Milestones
> M1–M3 are released and M4 is owner-authorized; the document as a whole
> awaits formal approval.
> Once approved, this document is the requirements baseline; later changes
> go through RFCs listed in [`rfcs/README.md`](https://github.com/nabbisen/leptos-hl-contact/blob/main/rfcs/README.md).
>
> **Audience.** Project owner, architect, dev team, contributors.
>
> **Conventions.** MUST / SHOULD / MAY follow RFC 2119.  Every requirement
> carries a status against the baseline:
>
> | Status | Meaning |
> |--------|---------|
> | **Met** | Implemented and observable in the release named in the status, or in `0.3.3` when none is named |
> | **Partial** | Implemented with a known defect or incomplete coverage |
> | **Gap** | Required but not implemented |
> | **Planned** | Not yet required to be implemented; scheduled in the roadmap |
> | **Decision** | Cannot be finalised until the owner decides an open question |
>
> A dagger (†) marks a status that rests on reading crate or framework
> source and has not yet been reproduced by an automated test.  `P-nn`
> references point to [`ROADMAP.md`](https://github.com/nabbisen/leptos-hl-contact/blob/main/ROADMAP.md).

---

## 1. Purpose and scope

`leptos-hl-contact` gives a Leptos 0.8 application a complete contact form:
an accessible UI component, a server function that validates and filters
submissions, and a pluggable delivery layer that hands the validated
submission to SMTP or any other backend.

This document states **what** the crate must do and guarantee.
[External Design](./external-design.md) states how that appears at the
crate's boundaries.  [Architecture](./architecture.md) describes internals.

### In scope

- The form UI, its states, and its customisation surface.
- Server-side processing of a submission up to and including hand-off to a
  delivery backend.
- Built-in delivery backends: no-op and SMTP.
- Anti-abuse controls that can live inside the crate, and explicit
  requirements on what the hosting application must add.
- Internationalisation, accessibility, progressive enhancement.
- Documentation, testing, and release quality expectations.

### Out of scope

- Storing submissions, an admin panel, attachments, a form builder
  (declared non-goals in [Introduction](../introduction.md)).
- Operating the SMTP relay, DNS (SPF/DKIM/DMARC), TLS termination.
- Application-wide session management.

---

## 2. Stakeholders and personas

| Persona | Who | Primary needs |
|---------|-----|---------------|
| **Integrator** | Developer embedding the form in a Leptos + Axum (or other) app | Minimal wiring, safe defaults, styling and text control, clear failure modes, no surprises in production |
| **Visitor** | Person submitting the form, on any device, with or without JavaScript, possibly using assistive technology | Short form, clear feedback, no lost input, privacy |
| **Operator** | Person or team receiving the email | Readable message, working `Reply-To`, no spam flood, no forged sender |
| **Adversary** | Spam bots, header-injection attempts, cross-site forgery, floods | (to be defeated) |
| **Maintainer** | Owner, architect, dev team | Simple structure, tests derived from design, truthful documentation |

---

## 3. Goals and non-goals

### Goals

1. **Secure by default.** No credential or recipient address ever reaches
   WASM; every submission is validated on the server; errors shown to the
   visitor never reveal internals.
2. **Complete but minimal.** One component, one server function, one trait.
   Everything else is opt-in behind feature flags.
3. **Works everywhere Leptos SSR works.** Plain HTML POST without
   JavaScript; hydrated and Islands modes with it.
4. **Accessible and localisable by default**, not as add-ons.
5. **Extensible delivery** without touching UI or server function.

### Non-goals

- Being a general form library.
- Replacing application-level rate limiting, TLS, or a WAF.
- Bundling a CAPTCHA **by default**.  Opt-in challenge providers are in
  scope since 2026-09-12 (FR-ABUSE-10 to FR-ABUSE-14); the default form
  still loads no third-party script.

---

## 4. Definitions

| Term | Definition |
|------|------------|
| **Submission** | One POST of the form's fields to the server function |
| **Character** | One Unicode scalar value (Rust `char`).  All length limits in this document count characters, not bytes |
| **Context closure** | The closure the hosting application passes to `leptos_routes_with_context`.  With Axum it serves both server functions and server rendering, so it is the one place context is provided (RFC 007; before 0.4.0 the documentation wrongly described two sites) |
| **Fail-closed** | On missing or invalid security configuration the crate refuses the submission rather than proceeding unprotected |
| **PII** | Personally identifiable information: visitor name, email, message body, IP address |
| **Form token** | The value carried in the hidden `form_token` field when the `form-token` feature is enabled.  Called the anti-forgery or CSRF token before 0.5.0 |

---

## 5. Functional requirements

### 5.1 Form presentation (FR-UI)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-UI-01 | The form MUST present `name`, `email`, `message` as required fields and `subject` as an optional field that the integrator can hide or make required | MUST | Met |
| FR-UI-02 | Every visitor-visible string rendered by the component MUST be overridable by the integrator | MUST | Met (see FR-I18N-02 for server-originated strings) |
| FR-UI-03 | Every structural element MUST expose a CSS class hook; the crate MUST NOT ship mandatory CSS | MUST | Met |
| FR-UI-04 | The form MUST expose these states: idle, pending, success, field-error, generic-error | MUST | Met (RFC 002 handoff 02) |
| FR-UI-05 | While pending, the submit button MUST be disabled, announce busy state, and change its text | MUST | Met |
| FR-UI-06 | On success the form MUST be replaced by a success message announced politely to assistive technology | MUST | Met (inline with JS; navigation to a configured success page in both modes) |
| FR-UI-07 | On a validation failure the visitor's typed input MUST be preserved | MUST | Met (verified in the browser; regression guard) |
| FR-UI-08 | Each failed field MUST show its error message adjacent to the field | MUST | Met (M1, handoff 001-03) |
| FR-UI-09 | Delivery and configuration failures MUST show one generic message and MUST NOT reveal internal detail | MUST | Met |
| FR-UI-10 | The honeypot field MUST be invisible to sighted visitors, hidden from assistive technology, excluded from tab order, and excluded from autofill | MUST | Met |
| FR-UI-11 | Integrator options MUST include: show subject, require subject (UI), maximum message length (UI) | MUST | Met |
| FR-UI-12 | When an anti-forgery token is configured the form MUST carry it in a hidden field, and the field MUST remain valid for the lifetime of the rendered form, including after client-side re-renders | MUST | Met: SSR-then-hydrate (0.4.0); client-side navigation and refresh with `token_refresh_secs` (0.5.0) |
| FR-UI-13 | After a failed submission, focus SHOULD move to the first invalid field or to an error summary | SHOULD | Met (`focus_first_error`) |

### 5.2 Submission processing (FR-SUB)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-SUB-01 | Submissions MUST be handled by one server function at a stable path, accepting a standard form-encoded POST | MUST | Met (`POST /api/submit_contact`) |
| FR-SUB-02 | Processing order MUST be: anti-forgery check → normalisation → honeypot → field validation → server policy → delivery.  No step may be skipped by client input | MUST | Met |
| FR-SUB-03 | Normalisation MUST trim surrounding whitespace from every text field and treat a blank subject as absent | MUST | Met |
| FR-SUB-04 | A non-empty honeypot MUST produce a response indistinguishable from success, MUST NOT deliver, and MUST log at `warn` without content | MUST | Met |
| FR-SUB-05 | Field validation MUST run on every submission regardless of what the client enforced | MUST | Met |
| FR-SUB-06 | Validation failures MUST be reported per field with generic messages that never echo input | MUST | Met (M1) |
| FR-SUB-07 | A server-side policy (`require_subject`, `max_message_len`) MUST be enforceable independently of UI options | MUST | Met (M1) |
| FR-SUB-08 | If the delivery backend is not configured the server MUST log at `error` and return the generic configuration message | MUST | Met |
| FR-SUB-09 | Delivery errors MUST be logged with their category and transport detail and MUST reach the client only as the generic delivery message | MUST | Met |
| FR-SUB-10 | The crate MUST NOT read environment variables or files itself; all configuration is supplied by the integrator through typed values and Leptos context | MUST | Met |

### 5.3 Validation rules (FR-VAL)

| ID | Field | Rule | Level | Status |
|----|-------|------|-------|--------|
| FR-VAL-01 | `name` | 1–80 characters after trimming; MUST NOT contain CR or LF | MUST | Met |
| FR-VAL-02 | `email` | Syntactically valid email address after trimming | MUST | Met |
| FR-VAL-03 | `subject` | When present: 1–120 characters; MUST NOT contain CR or LF.  Blank becomes absent | MUST | Met |
| FR-VAL-04 | `message` | 1–4 000 characters after trimming | MUST | Met |
| FR-VAL-05 | `website` (honeypot) | MUST be empty | MUST | Met |
| FR-VAL-06 | `form_token` | When the `form-token` feature is enabled: MUST be present, verify, and be at least `min_age_secs` old | MUST | Met (0.5.0) |
| FR-VAL-07 | *all* | Length limits MUST be counted in characters consistently by the UI `maxlength`, the validator, and the server policy | MUST | Met (M1) |
| FR-VAL-08 | `message` | 4 000 characters is the hard ceiling; UI options and server policy MUST NOT be able to raise it and SHOULD be clamped or rejected if they try | MUST | Met (M1: `MESSAGE_MAX_LEN`, clamped) |

Known tolerance: browsers count `maxlength` in UTF-16 code units, which is
never fewer than the character count, so the browser limit is equal to or
stricter than the server limit.  This never causes a server rejection of
input the browser accepted.

### 5.4 Anti-abuse (FR-ABUSE)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-ABUSE-01 | A honeypot MUST be built in and enabled without configuration | MUST | Met |
| FR-ABUSE-02 | The crate MUST either provide a request-forgery control that is effective on its own, or MUST state unambiguously that application-level Origin validation is the CSRF control and describe the built-in token as an anti-automation measure | MUST | Met (0.5.0): renamed form token; optional cookie binding with `__Host-` prefix as defence in depth; Origin validation documented as the control |
| FR-ABUSE-03 | Token verification MUST use constant-time comparison, MUST enforce a TTL, and MUST tolerate bounded clock skew | MUST | Met |
| FR-ABUSE-04 | When the `form-token` feature is enabled and its configuration is missing, submissions MUST be rejected (fail-closed) with a logged `error` | MUST | Met |
| FR-ABUSE-05 | Rate limiting is an application responsibility; the crate MUST document it and ship a working example | MUST | Met |
| FR-ABUSE-06 | Origin / Referer validation is an application responsibility; the crate MUST document a strict scheme+host+port comparison and ship a working example | MUST | Met |
| FR-ABUSE-07 | A request body size limit is an application responsibility; the crate MUST document it and include it in examples | MUST | Met |
| FR-ABUSE-08 | The crate MUST document CAPTCHA integration; superseded for adapters by FR-ABUSE-10 | MUST | Docs Met (rewritten 2026-09-12) |
| FR-ABUSE-09 | Bot-detection outcomes MUST NOT be distinguishable from success by the sender, including the success redirect when one is configured | MUST | Met (0.5.0); regressed in 0.4.0 when a success page was configured, fixed by `67c1ac1` |
| FR-ABUSE-10 | The crate MUST offer opt-in challenge verification through one abstraction (`ChallengeVerifier`) with built-in providers Cloudflare Turnstile, hCaptcha, reCAPTCHA v2 and v3; the component renders the widget and the token field, the server function verifies before delivery | MUST | Met (0.5.0, RFC 005; verified against real vendor endpoints) |
| FR-ABUSE-11 | When a challenge is enabled, a submission without JavaScript MUST be rejected with a `<noscript>` explanation, fail-closed; an explicit opt-in MAY accept such submissions under honeypot-only protection | MUST | Met (0.5.0, RFC 005); owner decision 2026-09-12 |
| FR-ABUSE-12 | Challenge verification MUST fail closed on a missing secret, a failed verification, or an unreachable verify endpoint, MUST be time-bounded, and MUST log the reason without the token | MUST | Met (0.5.0, RFC 005) |
| FR-ABUSE-13 | The crate SHOULD reject a submission that arrives sooner than a configurable minimum age after the page render, using the issue time already carried by the form token | SHOULD | Met (0.5.0, default two seconds) |
| FR-ABUSE-14 | The crate SHOULD offer a pre-delivery filter hook (`ContactFilter`) returning accept, reject, or silent drop for a validated submission | SHOULD | Met (0.5.0) |

### 5.5 Delivery (FR-DEL)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-DEL-01 | Delivery MUST be an object-safe async trait usable as `Arc<dyn …>` and `Send` | MUST | Met |
| FR-DEL-02 | A backend MUST receive only a fully validated and normalised submission | MUST | Met |
| FR-DEL-03 | A no-op backend MUST exist for development and tests and MUST log at `debug` without PII | MUST | Met |
| FR-DEL-04 | An SMTP backend MUST support STARTTLS, implicit TLS, and a conspicuously named plaintext mode for local development | MUST | Met |
| FR-DEL-05 | The email MUST use the server-configured `From` and `To`; the visitor's address MUST appear only in `Reply-To`, with the display name encoded safely; the subject MUST be prefix + sanitised subject | MUST | Met |
| FR-DEL-06 | The body MUST be plain text UTF-8 containing name, email, subject, and message | MUST | Met |
| FR-DEL-07 | Custom backends MUST be possible without modifying the UI or the server function | MUST | Met |
| FR-DEL-08 | Delivery SHOULD complete within a bounded time so a slow relay cannot hold a request indefinitely; delivery MAY be offloaded to a queue | SHOULD | Gap (no timeout; queue adapter is a Future item) |

### 5.6 Configuration and integration (FR-CFG)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-CFG-01 | Feature flags: `default = []`, `hydrate`, `ssr`, `islands`, `smtp-lettre` (implies `ssr`), `axum-helpers` (implies `ssr`), `form-token` (implies `ssr`; `csrf` is a deprecated alias).  Feature tables in docs and rustdoc MUST list all of them | MUST | Met (M1) |
| FR-CFG-02 | Required context values MUST be documented for the context closure, and helpers MUST exist for Axum | MUST | Met (RFC 007) |
| FR-CFG-03 | Misconfiguration MUST surface loudly (startup panic in examples, `error` log in the crate) and MUST NOT fall back to an insecure default | MUST | Met |
| FR-CFG-04 | Types holding secrets MUST redact them in `Debug` output | MUST | Met |
| FR-CFG-05 | Defaults MUST be the secure choice (STARTTLS; one-hour token TTL; policy off means "validator limits apply", never "no limits") | MUST | Met |

### 5.7 Internationalisation (FR-I18N)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-I18N-01 | All strings rendered by the component MUST be overridable per instance | MUST | Met |
| FR-I18N-02 | All visitor-visible strings that originate on the server (validation, policy, token, configuration, delivery messages) MUST be localisable by the integrator | MUST | Met (RFC 003: codes on the wire, labels on the client) |
| FR-I18N-03 | Presets for common languages SHOULD be offered | SHOULD | Planned (P-20) |
| FR-I18N-04 | The component MUST NOT hard-code text direction, locale formatting, or language attributes; these belong to the host page | MUST | Met |
| FR-I18N-05 | Unicode input MUST be accepted and preserved end to end, including correct encoding of non-ASCII display names and subjects in email headers | MUST | Met (lettre encodes headers) |

### 5.8 Accessibility (FR-A11Y)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-A11Y-01 | Every input MUST have a programmatically associated `<label>`; placeholders MUST NOT substitute for labels | MUST | Met |
| FR-A11Y-02 | Required inputs MUST carry both `required` and `aria-required="true"` | MUST | Met |
| FR-A11Y-03 | An invalid input MUST carry `aria-invalid="true"` and reference its error text via `aria-describedby` | MUST | Met (M1) |
| FR-A11Y-04 | Success uses a polite live region; generic errors use an assertive alert; field errors use polite alerts | MUST | Met |
| FR-A11Y-05 | The submit button MUST expose `aria-busy` while pending | MUST | Met |
| FR-A11Y-06 | The honeypot MUST be `aria-hidden` and outside the tab order | MUST | Met |
| FR-A11Y-07 | Only native form controls are used; focus outlines MUST NOT be suppressed by the crate | MUST | Met |
| FR-A11Y-08 | State MUST be conveyed in text, never by colour alone | MUST | Met |
| FR-A11Y-09 | Target: WCAG 2.2 AA for everything the crate renders; colour contrast is the integrator's responsibility because the crate ships no colours | MUST | Met (by construction) |

### 5.9 Progressive enhancement (FR-PE)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-PE-01 | The form MUST submit and be processed as a plain HTML POST when no JavaScript or WASM runs | MUST | Met |
| FR-PE-02 | Validation errors MUST be shown after a no-JS submission | MUST | Met (framework redirects to the Referer with the error encoded in the URL, which the server-rendered action reads; reproduced by `field_errors_round_trip_without_javascript`) |
| FR-PE-03 | Success MUST be shown after a no-JS submission | MUST | Met when a success page is configured (RFC 002 handoff 03); unconfigured deployments reload, documented |
| FR-PE-04 | Validation and delivery behaviour MUST be identical in both modes | MUST | Met |

### 5.10 Observability and privacy (FR-OBS)

| ID | Requirement | Level | Status |
|----|-------------|-------|--------|
| FR-OBS-01 | The crate MUST emit `tracing` events for: honeypot trigger (`warn`), validation failure (`debug`, flags only), token failure (`warn`), missing context (`error`), delivery failure (`error`), delivery success (`info`) | MUST | Met |
| FR-OBS-02 | Log events MUST NOT contain visitor name, email, message body, tokens, or secrets | MUST | Met |
| FR-OBS-03 | Delivery failures MUST include the error category and transport detail for operators | MUST | Met |
| FR-OBS-04 | The crate MUST NOT persist submissions | MUST | Met |

---

## 6. Non-functional requirements

### 6.1 Security (NFR-SEC)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-SEC-01 | SMTP credentials, recipient address, token secret MUST exist only in server-side types that are never serialised to the client | Met |
| NFR-SEC-02 | Email header injection MUST be prevented by validation and by defence-in-depth sanitisation at message build time | Met |
| NFR-SEC-03 | Cryptographic comparisons MUST be constant-time | Met |
| NFR-SEC-04 | Security features MUST fail closed | Met |
| NFR-SEC-05 | The threat model in [External Design §5](./external-design.md) MUST be updated whenever a change adds a data flow, an external integration, or authentication logic (project release rule) | Met (this document establishes it) |
| NFR-SEC-06 | The crate MUST NOT introduce a dependency that performs network I/O outside the delivery layer | Met |

### 6.2 Privacy (NFR-PRIV)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-PRIV-01 | PII MUST be minimised in logs and never persisted by the crate | Met |
| NFR-PRIV-02 | Any third-party processing added by an optional feature (challenge providers send visitor signals to the vendor) MUST be documented per provider, and the default form MUST load no third-party script | Met (0.5.0: per-provider notes in the challenge guide's Privacy section) |

### 6.3 Compatibility (NFR-COMPAT)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-COMPAT-01 | Leptos 0.8 only; SSR, hydrate, Islands | Met |
| NFR-COMPAT-02 | MSRV 1.85 (edition 2024); an MSRV bump is a minor release and is stated in the CHANGELOG | Met |
| NFR-COMPAT-03 | Axum 0.8 only through the optional `axum-helpers` feature; the core MUST stay framework-neutral | Met |
| NFR-COMPAT-04 | Semantic versioning in the 0.x range: a patch release MUST NOT change public API or observable behaviour except to fix defects; a minor release MAY break with a migration note | Met (policy) |
| NFR-COMPAT-05 | The DOM contract (element ids, field names, class hooks, ARIA attributes) is part of the public API; changes are breaking | Met (policy; see External Design §4.1) |

### 6.4 Portability (NFR-PORT)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-PORT-01 | The core (`default = []`) MUST compile for `wasm32-unknown-unknown` and native targets | Met |
| NFR-PORT-02 | Cloudflare Workers: the SMTP backend cannot run there (tokio, native TLS).  Whether a fetch-based delivery adapter and a runtime-neutral server path are required is an owner decision | Decision (P-23) |

### 6.5 Performance (NFR-PERF)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-PERF-01 | The request path MUST perform no blocking I/O; the only awaited operation is delivery | Met |
| NFR-PERF-02 | Per-request CPU work MUST be bounded by input size limits (validation is linear in input length) | Met |
| NFR-PERF-03 | Delivery latency is relay latency; it SHOULD be bounded (see FR-DEL-08) | Gap |

### 6.6 Dependencies (NFR-DEP)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-DEP-01 | Heavy dependencies (lettre, tokio, axum, crypto) MUST be optional behind features | Met |
| NFR-DEP-02 | Dependencies SHOULD be kept current and free of known advisories (`deps.rs` badge, periodic `cargo outdated`) | Partial (P-24; `rand` bump folded into RFC 004) |

### 6.7 Documentation (NFR-DOC)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-DOC-01 | Documentation MUST describe released behaviour; a release MUST include a docs-vs-code verification pass (project release rule) | Met (M1); re-verified at each release |
| NFR-DOC-02 | Every rustdoc example MUST compile and pass, or be marked `ignore` / `no_run` with reason | Met (M1) |
| NFR-DOC-03 | mdBook examples fenced as `rust` MUST compile against the current API or be fenced `rust,ignore` | Met (docs restructure 2026-09-12) |
| NFR-DOC-04 | `README.md` stays concise per the six-section structure; full docs live in `docs/src` for three personas | Met |

### 6.8 Testing and quality (NFR-TEST)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-TEST-01 | CI gates (fmt, clippy `-D warnings`, tests, doc) MUST be green on `main` at every tag | Met (M1; first green run 2026-09-12, plus a feature-combination clippy step and an examples job) |
| NFR-TEST-02 | Test cases MUST be derived from this specification and the external design, not from the code | Partial (P-15, RFC 008 D5) |
| NFR-TEST-03 | `submit_contact` MUST have integration tests covering: happy path, honeypot, each validation rule, policy, token fail-closed, token invalid, missing delivery context, delivery error | Partial (P-15): server suite `tests/server` (`7520763`); delivery error and two validation rules pending (RFC 008 handoff 01 review, C1–C2) |
| NFR-TEST-04 | Tests live in `src/<module>/tests.rs`, never inline (project rule) | Met |

### 6.9 Release (NFR-REL)

| ID | Requirement | Status |
|----|-------------|--------|
| NFR-REL-01 | Tags are `X.Y.Z` without prefix | Met (docs drift P-03) |
| NFR-REL-02 | CHANGELOG MUST record each released version with its date | Met (M1) |
| NFR-REL-03 | A release MUST pass the security audit step from the project rules and the docs verification pass | Met (exercised at 0.4.0 and 0.5.0) |

---

## 7. Constraints and assumptions

**Constraints (framework and platform facts)**

- Leptos runs server functions in a handler separate from the SSR renderer,
  but `leptos_routes_with_context` registers them itself and serves both with
  the same context closure, so one closure provides everything (RFC 007).
  Where a value must differ by request kind, the closure reads `Parts`.
- Leptos context does not exist in the browser; anything the client needs
  after hydration must be in the DOM or in reactive state.
- Without JavaScript, the framework answers a form POST with a redirect to
  the Referer; success carries no signal, errors are carried in the URL.
- `maxlength` in browsers counts UTF-16 code units.

**Assumptions**

- Deployments run behind TLS termination.
- One operator inbox per form instance.
- Integrators can set environment variables and add Axum layers.

---

## 8. Gap summary

| Requirement | Status | Roadmap item |
|-------------|--------|--------------|
| NFR-TEST-02, NFR-TEST-03 | Partial | P-15 / RFC 008 |
| FR-DEL-08, NFR-PERF-03 | Gap | P-32 (proposed), queue adapter (Future) |
| FR-I18N-03 | Planned | P-20 |
| NFR-PORT-02 | Decision | P-23 |
| NFR-DEP-02 | Partial | P-24 |

---

## 9. Open questions for the owner

1. ~~**Anti-forgery token (FR-ABUSE-02).**~~ Decided 2026-09-12 (RFC 004):
   form token with minimum age; cookie binding opt-in in `axum-helpers`.
2. **Target platforms (NFR-PORT-02).** Is Cloudflare Workers a supported
   target?  If yes, a fetch-based delivery adapter becomes a requirement.
3. ~~**Turnstile adapter.**~~ Decided 2026-09-12: challenge providers are in
   scope (Turnstile, hCaptcha, reCAPTCHA v2/v3), opt-in, no-JS rejected
   by default, HTTP client behind `challenge-http`.
4. **Message ceiling (FR-VAL-08).** Does 4 000 characters stay a hard
   constant, or should the ceiling become configurable upward with a
   documented maximum?
5. **First preset languages (FR-I18N-03).**

---

## 10. Change history

| Date | Version | Change |
|------|---------|--------|
| 2026-09-12 | Draft 1 | Initial specification from architect baseline review of `0.3.3` |
| 2026-09-13 | Draft 11 | RFC 008 handoff 01: NFR-TEST-03 Partial, FR-PE-02 reproduced by test.  Drift corrected: FR-ABUSE-10..12 and NFR-PRIV-02 Met at 0.5.0, NFR-REL-03 Met, document status, "context closure" definition, gap summary |
| 2026-09-13 | Draft 10 | FR-ABUSE-09 regression recorded; FR-ABUSE-14 Met |
| 2026-09-13 | Draft 9 | RFC 004 handoff 03: FR-UI-12 Met for client-side navigation |
| 2026-09-13 | Draft 8 | RFC 004 handoff 02: FR-ABUSE-02 Met |
| 2026-09-13 | Draft 7 | RFC 004 handoff 01: form-token naming, FR-VAL-06 and FR-ABUSE-13 Met |
| 2026-09-13 | Draft 6 | RFC 007 and RFC 003 handoff 01: FR-CFG-02 and FR-I18N-02 Met; the "two context sites" constraint corrected |
| 2026-09-12 | Draft 5 | RFC 002 handoff 03: FR-UI-06, FR-PE-03 Met; FR-CFG-02 downgraded to Partial pending RFC 007 (one context closure) |
| 2026-09-12 | Draft 4 | RFC 002 handoff 02: FR-UI-04, FR-UI-07, FR-UI-12 (SSR+hydrate), FR-UI-13 set to Met |
| 2026-09-12 | Draft 3 | M1 (RFC 001) implemented and approved: statuses for FR-UI-08, FR-SUB-06/07, FR-VAL-07/08, FR-CFG-01, FR-A11Y-03, NFR-DOC-01..03, NFR-TEST-01, NFR-REL-02 set to Met |
| 2026-09-12 | Draft 2 | Anti-abuse theme: FR-ABUSE-10 to FR-ABUSE-14 added, non-goal narrowed, NFR-PRIV-02 widened, open question 3 resolved |
