# Handoff 011-05 — Fixes from the runtime report: script nonce grammar, Workers notes

**RFC.** [RFC 011](../../done/011-cloudflare-workers.md), testing follow-up
**Origin.** The reflerd.com team's runtime report on pre-release `152d675`,
2026-09-16.  Every case they ran on a deployed Worker passed.  They found
one bug and four documentation gaps.
**Roadmap.** P-23
**Requirements.** FR-ABUSE-10, NFR-PRIV-02 (CSP), NFR-PORT-02, FR-OBS-01
**Depends on.** Handoffs 01–04 and both r2s (all approved).

## Goal

1. **Nonces.**  `ChallengeWidget::with_script_nonce` accepts every valid
   Content Security Policy nonce, including the ones Leptos generates.
2. **Docs.**  A Workers integrator learns from the book the four things the
   reflerd.com team had to find out themselves.

## 1. The bug — `with_script_nonce` rejects Leptos nonces

**What is wrong.**
- **The current check.**  `src/config.rs:428` accepts only
  `[A-Za-z0-9+/=]+`, through `is_base64` (`config.rs:487–491`).
- **What Leptos generates.**  Leptos 0.8 generates nonces with
  `alphabet::URL_SAFE, NO_PAD` (`leptos/src/nonce.rs:169`), which uses `-`
  and `_`.  About half of them contain one of those.
- **The effect.**  The widget renders without a nonce.  Under a nonce-only
  policy with `'strict-dynamic'`, the vendor script is blocked on about half
  of page loads.
- **Measured** by the reflerd.com team: 8 of 30 renders on `wrangler dev`,
  and 4 of 15 form loads on a deployed Worker, rendered without the nonce.
- **The standard allows it.**  CSP Level 3 defines
  `base64-value = 1*( ALPHA / DIGIT / "+" / "/" / "-" / "_" )*2( "=" )`
  (<https://www.w3.org/TR/CSP3/>, `nonce-source`).
- **Since when.**  The check has been there since 0.5.0, so 0.5.0 and 0.6.0
  are affected too.

**The fix — follow the CSP3 grammar exactly.**
- **The predicate.**  Replace `is_base64` with `is_csp_nonce(value) -> bool`:
  - one or more of `A–Z a–z 0–9 + / - _`;
  - then at most two `=`, only at the end.
- **The error text:** "script_nonce must be a CSP nonce
  (base64 or base64url: [A-Za-z0-9+/_-], up to two trailing '=')".
- **The rustdoc on `with_script_nonce`:**
  - it accepts base64 and base64url, as CSP Level 3 does;
  - `leptos::nonce::use_nonce()` values are accepted.
- **Scope.**  `components.rs:84` and `config.rs:474` check other values
  (element ids, action names).  Leave them.
- **One stricter case:** a `=` not at the end, such as `a=b`, is now
  refused.  Browsers ignore such a nonce anyway, so no working page
  changes.  Say so in the CHANGELOG line.

**Tests (`src/config/tests.rs`).**  Replace `widget_script_nonce_must_be_base64`
with `widget_script_nonce_follows_the_csp3_grammar`:

| Accepted | Refused |
|----------|---------|
| `abc123`, `rAnd0m+/nonce==`, `ZXhhbXBsZQ` (as before); `abc-DEF_123`; a 22-character URL-safe value with both `-` and `_`; `ab_=`; `ab-==` | empty; `a b`; `a"b`; `a'b`; `a<b`; `abc===`; `a=b`; `=abc` |

**Plus a render test** in `src/components/tests.rs`:
`a_url_safe_nonce_is_on_every_script_tag` builds the widget with
`abc-DEF_123`.  It asserts `nonce="abc-DEF_123"` on every script tag, the
same shape as `the_nonce_is_on_every_script_tag_when_set`.

**Deliberate break.**  Restore the old check.  Both new tests fail.

**CHANGELOG `[Unreleased]` → `### Fixed`:** "`ChallengeWidget::with_script_nonce`
refused base64url nonces (with `-` or `_`), such as about half of those
Leptos generates, so the widget rendered without a nonce and a nonce-only
Content Security Policy blocked it.  Since 0.5.0.  It now accepts exactly
the CSP Level 3 nonce grammar; a `=` other than trailing padding is now
refused, which browsers ignored anyway."

## 2. Documentation from the report

| # | Report finding | Where | What to write |
|---|---------------|-------|---------------|
| 2a | Logs are silent on a Worker (§4.3) | `guides/cloudflare-workers.md`, a new "Logs" section before "Testing locally" | The crate logs through `tracing`.  A Worker usually installs a `log` logger (such as `console_log`) and no `tracing` subscriber, so nothing prints.  Enable `tracing`'s `log` feature in your Worker's crate.  With no subscriber active, `tracing` then emits `log` records (tracing's crate docs, "Emitting log records").  The deadline and testing sections rely on these lines |
| 2b | The no-JavaScript error redirect (§4.5) | `guides/axum-integration.md`, near the middleware section; link it from the Workers guide | Without JavaScript, a refused submission is redirected to the `Referer` with the error in a base64 `__err` query parameter; the page reads it to show the message.  Middleware, caching or analytics that match on URLs should allow that parameter.  **Verify the parameter name** against `leptos_axum`'s behaviour (the server suite asserts `__err`), and cite the test |
| 2c | `use_nonce` only on server builds (§4.6) | `security/challenge.md`, the nonce paragraph in "Content Security Policy" | `leptos::nonce::use_nonce()` exists only with Leptos's `nonce` feature, which server builds have (through `leptos_axum`) and `hydrate` builds do not.  Show the call gated with `#[cfg(feature = "ssr")]`, as a `rust,ignore` snippet.  Say the widget's nonce matters only on the server render |
| 2d | Bundle size near the Free plan limit (§5) | `guides/cloudflare-workers.md`, "Dependencies" | One sentence: a Worker with all five features and a challenge widget measured about 3 MiB gzipped in the reflerd.com team's test build, close to the Free plan's limit.  **Check the current limit** on Cloudflare's Workers limits page and cite it.  Build with `--release` and the usual size settings.  Do not invent per-feature figures |

## Acceptance

1. **The nonce fix.**  The new tests pass; the break fails both; the
   CHANGELOG *Fixed* line is present.
2. **Nonce grep.**  `git grep -n 'A-Za-z0-9+/=' -- crates docs` prints
   nothing: no stale description of the old alphabet remains.
3. **Docs 2a–2d.**
   - **Sources:** each external claim cites its source, with the date
     checked.
   - **The `__err` name** is confirmed by an existing server test; name it.
   - **Book:** `mdbook build docs` passes with no warnings.
4. **Gates, both wasm32 checks, and both suites** pass: native, the Workers
   check, the worker suite, the browser suite, and the examples with
   `--locked`.
5. **Traceability.**  The FR-ABUSE-10 row cites the two new tests.

## Review request

`.git-exclude/review-request/011-cloudflare-workers/05-runtime-report-fixes.md`
