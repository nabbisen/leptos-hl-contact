# Handoff 03 — HTTP verifiers, example, documentation

**RFC.** [005](../../accepted/005-challenge-providers.md), D4, D5, D6.
**Requirements.** FR-ABUSE-12, NFR-PRIV-02, NFR-DOC-01.
**Depends on.** Handoffs 01 and 02.

## Change scope

`Cargo.toml` (feature `challenge-http`, `reqwest`), new
`src/challenge/http.rs` + tests, `examples/axum-with-security`, docs,
CI (`--all-features` already covers the new feature).

## Required implementation

1. **Feature.**  `challenge-http = ["ssr", "dep:reqwest"]`;
   `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "http2"], optional = true }`.
   No other new dependency.
2. **`HttpChallengeVerifier`.**

   ```rust
   pub struct HttpChallengeVerifier { provider: ChallengeProvider, secret: String, timeout: Duration, verify_url: Option<String>, client: reqwest::Client }
   impl HttpChallengeVerifier {
       pub fn new(provider: ChallengeProvider, secret: impl Into<String>) -> Self;   // timeout 5 s
       pub fn with_timeout(self, Duration) -> Self;
       pub fn with_verify_url(self, impl Into<String>) -> Self;                       // tests / proxies
   }
   impl std::fmt::Debug for … { /* secret redacted */ }
   impl ChallengeVerifier for HttpChallengeVerifier { … }
   ```

   Endpoints per RFC D4.  POST form fields `secret`, `response`.  Map:
   `reqwest` timeout → `Timeout`; other transport or non-2xx →
   `Unavailable(status or error)`; JSON without `success` →
   `Unavailable("malformed response")`; empty secret → `Misconfigured`.
   Parse `success`, `score`, `action`, `error-codes`.
3. **Tests (`challenge/http/tests.rs`).**  A minimal local HTTP responder
   built on `tokio::net::TcpListener` inside the test (read the request,
   write a canned HTTP/1.1 response), used via `with_verify_url`:
   success JSON for each vendor shape; `success:false` with error codes;
   500 → `Unavailable`; a listener that accepts and never writes, with
   `with_timeout(200 ms)` → `Timeout`.  Live tests `#[ignore]` per vendor
   with the published test keys (pass and fail keys where the vendor has
   both); document the command in `testing.md`.
4. **Example.**  `axum-with-security` reads `CHALLENGE_PROVIDER`
   (`turnstile|hcaptcha|recaptcha-v2|recaptcha-v3`), `CHALLENGE_SITE_KEY`,
   `CHALLENGE_SECRET`; when all are set it provides `ChallengeContext`
   and passes `challenge=` to the form; otherwise nothing.  A comment
   lists the Turnstile test keys.
5. **Docs.**
   - `git mv docs/src/security/turnstile.md docs/src/security/challenge.md`
     and rewrite: providers table, setup (prop + context), the decision
     table in words, no-JS policy with the weakness sentence, CSP snippet
     per vendor (`script-src`, `frame-src`, `connect-src`), privacy
     disclosure paragraph per vendor, test keys, "what the visitor sees"
     mapped to labels.  `book.toml` redirect `/security/turnstile.html` →
     `challenge.html`.  `SUMMARY.md`.
   - `security/README.md`: replace the two "challenge" mentions with links
     to the new page; the layering list stays.
   - `getting-started/production-checklist.md`: optional row updated.
   - `guides/localization.md`: the four new labels.
   - `reference/feature-flags.md`: `challenge-http`.
   - `development/testing.md`: live test command.
   - `CHANGELOG.md` Unreleased.

## Two small edits carried from the 005-01 and 005-02 reviews

- `server.rs`, step 6 comment: replace "Leptos context is not reachable
  after an await point here" with the reason that holds — every
  configuration check is made before any network call.  The await claim was
  never demonstrated.
- `security/challenge.md`: one line saying Turnstile and hCaptcha are
  rendered directly because their `ready` does not fire after the script has
  loaded, and reCAPTCHA through `ready`.

## Acceptance criteria

- Tests pass; gates green; `cargo test --features challenge-http -- --ignored`
  passes against Turnstile and hCaptcha test keys (paste the output;
  reCAPTCHA test keys always pass on Google's side, include it too).
- Hydrated example with Turnstile pass key → success; with fail key
  `2x00000000000000000000AB` / secret `2x0000000000000000000000000000000AA`
  → "did not pass" banner; with the secret unset but the prop set →
  `not_configured` and an `error!` line.  Record.

## Prohibited shortcuts

`reqwest` default features; a global client per call; retrying a failed
verification; swallowing malformed JSON as failure instead of unavailable.

## Compatibility and security constraints

Additive.  Secret redacted; 5-second cap; fail-closed on every error.

## Known risks

Corporate proxies: `with_verify_url` exists for that; document it.  The
`http2` feature needs no extra system libraries with rustls.

## Required evidence

Gate outputs; unit and live test outputs; the three recordings.
