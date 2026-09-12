# Cloudflare Turnstile

[Turnstile](https://www.cloudflare.com/products/turnstile/) is a
privacy-friendly CAPTCHA that usually needs no interaction from real
visitors.  Add it when the honeypot and rate limiting are not enough.

The crate does not bundle an adapter yet (roadmap P-21).  The pattern
below works with the current component.

## Why a plain wrapper does not work

`ContactForm` submits to `submit_contact` and has no slot for extra
markup, so you cannot put the widget's hidden `cf-turnstile-response`
field inside the form, and you cannot point the form at a wrapper server
function.  Verification therefore has to happen **before** the request
reaches the server function, in middleware, and the Turnstile token has
to travel in a cookie instead of a form field.

## Step 1 — Widget and cookie

Load the script and render the widget anywhere on the contact page.  On
success, store the token in a same-site cookie.

```html
<script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>

<div class="cf-turnstile"
     data-sitekey="YOUR_SITE_KEY"
     data-callback="onTurnstile"></div>

<script>
  function onTurnstile(token) {
    document.cookie = "cf_turnstile=" + encodeURIComponent(token)
      + "; Path=/; Max-Age=300; SameSite=Strict; Secure";
  }
</script>
```

Turnstile tokens are valid for 300 seconds and are accepted by Cloudflare
once.

## Step 2 — Verify in middleware

Check the cookie on every POST to the server-function path and call
Cloudflare's `siteverify` endpoint.

```rust,ignore
use axum::{body::Body, extract::Request, http::{StatusCode, header}, middleware::Next, response::Response};

async fn verify_turnstile(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if req.method() == axum::http::Method::POST && req.uri().path().starts_with("/api/") {
        let token = req.headers()
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|c| c.split(';').map(str::trim)
                .find_map(|kv| kv.strip_prefix("cf_turnstile=")))
            .map(|t| urlencoding::decode(t).unwrap_or_default().into_owned())
            .unwrap_or_default();

        let secret = std::env::var("TURNSTILE_SECRET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let ok = reqwest::Client::new()
            .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
            .form(&[("secret", secret.as_str()), ("response", token.as_str())])
            .send().await
            .and_then(|r| r.error_for_status())
            .map_err(|_| StatusCode::BAD_GATEWAY)?
            .json::<serde_json::Value>().await
            .map_err(|_| StatusCode::BAD_GATEWAY)?
            ["success"].as_bool() == Some(true);

        if !ok {
            tracing::warn!("Turnstile verification failed");
            return Err(StatusCode::FORBIDDEN);
        }
    }
    Ok(next.run(req).await)
}
```

Add it with `.layer(axum::middleware::from_fn(verify_turnstile))` next to
the other [hardening layers](./hardening.md#layer-order).  A `403` here
is shown to the visitor as the generic error banner; a localised "please
complete the check" message needs the error-code work in roadmap P-14.

## Environment

```bash
TURNSTILE_SITE_KEY=0x4AAAAAAA...   # public, used in HTML
TURNSTILE_SECRET=0x4AAAAAAA...     # server only
```

Use Cloudflare's [test keys](https://developers.cloudflare.com/turnstile/reference/testing/)
in development.

## Privacy note

The widget sends browser signals to Cloudflare.  Mention it in your
privacy notice.
