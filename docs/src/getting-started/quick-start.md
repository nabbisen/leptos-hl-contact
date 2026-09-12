# Quick Start

This page gets a contact form running in a Leptos + Axum application.  It
uses the no-op delivery backend so nothing is sent while you wire things
up.  When it works, continue with the
[Production Checklist](./production-checklist.md).

## Requirements

- Rust 1.85 or later (edition 2024)
- A Leptos v0.8 SSR application with Axum 0.8

## Step 1 — Add the dependency

Server binary:

```toml
[dependencies]
leptos-hl-contact = { version = "0.3", features = ["ssr", "smtp-lettre", "axum-helpers"] }
```

WASM binary:

```toml
[dependencies]
leptos-hl-contact = { version = "0.3", features = ["hydrate"] }
```

The crate has no default features.  See
[Feature Flags](../reference/feature-flags.md) for the full list.

## Step 2 — Choose a delivery backend

For local development, discard everything:

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};

let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

For real email, build the SMTP backend from environment variables:

```bash
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=your-smtp-password
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
```

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::delivery::{
    ContactDeliveryContext,
    smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode},
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

The crate never reads environment variables itself; you load them and pass
typed values.

## Step 3 — Provide the backend to Leptos

Everything the crate reads from context goes in the one closure you pass to
`leptos_routes_with_context`.  `delivery_context_fn` builds that closure for
the delivery backend.

```rust,ignore
use axum::Router;
use leptos::config::get_configuration;
use leptos_axum::{LeptosRoutes, generate_route_list};
use leptos_hl_contact::axum_helpers::delivery_context_fn;

let ctx = delivery_context_fn(delivery);

let conf = get_configuration(None).unwrap();
let leptos_options = conf.leptos_options.clone();
let routes = generate_route_list(App);

let app = Router::new()
    .leptos_routes_with_context(&leptos_options, routes, ctx, App)
    .with_state(leptos_options);
```

> One closure covers both page renders and `submit_contact`:
> `leptos_routes_with_context` registers each server function at its own path
> using the same closure.  You do not need a `/api/{*fn_name}` route of your
> own, and a context value provided only on such a route would not reach the
> server function.

## Step 4 — Place the component

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

That is a working form with server-side validation and a honeypot.

## Next steps

- [Production Checklist](./production-checklist.md) — what to add before the
  form is public
- [Customization](../guides/customization.md) — labels, classes, options,
  server policy
- [Axum Integration](../guides/axum-integration.md) — all context values in
  one place, middleware order
- [`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
  — complete production wiring
