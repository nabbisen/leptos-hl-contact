# RFC 005 — Challenge providers: Turnstile, hCaptcha, reCAPTCHA

**Status.** Implemented (0.5.0) — released 2026-09-13, tag `0.5.0` on commit `6b090eb`, published to crates.io.  Accepted and implemented 2026-09-12/13.
(`min_score` default 0.5; live vendor tests manual only, CI scheduling
deferred to a future RFC).  Amended at acceptance: see §Amendments.
**Handoffs.** [`../handoffs/005-challenge-providers/README.md`](../handoffs/005-challenge-providers/README.md)
**Tracks.** Roadmap M3 item P-21.  Requirements FR-ABUSE-10, FR-ABUSE-11,
FR-ABUSE-12, NFR-PRIV-02.  External Design §5.3 layer 8.
**Touches.** New `challenge.rs` (+ `challenge/http.rs`), `components.rs`,
`server.rs`, `config.rs`, `error.rs`, features, examples, docs.

## Summary

An opt-in `challenge` prop renders a CAPTCHA widget inside the form; the
server function verifies the vendor token with the vendor before
delivery.  One trait, one config, four built-in providers, fail-closed
everywhere, no third-party script unless the integrator asks for it.

## Motivation

Honeypot, rate limit, and the form token stop unsophisticated bots.
High-value or high-traffic forms need a challenge.  Today integrators are
on their own, and the component's lack of a widget slot makes even a
manual integration awkward.

## Goals

- Turnstile, hCaptcha, reCAPTCHA v2 (checkbox and invisible) and v3
  supported by configuration only.
- Verification inside `submit_contact`, fail-closed, time-bounded.
- No JavaScript glue written by the integrator; no inline script except
  for reCAPTCHA v3, which cannot work without one.
- Rejected no-JS submissions get an explanation; the opt-in relaxation is
  explicit and its weakness documented.
- Testable with vendor test keys and with a mock verifier.

## Non-goals

- reCAPTCHA Enterprise; self-hosted or proof-of-work challenges (future
  adapters through the same trait).
- Passing the client IP to vendors (MAY, later; needs a client-IP
  context from the integration).
- Any change to the honeypot or the form token.

## Design

### D1 — Client-visible widget configuration (component prop)

```rust
pub enum ChallengeProvider { Turnstile, HCaptcha, RecaptchaV2, RecaptchaV3 { action: String } }
pub enum ChallengeTheme { Auto, Light, Dark }
pub enum NoJsPolicy { Reject, AcceptWithHoneypotOnly }   // default Reject

pub struct ChallengeWidget {
    pub provider: ChallengeProvider,
    pub site_key: String,             // public
    pub theme: ChallengeTheme,        // Auto
    pub language: Option<String>,     // BCP 47; None = vendor auto
    pub load_script: bool,            // true: the component emits the vendor <script>
    pub script_nonce: Option<String>, // CSP nonce for the script tags
    pub no_js: NoJsPolicy,
}
```

`ContactForm` gains `#[prop(optional)] challenge: Option<ChallengeWidget>`.
Rendering, inside the `<ActionForm>` after the message field:

| Provider | Markup | Token field |
|----------|--------|-------------|
| Turnstile | `<div class="cf-turnstile" data-sitekey data-theme data-language>` | vendor-injected `cf-turnstile-response` |
| hCaptcha | `<div class="h-captcha" data-sitekey data-theme data-hl>` | vendor-injected `h-captcha-response` |
| reCAPTCHA v2 | `<div class="g-recaptcha" data-sitekey data-theme>` | vendor-injected `g-recaptcha-response` |
| reCAPTCHA v3 | hidden `<input name="g-recaptcha-response">` plus an inline script that calls `grecaptcha.execute(site_key, {action})` on load and again on `submit`, writing the token into that input | same name |

Script tags (`load_script = true`): Turnstile
`https://challenges.cloudflare.com/turnstile/v0/api.js`, hCaptcha
`https://js.hcaptcha.com/1/api.js?hl=`, reCAPTCHA
`https://www.google.com/recaptcha/api.js?hl=` (v3: `?render=<site_key>`),
all `async defer`, with `nonce` when given.  Integrators who load scripts
themselves set `load_script = false`.

No-JS: when `no_js == Reject`, the component renders
`<noscript><p class=error role="alert">{labels.errors.challenge_requires_js}</p></noscript>`
next to the widget.

### D2 — Server function arguments

`#[server]` forwards `#[server(rename = "…", default)]` to serde (verified
in `server_fn_macro` 0.8.10).  `submit_contact` gains three optional
arguments, always present in the signature:

```rust
#[server(rename = "cf-turnstile-response", default)] cf_turnstile_response: Option<String>,
#[server(rename = "h-captcha-response", default)]    h_captcha_response:    Option<String>,
#[server(rename = "g-recaptcha-response", default)]  g_recaptcha_response:  Option<String>,
```

The server takes the first `Some` non-empty value as the challenge token.
Unknown fields were already ignored by the deserialiser, so forms without
a widget are unaffected.

### D3 — Server-side verification

```rust
pub struct ChallengeOutcome { pub passed: bool, pub score: Option<f32>, pub action: Option<String>, pub error_codes: Vec<String> }
pub enum ChallengeError { Timeout, Unavailable(String), Misconfigured(String) }

pub trait ChallengeVerifier: Send + Sync + 'static {
    fn verify(&self, token: &str) -> Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + '_>>;
}

pub struct ChallengePolicy { pub no_js: NoJsPolicy, pub min_score: Option<f32> /* v3, default 0.5 */, pub expected_action: Option<String> }
pub struct ChallengeContext { pub verifier: Arc<dyn ChallengeVerifier>, pub policy: ChallengePolicy }
```

Provided through Leptos context in the **server-function handler**.
Processing order in `submit_contact`: form token → normalise → honeypot →
field validation → server policy → **challenge** → filter (RFC 006) →
deliver.  The challenge runs after all local checks so garbage input
never costs a vendor call.

Decision table:

| Context | Token in request | Result |
|---------|------------------|--------|
| absent | absent | proceed (feature not in use) |
| absent | present | **reject** `not_configured`, `error!` log: widget rendered but no verifier (misconfiguration made loud) |
| present | absent, policy `Reject` | reject `challenge_required` |
| present | absent, policy `AcceptWithHoneypotOnly` | proceed, `info!` log "challenge skipped (no token)" |
| present | present, verifier `passed` and score/action satisfy policy | proceed |
| present | present, verifier `!passed` or score/action fail | reject `challenge_failed`, `warn!` with error codes |
| present | present, verifier `Err` | reject `challenge_unavailable`, `error!` with the error; fail-closed (FR-ABUSE-12) |

New `ContactErrorCode`s: `ChallengeRequired`, `ChallengeFailed`,
`ChallengeUnavailable`; labels `challenge_required` ("Please complete the
security check."), `challenge_failed` ("The security check did not pass.
Please try again."), `challenge_unavailable` ("The security check is
unavailable right now. Please try again later."),
`challenge_requires_js` ("This form needs JavaScript to verify you are
human.").

On the weakness of `AcceptWithHoneypotOnly`: a server cannot distinguish
a no-JS browser from a bot that omits the token, so this policy makes the
challenge advisory.  The documentation says so in one plain sentence and
recommends `Reject` unless no-JS visitors matter more than bots.

### D4 — Built-in verifiers (`challenge-http`)

```rust
pub struct HttpChallengeVerifier { provider, secret: SecretString /* redacted Debug */, timeout: Duration /* 5 s */, verify_url: Option<Url> /* override for tests */ }
```

POST `application/x-www-form-urlencoded` `secret`, `response` to:
Turnstile `https://challenges.cloudflare.com/turnstile/v0/siteverify`,
hCaptcha `https://api.hcaptcha.com/siteverify`, reCAPTCHA
`https://www.google.com/recaptcha/api/siteverify`.  Parse `success`,
`score`, `action`, `error-codes`.  HTTP client: `reqwest` with
`default-features = false, features = ["rustls-tls", "http2"]` to avoid a
second TLS stack on musl; `serde_json` already present.  Feature
`challenge = ["ssr"]`; `challenge-http = ["challenge", "dep:reqwest"]`.

### D5 — Testing

- Unit: decision table with a mock verifier (every row); response parsing
  for the three vendors' JSON shapes including `error-codes`; timeout
  path via `verify_url` pointing at a local listener that never answers
  (`tokio::net::TcpListener` in the test).
- Live: `#[ignore]` tests against the real endpoints using the vendors'
  published test keys (Turnstile `1x…AA` pass / `2x…AB` fail, hCaptcha
  `10000000-ffff-ffff-ffff-000000000001`, reCAPTCHA
  `6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI`), run in CI weekly via a
  scheduled job with `--ignored`, not on every push.
- SSR render tests: markup per provider; `<noscript>` present only under
  `Reject`.
- Browser evidence on the hydrated example with Turnstile test keys.

### D6 — Documentation

`security/turnstile.md` becomes `security/challenge.md` covering the four
providers, the decision table, no-JS policy and its weakness, CSP notes
(script sources and frame sources per vendor), privacy disclosure per
vendor, test keys.  Redirect entry in `book.toml`.  Production Checklist
gains the optional row.  Examples: `axum-with-security` gains Turnstile
with test keys behind `CHALLENGE_PROVIDER` / `CHALLENGE_SITE_KEY` /
`CHALLENGE_SECRET` env vars, off when unset.

## Amendment 2026-09-13 — hydration and client-side navigation

Written before RFC 002, 004 and 007 established how the form behaves under
hydration and client-side navigation.  Three consequences for D1, carried
into handoff 02:

- The `<noscript>` message is rendered with `inner_html`: with scripting on,
  the HTML parser keeps `<noscript>` content as text, so hydrating child
  views there would fail.
- Vendor scripts scan for their widget class once, on load; a form reached by
  client-side navigation may need explicit rendering.  Evidence decides.
- A vendor script is inserted only when not already present, so returning to
  the form does not load it twice.

## Amendment 2026-09-13 (2) — as implemented in handoffs 01 and 02

- **D2.**  `server_fn_macro` 0.8.10 parses one meta item per `#[server(…)]`
  attribute, so each challenge argument carries `#[server(rename = "…")]`
  and `#[server(default)]` as two attributes.
- **D3.**  `ChallengePolicy::min_score` is `f32` with a `Default` of `0.5`,
  not `Option<f32>`; a NaN score fails.
- **D1.**  `ChallengeWidget` fields are crate-private and the type is not
  `Deserialize`, so every value reaching markup or the v3 inline script has
  passed `new` and the builders.  In the browser a widget created after the
  vendor script loaded is rendered explicitly (all three vendors skip late
  elements, measured); Turnstile and hCaptcha render directly, reCAPTCHA via
  `ready`.  A browser-inserted vendor script goes into `<head>`, carries the
  nonce, and is inserted only if absent; the server render keeps it in the
  form so hydration matches.

## Amendment 2026-09-13 (3) — optional prop and no redirects

- **D1.**  `ContactForm`'s `challenge` prop is `Option<ChallengeWidget>`
  with `#[prop(optional_no_strip)]`, so a widget that exists only when keys
  are configured is one form, not two.
- **D4.**  `HttpChallengeVerifier` builds its client with redirects disabled.
  The request body carries the vendor secret and `reqwest` re-attaches a body
  when it follows a redirect; no siteverify endpoint redirects in normal
  operation, so a 3xx is `Unavailable`.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Our own hidden field filled by vendor callbacks | Needs inline JavaScript for two of three vendors; renamed fields solve it without any |
| Middleware verification (buffer the body) | Framework-specific, duplicates form parsing, cannot report a field-level code |
| Verify before validation | Wastes vendor calls on invalid input |
| Fail-open on vendor outage | Violates FR-ABUSE-12; a vendor outage is rare and short, spam is constant |
| `reqwest` with native-tls | Would add a second TLS backend requirement on musl; rustls has none |

## Compatibility

Minor.  Additive: new prop, arguments, types, features, codes, labels.
DOM contract: new elements only when the prop is set.

## Security considerations

- Secret only in `HttpChallengeVerifier`, redacted.
- Fail-closed in every error case; the misconfiguration row rejects.
- Vendor scripts widen the page's script surface; CSP guidance included.
- Threat model: T4 gains the challenge control; new row T16 "vendor
  outage causes rejection" (accepted: availability trade for safety).
- Privacy: per-vendor disclosure text; the default form loads nothing.

## Operational considerations

One outbound HTTPS call per submission with a 5-second cap; the
integrator's rate limit still applies first.  Vendor dashboards show
verification volume.

## Acceptance criteria

FR-ABUSE-10, 11, 12 Met; NFR-PRIV-02 Met.  Demonstrated on the hydrated
example with Turnstile test keys (pass and fail keys), and the decision
table fully covered by unit tests.

## Implementation boundaries

Three handoffs: (1) core trait, config, codes, decision table with mock
verifier, server arguments; (2) component rendering for four providers
and no-JS; (3) `challenge-http` verifiers, live tests, example, docs.

## Owner decisions

1. Default `min_score` for reCAPTCHA v3 is **0.5**.
2. Live tests against vendor endpoints are **manual only** (`#[ignore]`,
   run by hand).  A scheduled CI job is a future RFC because it has a
   cost for the owner (roadmap P-26).

## Amendments at acceptance

- **No `challenge` feature flag.**  The trait, config, codes, and decision
  logic have no dependencies and are compiled whenever `ssr` is; the
  client-visible widget types live in `config` and are always compiled.
  Only the built-in HTTP verifiers sit behind `challenge-http`.  One flag
  for one dependency; nothing for developers to pair up.
- **`ChallengeWidget::new` validates `site_key` and `action`** to
  `[A-Za-z0-9_-]` so the values can be embedded in markup and in the v3
  inline script without escaping concerns.
- **reCAPTCHA v3 tokens are single-use**, so the inline script fetches a
  token on every submit rather than at load; the load-time call in D1 is
  dropped.
- **D5 testing**: the "weekly scheduled" sentence is replaced by "manual,
  `cargo test -- --ignored` with the vendor test keys".

## Release implications

Proposed `0.5.0` together with RFC 004, or `0.5.1` if it lags.
