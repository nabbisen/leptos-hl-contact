# Quick Start

This guide walks through adding a working contact form to a Leptos + Axum
application from scratch.  Estimated time: **10 minutes**.

## Prerequisites

- Rust 1.85 or later
- A Leptos v0.8 SSR application with Axum
- An SMTP relay you can send from (or run [MailHog](https://github.com/mailhog/MailHog) locally)

---

## Step 1 — Add dependencies

In your **server binary** `Cargo.toml`:

```toml
[dependencies]
leptos-hl-contact = { version = "0.2", features = ["ssr", "smtp-lettre", "axum-helpers"] }
```

In your **WASM binary** `Cargo.toml`:

```toml
[dependencies]
leptos-hl-contact = { version = "0.2", features = ["hydrate"] }
```

---

## Step 2 — Set environment variables

```bash
# Never commit these values to source control.
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=your-smtp-password
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
```

Load them with [dotenvy](https://docs.rs/dotenvy) during development:

```rust,ignore
let _ = dotenvy::dotenv();
```

---

## Step 3 — Build the delivery backend

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode}},
};

let delivery: ContactDeliveryContext = Arc::new(LettreSmtpDelivery {
    config: SmtpConfig {
        host:           std::env::var("SMTP_HOST").expect("SMTP_HOST"),
        port:           587,
        username:       std::env::var("SMTP_USER").expect("SMTP_USER"),
        password:       std::env::var("SMTP_PASS").expect("SMTP_PASS"),
        from_address:   std::env::var("SMTP_FROM").expect("SMTP_FROM"),
        to_address:     std::env::var("CONTACT_TO").expect("CONTACT_TO"),
        subject_prefix: "[Contact]".into(),
        tls_mode:       SmtpTlsMode::StartTls,
    },
});
```

For local development without a real SMTP server, use `NoopDelivery` instead:

```rust,ignore
use leptos_hl_contact::delivery::noop::NoopDelivery;
let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

---

## Step 4 — Inject the delivery context into Axum

The delivery backend must be provided to **two** places in your Axum router.
`delivery_context_fn` builds a reusable closure that handles the `Arc::clone`
for you:

```rust,ignore
use axum::{Router, body::Body, extract::Request, routing::post};
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};

let ctx = delivery_context_fn(delivery);   // build once
let routes = generate_route_list(App);

let app = Router::new()
    // 1. Server function handler
    .route("/api/*fn_name", post({
        let ctx = ctx.clone();
        move |req: Request<Body>| {
            let ctx = ctx.clone();
            async move { handle_server_fns_with_context(ctx, req).await }
        }
    }))
    // 2. SSR renderer
    .leptos_routes_with_context(&leptos_options, routes, ctx, App)
    .with_state(leptos_options);
```

> **Why both?** Leptos server functions are dispatched by their own HTTP handler
> (`/api/*fn_name`), separately from the SSR renderer.  Both need access to
> the delivery context.

---

## Step 5 — Place the component

```rust,ignore
use leptos::prelude::*;
use leptos_hl_contact::ContactForm;

#[component]
fn ContactPage() -> impl IntoView {
    view! {
        <h1>"Contact us"</h1>
        <ContactForm />
    }
}
```

The form renders with English labels, no extra classes, and the default options
(subject field visible and optional, max 4 000 characters).

---

## Step 6 — Verify

1. Start your server.
2. Navigate to the page with `ContactForm`.
3. Fill in the form and submit.
4. If using `NoopDelivery`, check your server log for
   `NoopDelivery: discarding contact form submission`.
5. If using `LettreSmtpDelivery`, check your inbox.

---

## Next steps

- [Configuration](./configuration.md) — customise classes, labels, and options
- [Styling](./styling.md) — Tailwind / CSS class injection
- [Security](./security.md) — rate limiting, CSRF, deployment checklist
- [Delivery Backends](./delivery-backends.md) — implement a custom backend
- [Full Axum example](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-basic)
