# Hardening

Application-level controls the crate relies on but cannot provide itself.
All snippets are taken from
[`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security).

## HTTPS

Terminate TLS in front of the application.  Form contents and, in the
`csrf` case, tokens travel in the request; only TLS protects them in
transit.

## Request body limit

Reject oversized POSTs before any handler runs.  A contact form never
needs more than a few kilobytes.

```rust,ignore
use tower_http::limit::RequestBodyLimitLayer;

let app = app.layer(RequestBodyLimitLayer::new(32 * 1024));
```

## Rate limiting

Without it a public form will be flooded.  `tower_governor` keys on the
client IP and answers `429 Too Many Requests` when the budget is exceeded.

```toml
tower_governor = "0.8"
```

```rust,ignore
use std::sync::Arc;
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor};

let governor_config = Arc::new(
    GovernorConfigBuilder::default()
        .key_extractor(SmartIpKeyExtractor)
        .per_second(2)
        .burst_size(5)
        .finish()
        .expect("valid governor config"),
);

let app = app.layer(GovernorLayer::new(governor_config));
```

`SmartIpKeyExtractor` reads `X-Forwarded-For`.  Make sure your reverse
proxy sets it from the real connection and strips any value the client
sent, otherwise the limit can be bypassed with a forged header.

## Origin / Referer validation

This is the cross-site request forgery control.  Parse the header as a URL
and compare scheme, host, and port; a `starts_with` check would accept
`https://example.com.evil.test`.

```rust,ignore
use axum::{body::Body, extract::{Request, State}, http::{StatusCode, header}, middleware::Next, response::Response};
use url::Url;

fn origin_matches(value: &str, allowed: &Url) -> bool {
    let Ok(parsed) = Url::parse(value) else { return false };
    parsed.scheme() == allowed.scheme()
        && parsed.host_str() == allowed.host_str()
        && parsed.port_or_known_default() == allowed.port_or_known_default()
}

async fn check_origin(
    State(allowed): State<Arc<Url>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
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

Parse the allowed origin once at startup from `ALLOWED_ORIGIN` and refuse
to start if it is missing or invalid.  Browsers send `Origin` on
cross-origin form POSTs and on same-origin POSTs; `Referer` is the
fallback for older clients.

## Secrets

Load `SMTP_PASS`, `CSRF_SECRET`, and any API keys from environment
variables or a secret store.  Never commit them, never fall back to a
built-in default, and refuse to start when they are missing.  The crate's
config types redact secrets in `Debug` output so an accidental
`{:?}` does not leak them.

## Layer order

See [Axum Integration](../guides/axum-integration.md#middleware-order):
body limit outermost, then rate limit, then origin check.
