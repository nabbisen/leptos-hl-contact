# CSRF Protection

`leptos-hl-contact` provides a built-in CSRF token helper via the `csrf`
feature flag.  It uses stateless HMAC-SHA256 tokens — no session storage
or database required.

## How it works

```
Server startup
  └─ CsrfConfig { secret_key, token_ttl_secs }
       └─ provided as CsrfConfigContext via Leptos context
              (in both SSR renderer + server-fn handler)

Per SSR render
  └─ generate_csrf_token(&config) → CsrfToken
       └─ provided as CsrfToken via Leptos context
            └─ ContactForm embeds token in hidden form field

Form submission
  └─ submit_contact(…, csrf_token: String)
       └─ use_context::<CsrfConfigContext>()
            └─ verify_csrf_token(&token, &config) → bool
```

## Token format

```
{unix_timestamp}|{random_nonce_hex}|{hmac_sha256_hex}
```

- **Timestamp** — creation time; used for TTL expiry check.
- **Nonce** — 16 random bytes; ensures two renders in the same second produce
  different tokens.
- **HMAC** — HMAC-SHA256 of `{timestamp}|{nonce}` using the secret key;
  verified with constant-time comparison to prevent timing attacks.

## Setup

### 1. Add the feature

```toml
leptos-hl-contact = { version = "0.3", features = ["ssr", "csrf"] }
```

### 2. Generate a secret key

```bash
# At least 32 bytes; add to your server-side .env (never commit)
openssl rand -hex 32
```

### 3. Configure at startup

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::csrf::{CsrfConfig, CsrfConfigContext};

let csrf_config: CsrfConfigContext = Arc::new(CsrfConfig {
    secret_key:     std::env::var("CSRF_SECRET")
                        .expect("CSRF_SECRET must be set")
                        .into_bytes(),
    token_ttl_secs: 3600, // one hour
});
```

### 4. Inject into both Axum handler sites

```rust,ignore
use leptos_hl_contact::csrf::{CsrfConfigContext, generate_csrf_token};
use leptos::context::provide_context;

// ── Server function handler ───────────────────────────────────────────────
let csrf = Arc::clone(&csrf_config);
handle_server_fns_with_context(move || {
    ctx();
    // Provides config so submit_contact can verify tokens.
    provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
}, req)

// ── SSR renderer ─────────────────────────────────────────────────────────
.leptos_routes_with_context(&opts, routes, move || {
    ctx.clone()();
    provide_context::<CsrfConfigContext>(Arc::clone(&csrf_config));
    // Generate a unique token for this render; ContactForm embeds it.
    provide_context(generate_csrf_token(&csrf_config));
}, App)
```

`ContactForm` detects `CsrfToken` in context and automatically inserts:

```html
<input type="hidden" name="csrf_token" value="{token}" />
```

### 5. That's it

`submit_contact` verifies the token automatically when `CsrfConfigContext`
is in context.  No changes to the component or server function call sites
are needed.

## Backward compatibility

When `CsrfConfigContext` is **not** provided:

- No hidden field is added to the form (token is an empty string).
- `submit_contact` skips CSRF verification.

This means upgrading from v0.2 to v0.3 does not break existing deployments.
You opt in to CSRF protection by providing the context.

## Complete example

See [`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
for a full working integration with rate limiting and Origin validation.

## API reference

See [API Reference — CSRF Token Helper](./api-reference.md#csrf-token-helper).
