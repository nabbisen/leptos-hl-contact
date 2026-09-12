# Form Token (`form-token` feature)

A stateless, HMAC-SHA256-signed token that the page render places in a hidden
`form_token` field and the server function verifies before it looks at
anything else.  No session store, no database.

## What it guarantees

| Property | `Binding::None` (default) | `Binding::Cookie` |
|----------|---------------------------|-------------------|
| The sender obtained a token from this server within `ttl_secs` | yes | yes |
| The token was not forged (HMAC with your secret, constant-time compare) | yes | yes |
| At least `min_age_secs` passed between the page render and the submission | yes | yes |
| The token was issued to the browser that is now submitting | **no** | yes |
| The token cannot be reused | **no** — any token is valid until it expires | **no** |
| Cross-site POSTs are rejected | **no** — use [Origin validation](./hardening.md#origin--referer-validation) | as defence in depth only — see below |

**[Origin validation](./hardening.md#origin--referer-validation) remains the
control that rejects cross-site POSTs, with or without binding.**  With
`Binding::None` the token is an anti-automation measure.  With
`Binding::Cookie` it becomes a second, independent check against cross-site
submissions — defence in depth, not a replacement: keep the Origin check on.
Two controls fail independently, and binding has a limit the Origin check
does not, described under [What binding cannot
stop](#what-binding-cannot-stop).

## Cookie binding

The token's nonce — never the token, never the secret — is also written to a
cookie that is `HttpOnly` and `SameSite=Lax`.  Verification then requires
both halves to agree:

- **`HttpOnly`** means no script can read the value, so a cross-site page
  cannot copy it into a forged form.
- **`SameSite=Lax`** means the browser does not send the cookie on a
  cross-site POST at all, so a forged submission arrives without it and
  fails with `BindingMissing`.
- **The `__Host-` prefix** means a sibling subdomain cannot plant a cookie
  of its own choosing — see below.

With Axum, two helpers do the work inside the one context closure:

```rust,ignore
use leptos_hl_contact::{
    axum_helpers::{FormTokenCookie, provide_form_token_binding, provide_form_token_with_cookie},
    form_token::{Binding, FormTokenConfig, FormTokenContext},
};

let token_config: FormTokenContext = Arc::new(
    FormTokenConfig::new(secret).with_binding(Binding::Cookie),
);
let token_cookie = FormTokenCookie::default();   // __Host-hl_contact_ft, Secure, Path=/

// In the closure passed to `leptos_routes_with_context`:
provide_context::<FormTokenContext>(Arc::clone(&token_config));
provide_form_token_with_cookie(&token_config, &token_cookie);
provide_form_token_binding(&token_cookie);
```

`provide_form_token_with_cookie` issues a token and sets the cookie **only on
`GET`**.  That matters: one closure serves page renders and server functions
alike, so issuing on a POST would overwrite the cookie the submitted form is
bound to and break the next submission.

**The cookie is per browser, not per page.**  When a request already carries
one, that nonce is signed into the new token rather than a fresh one being
minted, so the value survives across pages, across tabs and across a visit to
any other route.  Several open copies of the form therefore all submit
successfully, and so does a form left open while the visitor browsed
elsewhere.  Only the token rotates per render — it carries its own timestamp,
and the TTL and minimum age count from that render.

A cookie that arrives truncated, over-long or not hexadecimal is never signed
into a token; the helper mints a new nonce instead.

`FormTokenCookie::secure` defaults to `true`.  A `Secure` cookie is never
sent back over plain HTTP, so set it to `false` — and only — when developing
against `http://localhost`.

### The `__Host-` prefix

At the defaults the cookie is sent as `__Host-hl_contact_ft`.  Browsers
refuse to store a `__Host-` cookie that carries a `Domain` attribute, and
that refusal is what stops **cookie tossing**: without it, anyone who
controls `anything.example.com` could set `hl_contact_ft` with
`Domain=.example.com; SameSite=None; Secure`, pair it with a token they
fetched in their own browser for the same nonce, and have the victim's
browser submit both.  The binding check would pass.

The prefix is applied only when the cookie's own attributes allow it —
`secure` is `true` and `path` is `/` — because a browser silently drops a
`__Host-` cookie that lacks either.  So:

- Production defaults: `__Host-hl_contact_ft`, protected.
- `secure: false` for local HTTP: plain `hl_contact_ft`, which is fine on
  `localhost` and must never reach production.
- A `path` other than `/`: plain name, and **no protection against a
  subdomain**.  Keep the default unless you have a reason.

Whatever you set, `name` is given without the prefix; it is added for you.

### What binding cannot stop

Binding rests on an attacker being unable to put a chosen value in the
victim's cookie.  The prefix closes the subdomain route at the defaults,
but not every route: a response-header injection on your own origin, or a
deployment where the prefix does not apply, reopens it.  That is why the
Origin check stays on.  Binding also does nothing against a script or bot
that simply loads the page and submits from its own browser — that is the
minimum age's and the rate limit's job.  And a page served from a cache
pairs one visitor's token with another visitor's cookie, which breaks the
binding outright; see [Do not cache the contact page](#cookie-binding)
above.

Any other framework can use binding by providing
[`FormTokenBinding`](https://docs.rs/leptos-hl-contact/latest/leptos_hl_contact/form_token/struct.FormTokenBinding.html)
itself; the core never parses cookies.

> **Do not cache the contact page.**  Each render sets a fresh cookie paired
> with a fresh token.  A reverse proxy or CDN that caches the page, or strips
> `Set-Cookie` from a cached response, serves one visitor's token to another
> and every submission fails.  Send `Cache-Control: no-store` for that route.

## Tokens in the browser: acquisition and refresh

A server render puts a token in the hidden field.  Two situations leave a
hydrated form without a usable one: the visitor reached the form by
client-side navigation, so no server rendered it; or the page stayed open
long enough for the token to expire.

**Handling either takes two switches.**  The browser cannot learn how your
server is configured — a server without form tokens and a client-side
navigation both leave the field empty — so it calls the token endpoint only
when the component is told to:

1. the server issues form tokens (the `form-token` feature, with
   `FormTokenContext` in the context closure — see [Setup](#setup)); **and**
2. the component sets `ContactFormOptions::token_refresh_secs` to
   `ttl_secs - 60`:

   ```rust,ignore
   <ContactForm options=ContactFormOptions {
       token_refresh_secs: Some(3540), // default TTL 3600, minus 60
       ..Default::default()
   } />
   ```

If you do only the first, server-rendered forms work exactly as before, but
**a form reached by client-side navigation submits an empty token and the
visitor sees the token-invalid message**.  With the option at its default,
`None`, the browser never calls `/api/form_token`, which is correct for a
server without form tokens, where that route does not exist.

**Client-side navigation.**  With the option set, after mount the component
reads the field's value from the DOM; if it is empty, it calls the
`issue_form_token_fn` server function (`POST /api/form_token`) and puts the
result in the field.  A form that *was* server-rendered already has a value
and makes no request.  The check is on the DOM, not on whether the page is
hydrating, so it is right in both cases.

**Refresh.**  The token the form mounted with is refreshed
`token_refresh_secs` after it was issued — or once, immediately, if that
moment has already passed, as it has for a page restored from the
back/forward cache.  Every token fetched after that is refreshed
`token_refresh_secs` after it *arrived*, timed on the browser's clock alone.
A browser clock that is wrong therefore costs at most one extra request per
page, never a loop.  Values of 60 or less fetch a missing token but
never refresh.

A fetch that fails — a `429` from your rate limiter, say — is not retried.
If no later refresh succeeds, the token eventually expires and the visitor
sees the token-invalid message, as they would have without the refresh.

With binding on, a fetched token reuses the browser's existing nonce, exactly
as a page render does, so fetching one never invalidates a form open in
another tab.  Provide the issuer in the context closure so the cookie is
re-sent for it:

```rust,ignore
provide_form_token_with_cookie(&token_config, &token_cookie);
provide_form_token_binding(&token_cookie);
provide_form_token_issuer(&token_cookie);   // for tokens fetched by the browser
```

**Rate-limit the endpoint.**  `/api/form_token` is public.  It grants nothing
a page render does not, but it is cheaper to call than a page, so make sure
your rate limit covers it like every other route.

Without JavaScript none of this runs, and none of it is needed: the page is
always server-rendered, with a token.

## The minimum age

A person needs a moment to read a form and fill it in.  A script does not.
`min_age_secs` — two seconds by default — rejects a submission that arrives
sooner than that after the page was rendered.

The rejection is **retryable**, and deliberately so: the same token becomes
valid a moment later, so somebody who autofilled and clicked immediately is
delayed by one attempt rather than turned away.  They see
`labels.errors.too_fast`, "Please wait a moment and try again." by default,
and submitting again works.

Set `min_age_secs` to `0` to disable the check.

## How it works

```text
startup        FormTokenConfig::new(secret)        ttl 3600, min age 2, no binding
                 → provided as FormTokenContext in the context closure

each render    issue_form_token(&config) → FormToken
                 → provided via context
                 → ContactForm renders <input type="hidden" name="form_token">

each submit    submit_contact receives form_token
                 → verify_form_token(token, None, &config)
                 → Err(TooYoung)  → contact_error:too_fast     (retryable)
                 → any other Err  → contact_error:token_invalid
```

Token format: `{unix_seconds}|{16-byte nonce hex}|{hmac_sha256 hex}`.  The
timestamp is what makes a token fresh; the nonce is what ties it to a browser,
and with `Binding::Cookie` it is deliberately stable for the cookie's life.
Verification runs in this order: format, timestamp, future skew (60 s),
expiry, minimum age, signature, binding.

## Setup

1. Enable the feature on the server binary:

   ```toml
   leptos-hl-contact = { version = "0.5", features = ["ssr", "smtp-lettre", "axum-helpers", "form-token"] }
   ```

2. Generate a secret and keep it server-side:

   ```bash
   FORM_TOKEN_SECRET=$(openssl rand -hex 32)
   ```

3. Build the config once:

   ```rust,ignore
   use std::sync::Arc;
   use leptos_hl_contact::form_token::{FormTokenConfig, FormTokenContext};

   let token_config: FormTokenContext = Arc::new(
       FormTokenConfig::new(
           std::env::var("FORM_TOKEN_SECRET")
               .expect("FORM_TOKEN_SECRET")
               .into_bytes(),
       ),
   );
   // Defaults: .with_ttl(3600).with_min_age(2)
   ```

4. Provide it in the context closure, with a fresh token per render:

   ```rust,ignore
   // The closure passed to `leptos_routes_with_context`
   provide_context::<FormTokenContext>(Arc::clone(&token_config));
   provide_context(issue_form_token(&token_config));
   ```

   The full router is in [Axum Integration](../guides/axum-integration.md#all-context-values-together).

`ContactForm` and `submit_contact` need no changes.

## Fail-closed behaviour

When the feature is enabled and `FormTokenContext` is missing from the
context closure, every submission is rejected with the `not_configured`
code and an `error` log line.  This is deliberate: forgetting the context
must not silently disable the check.

If `FormToken` is missing, the hidden field is empty and every submission
fails verification.

## Migration from the 0.3 and 0.4 names

The feature, module and items were renamed in 0.5 because the token is not a
CSRF control on its own.  The old names still work, warn, and are removed in
the next minor.

| 0.3 / 0.4 | 0.5 |
|-----------|-----|
| feature `csrf` | `form-token` |
| module `csrf` | `form_token` |
| `CsrfConfig` | `FormTokenConfig` |
| `CsrfToken` | `FormToken` |
| `CsrfConfigContext` | `FormTokenContext` |
| `generate_csrf_token` | `issue_form_token` |
| `verify_csrf_token(token, cfg) -> bool` | `verify_form_token(token, bound, cfg) -> Result<(), FormTokenError>` |
| hidden field `csrf_token` | `form_token` |
| env var `CSRF_SECRET` (docs) | `FORM_TOKEN_SECRET` |

Two things do **not** move by themselves:

- **A `CsrfConfig { secret_key, token_ttl_secs }` struct literal no longer
  compiles.**  `token_ttl_secs` is now `ttl_secs`, and two fields were added.
  Use `FormTokenConfig::new(secret)` with `with_ttl`, `with_min_age` and
  `with_binding`.
- **`min_age_secs` defaults to 2**, which is new behaviour.  A test suite
  that submits instantly will now see `too_fast`; call `.with_min_age(0)` if
  you need the old behaviour.

A page rendered by 0.4 still submits successfully to a 0.5 server:
`submit_contact` accepts the old `csrf_token` field for one minor and uses it
when `form_token` is absent.

## API

`FormTokenConfig`, `FormToken`, `FormTokenContext`, `FormTokenError`,
`Binding`, `issue_form_token`, `issue_form_token_with_nonce`,
`verify_form_token` — see the
[API reference](../reference/api.md#form_token-module).
