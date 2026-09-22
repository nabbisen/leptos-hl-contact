# Challenge (CAPTCHA)

A challenge asks the visitor's browser to prove it is operated by a person.
Add one when the honeypot, the rate limit and the form token are not enough
for a high-value or high-traffic form.

The component renders the widget from configuration alone, and
`submit_contact` verifies the token with the vendor before anything is
delivered.  Every error fails closed.  A form without the `challenge` prop
loads nothing from any vendor.

## Providers

| Provider | `ChallengeProvider` | Widget | Token field | Verification endpoint |
|----------|---------------------|--------|-------------|-----------------------|
| Cloudflare Turnstile | `Turnstile` | visible or managed | `cf-turnstile-response` | `https://challenges.cloudflare.com/turnstile/v0/siteverify` |
| hCaptcha | `HCaptcha` | checkbox | `h-captcha-response` | `https://api.hcaptcha.com/siteverify` |
| Google reCAPTCHA v2 | `RecaptchaV2` | checkbox or invisible | `g-recaptcha-response` | `https://www.google.com/recaptcha/api/siteverify` |
| Google reCAPTCHA v3 | `RecaptchaV3 { action }` | none; a score | `g-recaptcha-response` | `https://www.google.com/recaptcha/api/siteverify` |

## Setup

Two halves, and both are required: the widget in the page, and the verifier
on the server.

**1. Enable the built-in verifier** on the server binary:

```toml
leptos-hl-contact = { version = "0.9", features = ["ssr", "axum-helpers", "form-token", "challenge-http"] }
```

**2. Render the widget.**  The site key is public:

```rust,ignore
use leptos_hl_contact::config::{ChallengeProvider, ChallengeWidget};

let challenge = ChallengeWidget::new(ChallengeProvider::Turnstile, site_key)
    .expect("site key uses only [A-Za-z0-9_-]");

view! { <ContactForm challenge=Some(challenge) /> }
```

`challenge` takes an `Option`, so a widget that exists only when keys are
configured needs no second form: pass `None` and the form renders exactly
as it does without the prop.

`ChallengeWidget::new` and its `with_*` methods validate every value that
reaches the page: the site key and the v3 action against `[A-Za-z0-9_-]+`,
the language as a BCP 47 tag, the nonce as base64.

**3. Verify on the server.**  The secret never leaves it:

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::{ChallengeContext, ChallengePolicy, HttpChallengeVerifier};

let challenge = ChallengeContext {
    verifier: Arc::new(HttpChallengeVerifier::new(ChallengeProvider::Turnstile, secret)),
    policy: ChallengePolicy::default(),
};

// In the context closure passed to `leptos_routes_with_context`:
provide_context(challenge.clone());
```

For reCAPTCHA v3, set `ChallengePolicy::expected_action` to the widget's
action and adjust `min_score` (default 0.5) to your traffic.

`HttpChallengeVerifier` makes one call per submission, capped at five
seconds (`with_timeout`), and never retries.  If outbound traffic must go
through a forwarding proxy, point `with_verify_url` at it.

A complete, environment-driven setup is in
[`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security):
`CHALLENGE_PROVIDER`, `CHALLENGE_SITE_KEY`, `CHALLENGE_SECRET`.

## Size

`ChallengeWidget::with_size(ChallengeSize)` asks the vendor for a smaller
widget, for a narrow layout.  The default, `Normal`, renders no attribute at
all — every existing page keeps rendering exactly as it did before this
option existed.

| Provider | `data-size` / `size` values | Dimensions | Source |
|----------|------------------------------|------------|--------|
| Turnstile | `normal` (default), `flexible`, `compact` | 300×65 px; 100 % width, minimum 300 px, ×65 px; 150×140 px | Cloudflare, [Widget configurations](https://developers.cloudflare.com/turnstile/get-started/client-side-rendering/widget-configurations/), checked 2026-09-17 |
| hCaptcha | `normal` (default), `compact` | vendor-controlled | hCaptcha, [Configuration](https://docs.hcaptcha.com/configuration), checked 2026-09-17 |
| reCAPTCHA v2 | `normal` (default), `compact` | vendor-controlled | Google, [Display](https://developers.google.com/recaptcha/docs/display), checked 2026-09-17 |
| reCAPTCHA v3 | — (no visible widget) | — | — |

**For narrow layouts,** `Compact` is the choice: it is the only size that
helps below about 340 px of available width, and every visible provider
offers it.

**`Flexible` is Turnstile only,** and has a 300 px minimum.  It does not
help a layout narrower than 300 px plus padding — only `Compact` does.  On
another provider, `Flexible` renders that provider's default size.

## What the server decides

The challenge runs after the form token, the honeypot, field validation and
the server policy, so invalid input never costs a vendor call.

- **No `ChallengeContext` and no token:** nothing to do; the form proceeds.
- **A token but no `ChallengeContext`:** rejected as `not_configured`, with
  an `error` log.  A widget was rendered and nothing can verify it; that
  misconfiguration must be loud.
- **A `ChallengeContext` but no token:** rejected as `challenge_required`
  under `NoJsPolicy::Reject`, or accepted with an `info` log under
  `AcceptWithHoneypotOnly`.
- **The vendor passed the token,** and the score and action satisfy the
  policy: proceed.
- **The vendor failed the token,** the score is below `min_score` (or not a
  number), or the action does not match: rejected as `challenge_failed`,
  with a `warn` log carrying the vendor's error codes.
- **The vendor could not be asked** — a timeout, a network error, a non-2xx
  status, or a response that is not the expected JSON: rejected as
  `challenge_unavailable`, with an `error` log.  This fails closed: a vendor
  outage rejects submissions rather than letting spam through.

An empty or whitespace-only token counts as absent.  Tokens and secrets are
never logged.

**A missing secret.**  Whether a challenge is on is decided by whether the
widget renders, not by whether you have its secret.  Leaving out
`ChallengeContext` when the secret is missing looks safe but is not: it
refuses only submissions that carry a token, and a bot that posts without one
meets no challenge.  If the widget renders, provide the context, and pass a
missing secret to `HttpChallengeVerifier::new` as an empty string.  Then,
under `NoJsPolicy::Reject` (the default):

- **with a token:** refused as `challenge_unavailable`, with an `error` log;
- **without a token:** refused as `challenge_required`;
- **either way,** nothing is delivered.

```rust,ignore
// An absent secret becomes "", so every submission is refused.
let secret = std::env::var("CHALLENGE_SECRET").unwrap_or_default();
let context = ChallengeContext {
    verifier: Arc::new(HttpChallengeVerifier::new(ChallengeProvider::Turnstile, secret)),
    policy: ChallengePolicy::default(),
};
```

Where you can, refusing to start when a secret is missing is better still, as
[Hardening](./hardening.md#secrets) advises.  A Worker has no start-up to
refuse, so there the empty secret is the pattern.

## Without JavaScript

Every vendor needs JavaScript to produce a token.  `NoJsPolicy` decides what
happens to a submission that arrives without one:

- **`Reject`** (default): the component shows `challenge_requires_js` inside
  `<noscript>`, and the server answers `challenge_required`.
- **`AcceptWithHoneypotOnly`**: the submission is accepted on the honeypot
  alone.

A server cannot distinguish a no-JS browser from a bot that omits the token,
so `AcceptWithHoneypotOnly` makes the challenge advisory.  Use `Reject`
unless no-JS visitors matter more than bots.

Set the same policy on both sides: `ChallengeWidget::with_no_js` and
`ChallengePolicy::no_js`.

## What the visitor sees

| Situation | Label | Default text |
|-----------|-------|--------------|
| No token (policy `Reject`) | `challenge_required` | Please complete the security check. |
| The check failed | `challenge_failed` | The security check did not pass. Please try again. |
| The vendor could not be reached | `challenge_unavailable` | The security check is unavailable right now. Please try again later. |
| JavaScript is off (policy `Reject`) | `challenge_requires_js` | This form needs JavaScript to verify you are human. |
| Widget rendered, no verifier on the server | `not_configured` | This form is not available right now. |

All of them are in `ContactErrorLabels`; see
[Localization](../guides/localization.md#error-messages).

## Content Security Policy

One row per provider, from each vendor's own documentation (checked
2026-09-15).  Where a vendor does not state something, the table says so
rather than guessing.

| Provider | Directives | Nonce and `'strict-dynamic'` | Source |
|----------|------------|------------------------------|--------|
| Cloudflare Turnstile | `script-src https://challenges.cloudflare.com`; `frame-src https://challenges.cloudflare.com`; `connect-src 'self'` only in pre-clearance mode | a nonce on the `api.js` script propagates to what it loads; works with `'strict-dynamic'` (supported, not required) | [Turnstile CSP reference](https://developers.cloudflare.com/turnstile/reference/content-security-policy/) |
| hCaptcha | `script-src`, `frame-src`, `style-src` and `connect-src`, each `https://hcaptcha.com https://*.hcaptcha.com`.  Do not pin specific subdomains: asset hosts vary by time and region | not documented by the vendor | [hCaptcha: Content Security Policy Settings](https://docs.hcaptcha.com/#content-security-policy-settings) |
| Google reCAPTCHA (v2 and v3) | `script-src https://www.google.com/recaptcha/ https://www.gstatic.com/recaptcha/`; `frame-src https://www.google.com/recaptcha/ https://recaptcha.google.com/recaptcha/`; `connect-src https://www.google.com/recaptcha/` | a nonce on the `api.js` script, and the vendor handles the rest; works with `'strict-dynamic'` in browsers that support it | [reCAPTCHA FAQ](https://developers.google.com/recaptcha/docs/faq) |

hCaptcha also lists `'unsafe-eval'` and `'unsafe-inline'` as optional
additions for its enterprise features; the widget this crate renders does
not need them.

**The nonce.**  Every script tag the component writes carries the nonce from
`ChallengeWidget::with_script_nonce`:
- the vendor script;
- reCAPTCHA v3's inline submit script;
- the vendor script the browser adds after client-side navigation.

reCAPTCHA v3's inline script needs the nonce, or `'unsafe-inline'`, which is
not recommended.

**Leptos's own nonce.**  With a nonce-based policy, pass the nonce Leptos
puts on its own scripts:

```rust,ignore
let widget = ChallengeWidget::new(ChallengeProvider::Turnstile, site_key)?;
#[cfg(feature = "ssr")]
let widget = match leptos::nonce::use_nonce() {
    Some(nonce) => widget
        .with_script_nonce(nonce.to_string())
        .expect("a Leptos nonce is a valid CSP nonce"),
    None => widget,
};
```

- **Server builds only.**  `leptos::nonce::use_nonce()` exists only with
  Leptos's `nonce` feature.  Server builds have it, because `leptos_axum`
  enables it; `hydrate` builds do not.  Gate the call with
  `#[cfg(feature = "ssr")]`, as above.
- **Where the nonce matters.**  On the server render, which writes the
  script tags into the HTML the policy's nonce applies to.
- **After client-side navigation,** the hydrated form inserts the vendor
  script itself, with the widget's own nonce (`components.rs`, the
  `append_script` call).  A browser (`hydrate`) build has no Leptos nonce to
  give it, so that script carries none.  It still loads when the policy
  allows the vendor host by name, or uses `'strict-dynamic'`, which trusts
  scripts that trusted scripts insert.  The reflerd.com team confirmed the
  host allow-list case after navigation.

**The honeypot.**  The honeypot is hidden by an inline `style` attribute,
which a policy without `'unsafe-inline'` blocks.  Under such a policy:
- set `ContactFormOptions::honeypot_inline_style` to `false`;
- hide the wrapper through `ContactFormClasses::honeypot` with your own CSS.

See [Styling: Honeypot](../guides/styling.md#honeypot).

## The visitor's IP

All three vendors accept the visitor's IP as `remoteip` with the token, and
hCaptcha recommends sending it.  Provide it per request as
`ChallengeClientIp` in the context closure: `HttpChallengeVerifier` then
sends it, and a verifier of your own receives it in `verify_request`.

**The crate never reads a header for it.**  Which header can be trusted
depends on the proxy in front of your site, and a guess would accept a
spoofed `X-Forwarded-For`.  Behind Cloudflare, use `CF-Connecting-IP`; with no
proxy, the peer address.

```rust,ignore
use axum::http::request::Parts;
use leptos::prelude::*;
use leptos_hl_contact::ChallengeClientIp;

let context = move || {
    // … the other context values …
    let ip = use_context::<Parts>().and_then(|parts| {
        parts.headers.get("CF-Connecting-IP")?.to_str().ok()?.parse().ok()
    });
    if let Some(ip) = ip {
        provide_context(ChallengeClientIp(ip));
    }
};
```

The IP is personal data: say so in your privacy notice.  The crate never
logs it.

## Privacy

A challenge sends data about the visitor to a third party.  Say so in your
privacy notice, and name the vendor.

- **Cloudflare Turnstile.**  The widget runs in the visitor's browser and
  sends signals about the browser and its environment, and the visitor's IP
  address, to Cloudflare.  See Cloudflare's
  [privacy policy](https://www.cloudflare.com/privacypolicy/).  The server
  also sends the visitor's IP to Cloudflare when the site provides
  `ChallengeClientIp`.
- **hCaptcha.**  The widget sends browser and interaction data, and the
  visitor's IP address, to hCaptcha.  See hCaptcha's
  [privacy policy](https://www.hcaptcha.com/privacy).  The server also
  sends the visitor's IP to hCaptcha when the site provides
  `ChallengeClientIp`.
- **Google reCAPTCHA.**  The script sends hardware, software and interaction
  data, and the visitor's IP address, to Google.  See Google's
  [privacy policy](https://policies.google.com/privacy) and
  [terms](https://policies.google.com/terms).  Google's
  [FAQ](https://developers.google.com/recaptcha/docs/faq) explains the
  branding it requires if you hide the badge.  The server also sends the
  visitor's IP to Google when the site provides `ChallengeClientIp`.

The server additionally sends the token and your secret to the vendor's
verification endpoint, and the visitor's IP when you provide
`ChallengeClientIp` (see [The visitor's IP](#the-visitors-ip)).  It sends no
form content.

## Test keys

Never deploy these: they provide no protection.

| Vendor | Site key | Secret | Result |
|--------|----------|--------|--------|
| Turnstile | `1x00000000000000000000AA` | `1x0000000000000000000000000000000AA` | always passes |
| Turnstile | `2x00000000000000000000AB` | `2x0000000000000000000000000000000AA` | always fails |
| Turnstile | `1x00000000000000000000AA` | `3x0000000000000000000000000000000AA` | fails, token already spent |
| hCaptcha | `10000000-ffff-ffff-ffff-000000000001` | `0x0000000000000000000000000000000000000000` | always passes; response token `10000000-aaaa-bbbb-cccc-000000000001` |
| reCAPTCHA v2 | `6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI` | `6LeIxAcTAAAAAGG-vFI1TnRWxMZNFuojJ4WifJWe` | always passes; the widget shows a warning |

The Turnstile blocking site key `2x00000000000000000000AB` fails **in the
browser**: the widget reports an error and issues no token, so the
submission arrives without one and is rejected as `challenge_required` —
"Please complete the security check." — not `challenge_failed`.  The
secret is never consulted.  To see "The security check did not pass", use
the passing site key `1x00000000000000000000AA` with the failing secret
`2x0000000000000000000000000000000AA`.

Turnstile's and hCaptcha's test keys work on any host, including
`localhost`.  Google publishes no reCAPTCHA v3 test key; the v2 key above
issues tokens under `?render=` but its score means nothing.  Sources:
[Turnstile](https://developers.cloudflare.com/turnstile/troubleshooting/testing/),
[hCaptcha](https://docs.hcaptcha.com/),
[reCAPTCHA](https://developers.google.com/recaptcha/docs/faq).

Live tests against these endpoints are `#[ignore]`d; see
[Testing](../development/testing.md#live-challenge-tests).

## Client-side navigation

Every vendor scans the page for its widget once, when its script loads.  A
form reached by client-side navigation adds its widget after that scan, so
the component renders it explicitly, and adds the vendor script to `<head>`
at most once so returning to the form never loads it twice.

Turnstile and hCaptcha are rendered by calling `render` directly: Turnstile
does not run a `ready` callback registered after its script has loaded, and
hCaptcha has no `ready`.  reCAPTCHA is rendered through `ready`, which runs
at once when the script has already loaded.  Do not change the first to use
`ready`: the widget would silently never render.

reCAPTCHA v3's inline script uses `form.requestSubmit()`, which very old
browsers lack.
