# RFC 013 — Corrections from the first production report on Cloudflare Workers

**Status.** Accepted — 2026-09-17.  The owner approved the scope (P-42) on
the architect's assessment of the reflerd.com team's letter the same day.
Documentation, the security example and one server test; no crate code, no
public API change, no release required.
**Tracks.** Roadmap P-42.  Requirements FR-ABUSE-05, FR-ABUSE-12,
NFR-PORT-02.
**Touches.** `docs/src/guides/cloudflare-workers.md`,
`docs/src/security/challenge.md`, `docs/src/getting-started/production-checklist.md`,
`docs/src/development/testing.md`, `examples/axum-with-security/src/main.rs`,
`crates/leptos-hl-contact/tests/server/challenge.rs`, `CHANGELOG.md`.
**Handoffs.** [`../handoffs/013-production-report-corrections/README.md`](../handoffs/013-production-report-corrections/README.md)

## Summary

The reflerd.com team has run 0.7.0 in production on a Cloudflare Worker since
2026-09-17.  They found no defect in the crate.  Their letter shows four places
where our documentation, or our example, leads an integrator wrong.  One of
them lets bots past a challenge on a misconfigured site.

Source: `.git-exclude/upstream/reflerd/receive/2026-09-17-reflerd-0.7.0-in-production.md`;
assessment: `.git-exclude/reviewed/reflerd-letter-2026-09-17-production.md`.

## D1 — Do not promise runtime testing no one committed to

- **Today.**  The Workers guide ("What this project tests") and `testing.md`
  ("Worker tests") say the reflerd.com team verifies workerd behaviour
  before each release.  The team has said it cannot commit to that.
- **Change.**
  - **State the fact:** 0.7.0 was tested on workerd by an integrator, on a
    pre-release and then in production.
  - **State the limit:** later releases rely on CI's wasm32 suite in
    headless Chrome and on reports from integrators.
  - **Keep** the invitation to open an issue.
- **Unchanged:** the traceability row for NFR-PORT-02 names "the reflerd.com
  team's report before 0.7.0", which is a fact.

## D2 — Bundle size: strip the debug names

- **Measured by the architect** (a release `cdylib` for
  `wasm32-unknown-unknown`): a release build keeps the `name` custom
  section, and `strip = "symbols"` removes it.
- **Measured by the integrator** on a production Worker:
  - **Unstripped:** 59.1 MiB uncompressed, of which the `name` section was
    55.1 MiB.  That is 92 % of Cloudflare's 64 MiB limit.
  - **Stripped:** 3.9 MiB uncompressed, 1.3 MiB gzipped, with code and data
    changed by under 0.2 %.
- **Change** the guide's "Bundle size":
  - **Add `strip = "symbols"`** to the release-profile advice, first, with
    the reason and those figures.  It can also be set per build:
    `CARGO_PROFILE_RELEASE_STRIP=symbols`.
  - **Replace** "about 3 MiB gzipped in the reflerd.com team's test build"
    (an unstripped test build) with their stripped production build: about
    4.3 MiB uncompressed and 1.5 MiB gzipped, all five features and
    Turnstile.
  - **Say what the crate costs:** the challenge added about 25 KiB
    uncompressed, and the form token with the delivery deadline under
    2 KiB.  Most of a Worker is Leptos and the application.
  - **Name the source:** one integrator's application, measured with
    `wrangler deploy --dry-run`, 2026-09-17; not a benchmark.
  - **Keep** the limit sentence and its link.

## D3 — Rate limiting: verify the binding after deploying

- **Observed.**  The binding refused the sixth POST on `wrangler dev`.  In
  production, with a limit of 5 per 60 s, 27 POSTs from one address within
  about a minute got no 429.
- **Cloudflare's own words** (binding page, fetched 2026-09-17): "permissive,
  eventually consistent, and intentionally designed to not be used as an
  accurate accounting system", with "a unique limit per Cloudflare
  location".
- **Change** the guide's "Rate limiting":
  - **Quote that sentence,** replacing the paraphrase "treat the limit as
    approximate".
  - **Add a step:** after deploying, send more POSTs than the limit from one
    address and confirm a 429.
  - **Offer the alternative:** a rate-limiting rule for the zone in the
    Cloudflare dashboard (WAF), instead of or in addition to the binding.
    Link Cloudflare's rate-limiting-rules documentation.  Do not restate
    plan allowances.
- **Change the checklist's Workers row:** "Rate limit on `POST` verified on
  the deployed Worker: the binding before the router, or a rate-limiting
  rule".

## D4 — A missing challenge secret must fail closed

- **The gate, as documented and unchanged.**
  - No `ChallengeContext` and no token: the form proceeds.
  - A token but no context: `not_configured`.
- **The trap.**  A site renders the widget but leaves out the context when
  its secret is missing.  It then refuses only submissions that carry a
  token.  A bot that posts without a token meets no challenge.
- **Our example does exactly this:** `axum-with-security`, `main.rs`, the
  `(Some(_), None)` arm.
- **The crate already fails closed when used right.**
  `HttpChallengeVerifier::new` accepts an empty secret and reports
  `Misconfigured` on verification.  Given a context with an empty secret
  under `NoJsPolicy::Reject`:
  - **a submission with a token:** `challenge_unavailable`, with an `error`
    log;
  - **one without a token:** `challenge_required`;
  - **either way:** nothing is delivered.
- **Change the Challenge page** ("What the server decides"): add a paragraph,
  **"A missing secret"**.
  - **The rule.**  Whether a challenge is on is decided by whether the
    widget renders.  If it renders, provide `ChallengeContext`.
  - **Missing secret.**  Pass it to `HttpChallengeVerifier::new` as an empty
    string, and every submission is refused.  Never leave the context out.
  - **Where refusing to start is possible,** Hardening's "refuse to start"
    advice is better still.
  - **On a Worker** there is no start-up to refuse, so the empty secret is
    the pattern.
- **Change the checklist's challenge row:** add "`ChallengeContext` provided
  whenever the widget renders, with an empty secret if the secret is
  missing".
- **Change the example.**  In the `(Some(_), None)` arm:
  - **Provide the context,** with an empty secret and the same provider
    and policy logic as the `(Some(_), Some(secret))` arm.
  - **Update the warning** so it is true: every submission is refused.
  - **Update the comment** at the top of that block.
- **Add a server test** (L2) through the documented router: a context
  holding `HttpChallengeVerifier` with an empty secret, under
  `NoJsPolicy::Reject`.
  - **Submissions:** one without a token and one with a token.
  - **Both request forms:** fetch and no-JavaScript.
  - **Expected:** `challenge_required` and `challenge_unavailable`,
    respectively, and zero deliveries.
  - **Trace** it under FR-ABUSE-12.

## Compatibility

No crate change.  The example's behaviour changes only when a site key is
set without a secret: submissions without a token are now refused too.

## CHANGELOG

`## [Unreleased]`, *Documentation*: one line each for D1–D4.  D4's line
names the example's fix.

## Not in scope

- **P-41,** an opt-in `novalidate` on the form: proposed separately.
- **A crate-level guard,** such as refusing a rendered widget without a
  context at request time.  The server cannot know that a widget was
  rendered when no token arrives.
