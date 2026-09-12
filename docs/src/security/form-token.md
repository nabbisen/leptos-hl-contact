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
| Cross-site POSTs are rejected | **no** — use [Origin validation](./hardening.md#origin--referer-validation) | yes |

With `Binding::None` the token is an anti-automation measure only, and the
control that rejects cross-site POSTs is the [Origin
check](./hardening.md#origin--referer-validation).  Turn binding on and the
token becomes a genuine double-submit CSRF control in its own right; keep the
Origin check as well, because two independent controls fail independently.

## Cookie binding

The token's nonce — never the token, never the secret — is also written to a
cookie that is `HttpOnly` and `SameSite=Lax`.  Verification then requires
both halves to agree:

- **`HttpOnly`** means no script can read the value, so a cross-site page
  cannot copy it into a forged form.
- **`SameSite=Lax`** means the browser does not send the cookie on a
  cross-site POST at all, so a forged submission arrives without it and
  fails with `BindingMissing`.

With Axum, two helpers do the work inside the one context closure:

```rust,ignore
use leptos_hl_contact::{
    axum_helpers::{FormTokenCookie, provide_form_token_binding, provide_form_token_with_cookie},
    form_token::{Binding, FormTokenConfig, FormTokenContext},
};

let token_config: FormTokenContext = Arc::new(
    FormTokenConfig::new(secret).with_binding(Binding::Cookie),
);
let token_cookie = FormTokenCookie::default();   // hl_contact_ft, Secure, Path=/

// In the closure passed to `leptos_routes_with_context`:
provide_context::<FormTokenContext>(Arc::clone(&token_config));
provide_form_token_with_cookie(&token_config, &token_cookie);
provide_form_token_binding(&token_cookie);
```

`provide_form_token_with_cookie` issues a token and sets the cookie **only on
`GET`**.  That matters: one closure serves page renders and server functions
alike, so issuing on a POST would overwrite the cookie the submitted form is
bound to and break the next submission.

`FormTokenCookie::secure` defaults to `true`.  A `Secure` cookie is never
sent back over plain HTTP, so set it to `false` — and only — when developing
against `http://localhost`.

Any other framework can use binding by providing
[`FormTokenBinding`](https://docs.rs/leptos-hl-contact/latest/leptos_hl_contact/form_token/struct.FormTokenBinding.html)
itself; the core never parses cookies.

> **Do not cache the contact page.**  Each render sets a fresh cookie paired
> with a fresh token.  A reverse proxy or CDN that caches the page, or strips
> `Set-Cookie` from a cached response, serves one visitor's token to another
> and every submission fails.  Send `Cache-Control: no-store` for that route.

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

Token format: `{unix_seconds}|{16-byte nonce hex}|{hmac_sha256 hex}`.
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
`Binding`, `issue_form_token`, `verify_form_token` — see the
[API reference](../reference/api.md#form_token-module).
