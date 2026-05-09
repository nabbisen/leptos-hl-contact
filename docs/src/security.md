# Security

This page explains the security model of `leptos-hl-contact` and the steps
you must take before exposing the form publicly.

## What the crate handles

| Concern | Handled | Notes |
|---------|---------|-------|
| Server-side input validation | ✅ Always | Regardless of client state |
| Per-field validation errors | ✅ Generic messages | Never echoes user input |
| Honeypot bot detection | ✅ Built-in | Silent success on trigger |
| Email header injection | ✅ Newlines rejected | In `name` / `subject` |
| Credential isolation | ✅ | SMTP config never reaches WASM |
| Generic client errors | ✅ | Internal details logged only |
| CSRF token generation + verification | ✅ (`csrf` feature) | HMAC-SHA256 stateless tokens |
| Input escaping in `view!` | ✅ | Leptos escapes output by default |

## What you must configure in your application

### HTTPS

Run behind TLS in production.  Credentials and form data are transmitted
over HTTP — only TLS prevents interception.

---

## CSRF Protection (`csrf` feature)

`leptos-hl-contact` ships a stateless HMAC-SHA256 CSRF token helper that
requires no session storage or database.

### How it works

1. At startup, create a `CsrfConfig` with a secret key loaded from an
   environment variable.
2. Provide `Arc<CsrfConfig>` (`CsrfConfigContext`) to both the SSR renderer
   and the server-function handler via Leptos context.
3. In the SSR renderer, also call `generate_csrf_token(&config)` and provide
   the `CsrfToken` via context. Each page render gets a unique token.
4. `ContactForm` automatically embeds the token in a hidden
   `<input name="csrf_token">` field.
5. `submit_contact` verifies the token before processing the submission.

### Setup

```toml
leptos-hl-contact = { version = "0.3", features = ["ssr", "csrf"] }
```

```bash
# .env — server-side only, never commit
CSRF_SECRET=<output of: openssl rand -hex 32>
```

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token};

let csrf_config: CsrfConfigContext = Arc::new(CsrfConfig {
    secret_key:     std::env::var("CSRF_SECRET").expect("CSRF_SECRET").into_bytes(),
    token_ttl_secs: 3600,
});

// In both SSR renderer and server-fn handler closures:
provide_context::<CsrfConfigContext>(Arc::clone(&csrf_config));

// In SSR renderer closure only (generates unique token per page render):
provide_context(generate_csrf_token(&csrf_config));
```

See [`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
for the complete wiring.

### Token format

```
{unix_timestamp_secs}|{random_nonce_hex}|{hmac_sha256_hex}
```

Each token encodes its creation time and a random 16-byte nonce, signed with
HMAC-SHA256.  Verification checks the signature (constant-time comparison) and
rejects tokens older than `token_ttl_secs`.

---

## Rate limiting

A public form without rate limiting will be flooded by bots.  Add middleware
at the Axum layer before going live.

### `tower_governor` example

```toml
tower_governor = { version = "0.8" }
```

```rust,ignore
use std::sync::Arc;
use tower_governor::{
    GovernorLayer,
    governor::GovernorConfigBuilder,
    key_extractor::SmartIpKeyExtractor,
};

let governor_config = Arc::new(
    GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_second(2)
        .burst_size(5)
        .finish()
        .expect("valid config"),
);

let app = Router::new()
    // … routes …
    .layer(GovernorLayer::new(governor_config));
```

Exceeding the limit returns HTTP 429 Too Many Requests automatically.

> In production, ensure your reverse proxy sets and validates
> `X-Forwarded-For` before traffic reaches Axum, so
> `SmartIpKeyExtractor` reads the real client IP.

---

## Origin / Referer validation

Add a middleware that rejects POST requests from unknown origins.
This is a complementary layer to CSRF tokens.

```rust,ignore
use axum::{extract::Request, http::{StatusCode, header}, middleware::Next, response::Response};
use url::Url;

// Parse once at startup; panics on invalid URL (intentional — misconfiguration).
let allowed: Url = Url::parse("https://your-domain.com").unwrap();

fn origin_matches(value: &str, allowed: &Url) -> bool {
    let Ok(parsed) = Url::parse(value) else { return false; };
    parsed.scheme() == allowed.scheme()
        && parsed.host_str() == allowed.host_str()
        && parsed.port_or_known_default() == allowed.port_or_known_default()
}

async fn check_origin(req: Request, next: Next) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::POST {
        let value = req.headers()
            .get(header::ORIGIN)
            .or_else(|| req.headers().get(header::REFERER))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !origin_matches(value, &allowed) {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(req).await)
}
```

> **Why not `starts_with`?**  A check like `origin.starts_with("https://example.com")`
> would accept `https://example.com.evil.test`.  Parsing the URL and comparing
> scheme, host, and port individually prevents this class of bypass.

---

## Cloudflare Turnstile (optional CAPTCHA)

For high-traffic or high-value forms, add a CAPTCHA.  See
[Turnstile Integration](./turnstile.md) for a step-by-step guide.

---

## Secrets management

Load all secrets from environment variables or a dedicated secret store
(HashiCorp Vault, AWS Secrets Manager, etc.).  Never commit them to source
control.

```bash
# .env (never committed — loaded via dotenvy in development)
CSRF_SECRET=<32+ random bytes>
SMTP_PASS=<smtp password>
```

---

## Deployment checklist

- [ ] HTTPS enabled (TLS termination by reverse proxy or load balancer)
- [ ] `CSRF_SECRET` set to a 32+ byte random value in production
- [ ] Rate limiting middleware configured
- [ ] Origin / Referer validation middleware enabled
- [ ] `SameSite=Lax` or `Strict` cookies set on the session
- [ ] SMTP credentials loaded from environment variables, not source code
- [ ] Server logs contain no raw message bodies or passwords
- [ ] Reverse proxy validates `X-Forwarded-For` for IP-based rate limits
- [ ] Form smoke-tested with JavaScript disabled
