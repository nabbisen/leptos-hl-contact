# Security

This page explains the security model of `leptos-hl-contact` and the steps you should take before exposing the form publicly.

## What the crate handles

| Concern | Handled |
|---------|---------|
| Server-side validation | ✅ Always, regardless of client state |
| Honeypot bot detection | ✅ Built in |
| Email header injection | ✅ Newlines rejected in `name` / `subject` |
| Credential isolation | ✅ SMTP config never reaches WASM |
| Generic client errors | ✅ Internal details logged; not forwarded |
| Input sanitisation | ✅ `view!` escapes output by default |

## What you must handle in your application

### HTTPS

Run your Leptos application behind TLS in production.  Credentials and form submissions are sent over HTTP — only TLS prevents interception.

### Rate limiting

A public form without rate limiting will be flooded by bots.  Add a rate-limiting middleware at the Axum layer before going live.

Recommended libraries:
- [`tower-governor`](https://docs.rs/tower-governor) — Tower middleware, works with Axum
- [`axum-governor`](https://docs.rs/axum-governor) — Axum-specific wrapper

Example configuration (Axum):

```rust,ignore
use axum_governor::{GovernorConfigBuilder, GovernorLayer};

let governor_config = GovernorConfigBuilder::default()
    .per_second(2)
    .burst_size(5)
    .finish()
    .unwrap();

let app = Router::new()
    // ... routes ...
    .layer(GovernorLayer { config: Arc::new(governor_config) });
```

### CSRF protection

Leptos `<ActionForm/>` uses standard HTML form POST semantics.  Protect it with:

- `SameSite=Lax` (or `Strict`) cookies — prevents cross-site form submissions in most browsers.
- `Origin` / `Referer` header validation at the middleware layer.
- A CSRF token if your threat model requires it.

### Secrets management

Load SMTP credentials from environment variables or a dedicated secret store (HashiCorp Vault, AWS Secrets Manager, etc.).  Never commit them to source control.

```bash
# .env (never committed)
SMTP_PASS=super-secret
```

### Optional: CAPTCHA

For high-traffic or high-value forms, add a CAPTCHA.  Cloudflare Turnstile is a low-friction option.  Integration guidance will be added in a future release.

## Deployment checklist

- [ ] HTTPS enabled (TLS termination by reverse proxy or load balancer)
- [ ] Rate limiting middleware configured
- [ ] `SameSite=Lax` or `Strict` cookies
- [ ] SMTP credentials loaded from environment variables
- [ ] Logs do not contain raw message bodies or passwords
- [ ] Reverse proxy strips or validates `X-Forwarded-For` for IP-based rate limiting
- [ ] Form is smoke-tested with JavaScript disabled
