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

---

## Planned — proposed milestones (awaiting owner approval)

Source: architect baseline review on 2026-09-12 of commit `8d29d5a` (tag
`0.3.3`).  "Verified" means observed by running the toolchain or confirmed
by reading crate and framework source; "inferred" means a conclusion from
code reading that has not yet been reproduced by a test and must be
confirmed during implementation.

Documents produced from this review: [Requirements](./docs/src/development/requirements.md)
and [External Design](./docs/src/development/external-design.md).

### M1 — Green baseline (proposed release: 0.3.4, patch) — **authorized 2026-09-12**

Tracked by [RFC 001](./rfcs/accepted/001-m1-green-baseline.md); handoffs in
[`rfcs/handoffs/001-m1-green-baseline/`](./rfcs/handoffs/001-m1-green-baseline/README.md).

No public API change.  Goal: CI gates green, documentation truthful,
per-field errors visible.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-01 | Make CI gates green: rustfmt diffs in most files, four `clippy -D warnings` errors (`field_reassign_with_default` ×2 in `server.rs`, unused import in `model/tests.rs`, unused variable in `csrf/tests.rs`), failing `sanitize_header_value` doctest (CRLF becomes two spaces) | **High** | fix | verified locally, rustc 1.98.1 |
| P-02 | Per-field validation errors never render on the client: `components.rs` tests `starts_with("field_errors:")` on `ServerFnError`'s `Display` output, which the framework prefixes with `"error deserializing server function arguments: "`, so the generic banner is shown instead | **High** | fix + test | verified by reading `server_fn` 0.8 `Display` impl; needs an integration reproduction as evidence |
| P-03 | Documentation drift sweep. **Book and README done 2026-09-12** during the docs restructure (FAQ CSRF answer, csrf fail-closed text, `SmtpTlsMode::None`, custom-backend example, Turnstile guide rewritten, tag prefix, feature tables, version strings, `submit_contact` signature). **Remaining (code files, dev team):** `lib.rs` crate-level feature table lacks `csrf` and its security-doc link points to the old `docs/src/security.md` path; `security.rs` header comment is stale; `leptos_axum` doc examples are not ours to fix | **High** | docs | verified |
| P-04 | `ContactServerPolicy.max_message_len` compares bytes (`String::len`) while the validator counts characters (`chars().count()`); align to characters | Medium | fix + test | verified (validator 0.20 source) |
| P-05 | Release-record hygiene: CHANGELOG labels 0.2.0–0.3.3 "Unreleased" although tagged; example crates at 0.3.2 while workspace is 0.3.3 | Medium | docs | verified |
| P-06 | RFC directory scaffolding per RFC-000 5-folder variant: `rfcs/README.md` index and state folders | Medium | governance | done 2026-09-12 with this roadmap update |
| P-07 | `ContactFormOptions.max_message_len` and `ContactServerPolicy.max_message_len` are documented as "must not exceed 4000" but nothing enforces or clamps it | Low | fix | verified |
| P-08 | `docs/book.toml` `git-repository-icon` rejected by mdbook 0.5 in every tried form — **done 2026-09-12**: key removed, default icon used; book builds | Low | docs | verified |
| P-09 | Both examples use the Axum 0.7 wildcard `"/api/*fn_name"`; Axum 0.8 panics at router construction ("Path segments must not start with `*`"), so neither example starts.  Change to `"/api/{*fn_name}"`; examples are outside the workspace so CI never built them — add a CI job that at least `cargo check`s both | **High** | fix + CI | verified (axum 0.8.9 source) |

### M2 — Form robustness and anti-abuse redesign (proposed release: 0.4.0, minor)

Behavioural or API changes.  Each item needs an accepted RFC before a
handoff is written.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-10 | Form state preservation: the entire form subtree is rebuilt whenever the action value changes, so on a validation error in WASM mode the visitor's typed input is discarded | **High** | [RFC 002](./rfcs/accepted/002-form-state-model.md) | inferred from `components.rs` closure dependencies |
| P-11 | Hidden `csrf_token` field becomes empty after that client-side rebuild because `CsrfToken` context exists only during SSR; the second submit then fails with "reload the page" | **High** | RFC (with P-10) | inferred; tachys confirmed to keep SSR attribute on first hydration only |
| P-12 | Anti-forgery token redesign: the token is not bound to the visitor and is replayable within its TTL, so it does not prevent cross-site forgery on its own; decide between cookie binding, session binding, or repositioning it as an anti-automation "form token" with the Origin check as the documented CSRF control | **High** | RFC | verified by design reading |
| P-13 | Progressive-enhancement contract: without JavaScript the framework redirects back to the Referer, errors are surfaced via `__err`, but a successful submit shows no confirmation | Medium | RFC (with P-10) | inferred from `server_fn` / `leptos_server` source |
| P-14 | Localisable server-originated messages: validation, policy, token and configuration messages are English strings composed on the server and cannot be overridden through `ContactFormLabels`; move to error codes mapped to text on the client | Medium | [RFC 003](./rfcs/accepted/003-error-codes.md) | verified |
| P-15 | Test strategy: integration tests for `submit_contact` (CSRF fail-closed, policy, honeypot, delivery error), component render tests, no-JS flow; current server tests only check string sentinels | Medium | RFC / handoff | verified |
| P-16 | Focus management after a failed submission (move focus to first invalid field or error summary) | Low | RFC (with P-10) | design gap |

### M3 — Anti-abuse (proposed release: 0.5.0, minor) — **theme authorized 2026-09-12**

Owner decisions of 2026-09-12: theme approved and positioned directly
after M2; first-release providers are Cloudflare Turnstile, hCaptcha,
reCAPTCHA v2 and v3 (not Enterprise); when a challenge is enabled,
no-JavaScript submissions are rejected with a `<noscript>` message,
fail-closed, with an explicit opt-in to accept them under honeypot-only
protection; an HTTP client dependency is accepted behind a
`challenge-http` feature.

All three RFCs (004, 005, 006) were accepted on 2026-09-12; handoffs exist for each.

| ID | Item | Priority | Kind | Evidence |
|----|------|----------|------|----------|
| P-12 | Anti-forgery token redesign **plus token minimum age**: decide binding (cookie / session) or reposition as anti-automation "form token" and rename; reject submissions younger than a configurable number of seconds since the page render (JS-free bot signal); resolve the client-side-navigation case where a form created in the browser has no SSR token (new finding 2026-09-12) | **High** | [RFC 004](./rfcs/accepted/004-form-token.md) | verified by design reading |
| P-21 | Challenge providers: `challenge` prop renders the widget and a hidden token field inside the form; `ChallengeVerifier` trait; built-in Turnstile, hCaptcha, reCAPTCHA v2/v3 behind `challenge-http`; fail-closed; no-JS policy per owner decision; vendor test keys in CI; verify-endpoint timeout and outage behaviour defined | **High** | [RFC 005](./rfcs/accepted/005-challenge-providers.md) | decided |
| P-25 | Pre-delivery filter hook: `ContactFilter` trait returning accept / reject / silent-drop for a validated submission, for content heuristics or third-party spam services | Medium | [RFC 006](./rfcs/accepted/006-contact-filter.md) | new 2026-09-12 |

### M4 — Reach (proposed release: 0.6.x)

| ID | Item | Priority | Kind |
|----|------|----------|------|
| P-20 | Multi-language label presets (GUI rule requires i18n; depends on P-14 for server messages) | Medium | RFC |
| P-22 | HTTP-API delivery adapters: Resend, SendGrid, AWS SES (existing Future items) | TBD | RFC per adapter |
| P-23 | Cloudflare Workers compatibility: `lettre` with tokio and native TLS cannot run on Workers; requires a fetch-based delivery adapter and a runtime-neutral core; owner decision on target platforms | TBD (owner decision) | RFC |
| P-26 | Scheduled CI job running the `#[ignore]` live tests against challenge-vendor test keys; deferred by the owner on 2026-09-12 because it carries a cost; needs its own RFC | TBD (owner) | RFC |
| P-24 | Dependency and CI hygiene: `rand` 0.8 → 0.9, consider `subtle` for constant-time comparison, CI matrix on MSRV 1.85 plus stable instead of Debian `rustc-1.91` only | Low | task |

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
