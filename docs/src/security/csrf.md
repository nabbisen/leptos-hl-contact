# Anti-automation Token (`csrf` feature)

A stateless, HMAC-SHA256-signed token that the SSR renderer places in a
hidden `csrf_token` field and the server function verifies before it looks
at anything else.  No session store or database is involved.

## What it guarantees

| Property | Guaranteed? |
|----------|-------------|
| The sender obtained a token from this server within `token_ttl_secs` | yes |
| The token was not forged (HMAC with your secret, constant-time compare) | yes |
| The token was issued to the browser that is now submitting | **no** — it is not bound to a cookie or session |
| The token cannot be reused | **no** — any token is valid until it expires |
| Cross-site POSTs are rejected | **no** — use [Origin validation](./hardening.md#origin--referer-validation) for that |

In practice the token stops bots that POST without first fetching the page
and bounds replay to the TTL.  Read the [Security overview](./README.md#about-the-token)
for how it fits with the other layers.

## How it works

```text
startup        CsrfConfig { secret_key, token_ttl_secs }
                 → provided as CsrfConfigContext at BOTH context sites

each SSR       generate_csrf_token(&config) → CsrfToken
render           → provided via context (SSR renderer only)
                 → ContactForm renders <input type="hidden" name="csrf_token">

each submit    submit_contact receives csrf_token
                 → verify_csrf_token(token, &config)
                 → false: contact_error:token_invalid
                           (rendered from labels.errors.token_invalid)
```

Token format: `{unix_seconds}|{16-byte nonce hex}|{hmac_sha256 hex}`.
Verification rejects a bad signature, a token older than the TTL, or one
more than 60 seconds in the future.

## Setup

1. Enable the feature on the server binary:

   ```toml
   leptos-hl-contact = { version = "0.4", features = ["ssr", "smtp-lettre", "axum-helpers", "csrf"] }
   ```

2. Generate a secret and keep it server-side:

   ```bash
   CSRF_SECRET=$(openssl rand -hex 32)
   ```

3. Build the config once:

   ```rust,ignore
   use std::sync::Arc;
   use leptos_hl_contact::csrf::{CsrfConfig, CsrfConfigContext};

   let csrf: CsrfConfigContext = Arc::new(CsrfConfig {
       secret_key:     std::env::var("CSRF_SECRET").expect("CSRF_SECRET").into_bytes(),
       token_ttl_secs: 3600,
   });
   ```

4. Provide it in the context closure, with a fresh token per render:

   ```rust,ignore
   // The closure passed to `leptos_routes_with_context`
   provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
   provide_context(generate_csrf_token(&csrf));
   ```

   The token is issued on every request through that closure and used only
   by page renders; `submit_contact` verifies the token the form submitted.

   The full router is in [Axum Integration](../guides/axum-integration.md#all-context-values-together).

`ContactForm` and `submit_contact` need no changes.

## Fail-closed behaviour

When the feature is enabled and `CsrfConfigContext` is missing from the
context closure, every submission is rejected with the `not_configured`
code — rendered from `labels.errors.not_configured` — and an `error` log
line.  This is
deliberate: forgetting the context must not silently disable the check.
(Version 0.3.0 skipped verification in that case; 0.3.1 and later fail
closed.)

If `CsrfToken` is missing, the hidden field is empty and every submission
fails verification with the "reload the page" message.

## API

`CsrfConfig`, `CsrfToken`, `CsrfConfigContext`, `generate_csrf_token`,
`verify_csrf_token` — see the [API reference](../reference/api.md#csrf-module).
