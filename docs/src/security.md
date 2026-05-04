# Security

This page explains the security model of `leptos-hl-contact` and the steps
you should take before exposing the form publicly.

## What the crate handles

| Concern | Handled |
|---------|---------|
| Server-side input validation | ✅ Always, regardless of client state |
| Per-field validation error feedback | ✅ Generic messages, no PII echo |
| Honeypot bot detection | ✅ Built-in |
| Email header injection | ✅ Newlines rejected in `name` / `subject` |
| Credential isolation | ✅ SMTP config never reaches WASM |
| Generic client errors | ✅ Internal details logged; not forwarded |
| Input escaping in `view!` | ✅ Leptos escapes output by default |

## What you must handle in your application

### HTTPS

Run behind TLS in production. Credentials and form data are transmitted
over HTTP — only TLS prevents interception.

### Rate limiting

A public form without rate limiting will be flooded by bots.  Add
middleware at the Axum layer before going live.

Recommended crates:

| Crate | Notes |
|-------|-------|
| [`tower-governor`](https://docs.rs/tower-governor) | Tower middleware, IP-based leaky-bucket |
| [`axum-governor`](https://docs.rs/axum-governor) | Axum wrapper around tower-governor |

#### Axum example

```rust,ignore
use std::sync::Arc;
use axum_governor::{GovernorConfigBuilder, GovernorLayer};

let governor_config = Arc::new(
    GovernorConfigBuilder::default()
        .per_second(2)   // 2 requests per second sustained
        .burst_size(5)   // up to 5 at once
        .finish()
        .unwrap(),
);

let app = Router::new()
    // … your routes …
    .layer(GovernorLayer { config: governor_config });
```

Apply the layer **before** the server-function route so that bursts of
automated POST requests are throttled.

### CSRF protection

Leptos `<ActionForm/>` uses standard HTML form `POST` semantics. Protect
against cross-site request forgery with:

1. **`SameSite=Lax` (or `Strict`) cookies** — prevents cross-site form
   submission in most browsers.  This is the minimum viable protection.

2. **`Origin` / `Referer` header validation** — reject requests whose
   `Origin` does not match your domain.

3. **CSRF tokens** — for applications with stricter requirements, use a
   signed, per-session token embedded as a hidden form field and verified
   server-side.

#### Axum middleware snippet

```rust,ignore
use axum::{
    extract::Request,
    http::{StatusCode, header},
    middleware::{Next, from_fn},
    response::Response,
};

async fn check_origin(req: Request, next: Next) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::POST {
        let origin = req.headers()
            .get(header::ORIGIN)
            .or_else(|| req.headers().get(header::REFERER))
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !origin.starts_with("https://your-domain.com") {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(req).await)
}

// Add to your router:
let app = Router::new()
    // … routes …
    .layer(from_fn(check_origin));
```

### Cloudflare Turnstile (optional CAPTCHA)

For high-traffic or high-value forms, add a CAPTCHA. See
[Turnstile integration](./turnstile.md) for a step-by-step guide.

### Secrets management

Load SMTP credentials from environment variables or a dedicated secret
store (HashiCorp Vault, AWS Secrets Manager, etc.).  Never commit them
to source control.

```bash
# .env — never committed, loaded via dotenvy in development
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=super-secret
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
```

## Deployment checklist

- [ ] HTTPS enabled (TLS termination by reverse proxy or load balancer)
- [ ] Rate limiting middleware configured
- [ ] `SameSite=Lax` or `Strict` cookies set on the session
- [ ] `Origin` / `Referer` validation enabled (or CSRF token in use)
- [ ] SMTP credentials loaded from environment variables, not source code
- [ ] Logs contain no raw message bodies, email addresses, or passwords
- [ ] Reverse proxy strips or validates `X-Forwarded-For` for IP-based limits
- [ ] Form smoke-tested with JavaScript disabled (progressive enhancement)
