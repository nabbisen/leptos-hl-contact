# Handoff 013-01 — Documentation, the example's missing-secret arm, one server test

**RFC.** [RFC 013](../../done/013-production-report-corrections.md) D1–D4
**Roadmap.** P-42
**Requirements.** FR-ABUSE-05, FR-ABUSE-12, NFR-PORT-02

## Goal

- **The docs match production experience:** size, rate limiting, and who
  tests on workerd.
- **The example fails closed** when a site key is set without a secret.
- **A server test** pins that behaviour.

## Change scope

### 1. `docs/src/guides/cloudflare-workers.md`

**"Bundle size" (currently lines 49–58), per RFC D2.**
- **First bullet: strip the debug names.**
  - In the release profile, `strip = "symbols"`, or per build with
    `CARGO_PROFILE_RELEASE_STRIP=symbols`.
  - **Why:** a release build keeps the wasm `name` section.  In one
    production Worker it was 55.1 MiB of a 59.1 MiB module; stripped, the
    module was 3.9 MiB.
- **Measured build bullet.**  Replace it with the stripped production
  figures: about 4.3 MiB uncompressed and 1.5 MiB gzipped, with all five
  features and Turnstile.
  - **The crate's share:** the challenge adds about 25 KiB uncompressed;
    the form token and delivery deadline add under 2 KiB.
  - **Source:** one integrator's application, `wrangler deploy --dry-run`,
    2026-09-17.
- **Keep** the limit bullet unchanged.
- **Keep** the size settings (`opt-level`, `lto`, `codegen-units`) after the
  strip bullet.

**"Rate limiting" (currently lines 234–262), per RFC D3.**
- **Replace the "counters" bullet** with Cloudflare's own sentence, quoted:
  "permissive, eventually consistent, and intentionally designed to not be
  used as an accurate accounting system"; and "a unique limit per Cloudflare
  location".  Link the binding page, which is already linked above.
- **Add "Verify it after deploying."**
  - Send more `POST`s than the limit from one address to the deployed
    Worker, and expect a `429`.
  - An integrator's production Worker returned none for 27 `POST`s in about
    a minute, at a limit of 5 per 60 s.  `wrangler dev` did refuse.
- **Add "A rate-limiting rule"** as the alternative, or as a second layer: a
  rule for the zone in the Cloudflare dashboard.
  - **Link:** <https://developers.cloudflare.com/waf/rate-limiting-rules/>.
  - **Do not state** plan allowances or prices.

**"What this project tests" (currently lines 330–338), per RFC D1.**
Replace "verified by the reflerd.com team before each release, and otherwise
relies on reports from integrators" with three points:
- **0.7.0:** tested on workerd by an integrator, on a pre-release and in
  production.
- **Later releases:** they rely on the headless-Chrome suite above and on
  integrators' reports.
- **Unchanged:** the issue invitation.

### 2. `docs/src/development/testing.md`, "Worker tests" (currently line 172)

- **Replace** "workerd itself is verified by the reflerd.com team before a
  release (NFR-PORT-02)" with the same fact and limit as §1.
- **Do not change** the traceability row for NFR-PORT-02.

### 3. `docs/src/security/challenge.md`, "What the server decides" (RFC D4)

After the decision list, add a paragraph headed **A missing secret**.
- **Why the obvious fix is wrong.**  Leaving out `ChallengeContext` when the
  secret is missing refuses only submissions that carry a token.  A bot that
  posts without one passes.
- **The rule.**  If the widget renders, provide the context.  A missing
  secret goes to `HttpChallengeVerifier::new` as an empty string.
  - **With a token:** refused as `challenge_unavailable`, with an `error`
    log.
  - **Without a token:** refused as `challenge_required` under
    `NoJsPolicy::Reject`.
  - **Either way,** nothing is delivered.
- **Link Hardening's "Secrets"**, whose advice is to refuse to start when a
  secret is missing.  Say that it is the better choice where possible.  A
  Worker has no start-up to refuse, so there the empty secret is the pattern.
- **A short code sketch** is welcome, for example
  `let secret = env_secret.unwrap_or_default();`.  Mark it `rust,ignore`, as
  the page's other snippets are.

### 4. `docs/src/getting-started/production-checklist.md`

- **The challenge row** (currently line 22): append "; `ChallengeContext`
  provided whenever the widget renders, with an empty secret if the secret
  is missing".  Link `challenge.md`'s new paragraph.
- **The Workers rate-limiting row** (currently line 32) becomes "Rate limit
  on `POST` verified on the deployed Worker: the Rate Limiting binding
  before the router, or a rate-limiting rule".

### 5. `examples/axum-with-security/src/main.rs` (currently lines 207–243)

- **The `(Some(_), None)` arm** returns `Some(ChallengeContext { … })`:
  - `HttpChallengeVerifier::new(provider, "")`;
  - the same provider parsing and `expected_action` as the
    `(Some(_), Some(secret))` arm.  Share the construction instead of
    copying it, for example by matching the secret to `String::new()`.
- **The warning** says: the widget renders, and every submission is refused
  until `CHALLENGE_SECRET` is set.
- **The block comment** says why the context is still provided: leaving it
  out would let submissions without a token through.
- **Nothing else changes.**  Check the header comment (lines 14–17) still
  reads correctly.

### 6. `crates/leptos-hl-contact/tests/server/challenge.rs`

Add one test, gated with `#[cfg(feature = "challenge-http")]` (the suite's
`main.rs` does not require that feature).
- **Name:** for example
  `a_widget_with_an_empty_secret_refuses_with_and_without_a_token`.
- **Setup:** `Setup { challenge: Some(ChallengeContext { verifier:
  Arc::new(HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "")),
  policy: ChallengePolicy::default() }), ..Setup::default() }`.  The default
  policy is `NoJsPolicy::Reject`; assert that in the test.
- **Without a token,** through `expect_both`: `challenge_required`, with the
  banner that row 3 already uses.
- **With a token,** through `expect_both`: `challenge_unavailable`, with the
  banner that row 7 already uses.
- **After both:** `h.deliveries() == 0`.
- **No network.**  An empty secret sends nothing: the unit test
  `an_empty_secret_is_misconfigured_and_sends_nothing` shows it.
- **Rustdoc:** FR-ABUSE-12 and RFC 013 D4, one sentence on the trap.

**Break check, required.**  Temporarily set the test's `challenge` to
`None`, the pattern the example used.  The without-token half must then fail,
because the submission is delivered.  Report the failure output, then
restore the test.

### 7. `docs/src/development/testing.md`, traceability

Add the new test to the FR-ABUSE-12 row's server column.

### 8. `CHANGELOG.md`

Add `## [Unreleased]` above `[0.7.0]`, with a *Documentation* section of four
lines:
- **Cloudflare Workers guide, bundle size:** strip the wasm `name` section,
  with production size figures.
- **Cloudflare Workers guide, rate limiting:** the binding is permissive, so
  verify it after deploying; a rate-limiting rule is the alternative.
- **Challenge and Production Checklist:** a missing secret must fail closed.
  Provide `ChallengeContext` with an empty secret, never leave it out.  The
  `axum-with-security` example did leave it out, and now does not.
- **Workers testing:** workerd testing is stated as done for 0.7.0 by an
  integrator, not promised for each release.

## Gates

- **The shared gates** from handoff 001's README, on both toolchains.
- **`cargo test --all-features`:** the new test runs and passes.  Report the
  server suite's count, 27 → 28.
- **The example,** as CI runs it:
  `RUSTFLAGS=-D warnings cargo check --locked --features ssr` in
  `examples/axum-with-security`.
- **`mdbook build docs`:** no warnings.
- **The grep** `git grep -n "reflerd.com team before" -- docs` prints
  nothing.

## Review request

Write `.git-exclude/review-request/013-production-report-corrections/01-docs-example-and-test.md`:
- the commit;
- each section above with its before and after text, or a diff excerpt;
- the break check's output;
- the gate results.
