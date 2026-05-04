# Cloudflare Turnstile Integration

[Cloudflare Turnstile](https://www.cloudflare.com/products/turnstile/) is a
privacy-friendly CAPTCHA alternative that runs a challenge in the browser
without showing a puzzle to most real users.

`leptos-hl-contact` does not bundle a Turnstile adapter, but integrating it
is straightforward.  This guide shows the pattern.

## How it works

1. The browser loads the Turnstile widget script and renders an invisible or
   managed challenge on your page.
2. On success, Turnstile sets a hidden `cf-turnstile-response` token in the
   form.
3. Your server function receives the token and verifies it with Cloudflare's
   siteverify API before delivering the message.

## Step 1 — Add the widget to your page

Add the Turnstile script and a widget `<div>` somewhere in your layout:

```html
<script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>

<!-- Place inside or near your ContactForm -->
<div class="cf-turnstile" data-sitekey="YOUR_SITE_KEY"></div>
```

In a Leptos SSR app, add this inside the `<head>` via `leptos_meta::Script`
and place the `<div>` in the component that renders `ContactForm`.

## Step 2 — Add the token field to your server function

Create a wrapper server function that accepts the additional token:

```rust,ignore
#[server]
pub async fn submit_contact_with_turnstile(
    name: String,
    email: String,
    subject: Option<String>,
    message: String,
    website: String,
    cf_turnstile_response: String,
) -> Result<(), ServerFnError> {
    // 1. Verify the Turnstile token.
    verify_turnstile(&cf_turnstile_response).await?;

    // 2. Delegate to the standard submit_contact.
    leptos_hl_contact::server::submit_contact(name, email, subject, message, website).await
}
```

## Step 3 — Implement token verification

```rust,ignore
use leptos::server_fn::error::ServerFnError;

async fn verify_turnstile(token: &str) -> Result<(), ServerFnError> {
    let secret = std::env::var("TURNSTILE_SECRET")
        .map_err(|_| ServerFnError::ServerError("Turnstile not configured".into()))?;

    let client = reqwest::Client::new();
    let res: serde_json::Value = client
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .form(&[("secret", secret.as_str()), ("response", token)])
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Turnstile request failed");
            ServerFnError::ServerError("Verification failed".into())
        })?
        .json()
        .await
        .map_err(|_| ServerFnError::ServerError("Verification failed".into()))?;

    if res["success"].as_bool() != Some(true) {
        tracing::warn!("Turnstile verification failed");
        return Err(ServerFnError::Args("Please complete the security check.".into()));
    }
    Ok(())
}
```

## Environment variables

```bash
TURNSTILE_SITE_KEY=0x4AAAAAAA...   # public — used in HTML
TURNSTILE_SECRET=0x4AAAAAAA...     # private — server only, never in WASM
```

## Notes

- The Turnstile widget token is automatically set in the `cf-turnstile-response`
  form field when the challenge completes.
- Use `data-theme="light"` / `"dark"` on the widget `<div>` to match your
  design.
- For test environments, use Cloudflare's
  [test keys](https://developers.cloudflare.com/turnstile/reference/testing/).
