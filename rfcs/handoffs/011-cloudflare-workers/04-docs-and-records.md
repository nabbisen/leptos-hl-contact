# Handoff 011-04 — Workers guide, CSP directives, records

**RFC.** [RFC 011](../../accepted/011-cloudflare-workers.md) D8, D9
**Roadmap.** P-23, P-40
**Requirements.** NFR-PORT-02, NFR-DOC-01, NFR-DOC-03, NFR-PRIV-02,
FR-CFG-01
**Depends on.** Handoff 03 merged.
**Code changes.** None, apart from rustdoc if a guide link needs it.

## Goal

An integrator can put the form on Cloudflare Workers from the book alone:
- **Which features work.**
- **Build settings:** how to configure the dependencies and the
  `getrandom` flag.
- **Wiring:** how to provide context, the client IP and a delivery
  backend.
- **Security:** how to rate-limit, and which Content Security Policy
  directives the challenge widgets need.

The records say Workers is supported and how it is verified.

## Change scope

### 1. New guide — `docs/src/guides/cloudflare-workers.md`

Add it to `SUMMARY.md` under Guides, after Axum Integration.  Sections:

1. **What works.**  RFC 011 D1's feature table.  `smtp-lettre` does not
   work there; delivery is your own backend (link Delivery Backends).
2. **Dependencies.**  A `Cargo.toml` snippet:
   - `leptos-hl-contact` with the Workers features;
   - `leptos_axum` with `default-features = false, features = ["wasm"]`;
   - `axum` without default features.

   Then the build flag, with where to put it
   (`.cargo/config.toml` `[target.wasm32-unknown-unknown] rustflags`):
   `RUSTFLAGS='--cfg getrandom_backend="wasm_js"'`.
3. **The context closure on a Worker.**  The same values as Axum
   Integration.  The delivery is the site's own, and its future need not
   be `Send`; show a skeleton with the `DeliveryFuture` alias.
4. **The client IP.**  Provide `ChallengeClientIp` from `CF-Connecting-IP`.
   Say why the crate never reads the header itself.
5. **Rate limiting.**  The Workers Rate Limiting binding, keyed by
   `CF-Connecting-IP`.
   - **Link** Cloudflare's documentation; check the URL when you add it.
   - **Placement:** the check sits before the router.
6. **The delivery deadline.**  `DeliveryTimeout` works on Workers; a sensible
   limit relative to the Worker's own limits.  Link Cloudflare's limits
   page and check it.
7. **Testing locally.**  `wrangler dev`.  This project tests the wasm32
   server path in headless Chrome and relies on integrators for workerd
   runtime reports.  Say that plainly.

### 2. Content Security Policy — `docs/src/security/challenge.md`

A section "Content Security Policy", with one row per provider:

| Provider | Directives |
|----------|-----------|
| Turnstile | `script-src https://challenges.cloudflare.com` and `frame-src https://challenges.cloudflare.com`.  A nonce on the `api.js` script propagates to what it loads; `'strict-dynamic'` is supported, not required.  `connect-src 'self'` only in pre-clearance mode.  Source: Cloudflare Turnstile CSP reference, checked 2026-09-15; re-check the URL and date when you write it |
| hCaptcha | from hCaptcha's CSP documentation — cite it |
| reCAPTCHA v2 / v3 | from Google's reCAPTCHA CSP documentation — cite it |

**Then add:**
- **The nonce.**  `ChallengeWidget`'s nonce goes on the vendor script and on
  the reCAPTCHA v3 inline script.
- **The honeypot.**  Under a policy without `'unsafe-inline'`, set
  `ContactFormOptions::honeypot_inline_style` to `false` and hide the class
  (RFC 012; link Styling).

**Do not guess any directive.**  If a vendor's page does not state one,
write "not documented by the vendor" and report it.

### 3. Production Checklist — `docs/src/getting-started/production-checklist.md`

A "Cloudflare Workers" block of rows:
- the feature set;
- the `getrandom` flag;
- the client IP from `CF-Connecting-IP`;
- rate limiting with the binding;
- CSP per the challenge page;
- `DeliveryTimeout` around your backend.

### 4. Feature Flags — `docs/src/reference/feature-flags.md`

A "Workers" column: supported / not supported, as RFC 011 D1.

### 5. Records

**`docs/src/development/requirements.md`:**
- **NFR-PORT-02 text:** RFC 011 D9's wording.
- **Status:** "Met (0.7.0): compiles and is linted for wasm32 in CI; the
  wasm32 server paths tested in headless Chrome; runtime on Workers verified
  by the reflerd.com team before release (see the 0.7.0 readiness report)".
- **Change-history row.**

**`docs/src/development/external-design.md`:**
- **§1.2 responsibility split:** client IP and rate limiting on Workers are
  the application's.
- **§4.4.1 trait contract:** the future aliases.
- **§4.5.1 feature flags:** the Workers column.
- **§5.2 threat model:** the RFC 005 redirect row states the wasm32 path.

**`docs/src/development/architecture.md`:** the three build paths (native,
browser, wasm32 server), with `wasm_timer.rs` in the source layout.

**`docs/src/development/testing.md`:**
- the `tests/worker/` suite section, next to the browser suite;
- the traceability row for NFR-PORT-02 (now MUST);
- the counts reported.

**`README.md`:** one line in Design notes: runs on Cloudflare Workers; link
the guide.

### 6. CHANGELOG `[Unreleased]`

*Documentation:* the Workers guide, the CSP directives, and the checklist.

## Acceptance

1. **Book.**  `mdbook build docs` passes with no warnings; the guide is in
   the summary.
2. **Links.**  Every external URL added is listed in the review request with
   the date checked.  Every directive has a cited source, or is marked
   "not documented by the vendor".
3. **Snippets.**
   - **Checkable ones compile.**  Put the guide's `Cargo.toml` and
     context-closure snippets in a scratch crate and run
     `cargo check --target wasm32-unknown-unknown` with the flag.  Report
     it.
   - **The rest** is fenced `rust,ignore` and says why.
4. **Traceability.**  The table has 103 MUST rows (NFR-PORT-02 added), and
   every cited test resolves.
5. **Gates and both suites** pass; the code is unchanged.

## Review request

`.git-exclude/review-request/011-cloudflare-workers/04-docs-and-records.md`
