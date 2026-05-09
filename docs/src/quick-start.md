# Quick Start

This guide gets a contact form running in a Leptos + Axum application in
a few minutes.  For production-ready wiring (rate limiting + CSRF + Origin
validation), see
[`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security).

## Prerequisites

- Rust 1.85 or later
- A Leptos v0.8 SSR application with Axum

---

## Step 1 — Add dependencies

**Server binary `Cargo.toml`:**

```toml
[dependencies]
leptos-hl-contact = { version = "0.3", features = ["ssr", "smtp-lettre", "axum-helpers"] }
```

**WASM binary `Cargo.toml`:**

```toml
[dependencies]
leptos-hl-contact = { version = "0.3", features = ["hydrate"] }
```

> For local development you can use the `NoopDelivery` backend, which
> discards all submissions.  Add the `smtp-lettre` feature only when you
> are ready to configure real SMTP.

---

## Step 2 — Set environment variables

```bash
# SMTP — never commit these values to source control
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=your-smtp-password
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
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

For local development, substitute `NoopDelivery`:

```rust,ignore
use leptos_hl_contact::delivery::noop::NoopDelivery;
let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

---

## Step 4 — Inject context into Axum

```rust,ignore
use axum::{Router, body::Body, extract::Request, routing::post};
use leptos_axum::{LeptosRoutes, generate_route_list, handle_server_fns_with_context};

let ctx = delivery_context_fn(delivery);
let routes = generate_route_list(App);

let app = Router::new()
    .route("/api/*fn_name", post({
        let ctx = ctx.clone();
        move |req: Request<Body>| {
            let ctx = ctx.clone();
            async move { handle_server_fns_with_context(ctx, req).await }
        }
    }))
    .leptos_routes_with_context(&leptos_options, routes, ctx, App)
    .with_state(leptos_options);
```

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

---

## Adding CSRF protection

The steps above do **not** enable CSRF token verification.  To enable it:

**1. Add the `csrf` feature:**

```toml
leptos-hl-contact = { version = "0.3", features = ["ssr", "smtp-lettre", "axum-helpers", "csrf"] }
```

**2. Generate and set `CSRF_SECRET`:**

```bash
CSRF_SECRET=$(openssl rand -hex 32)
```

**3. Inject `CsrfConfigContext` and `CsrfToken` in both handler closures:**

```rust,ignore
use leptos_hl_contact::csrf::{CsrfConfig, CsrfConfigContext, generate_csrf_token};

let csrf_config: CsrfConfigContext = Arc::new(CsrfConfig {
    secret_key:     std::env::var("CSRF_SECRET").expect("CSRF_SECRET").into_bytes(),
    token_ttl_secs: 3600,
});
let csrf_for_fn  = Arc::clone(&csrf_config);
let csrf_for_ssr = Arc::clone(&csrf_config);

// Server function handler — inject config for verification:
.route("/api/*fn_name", post({
    let ctx  = ctx.clone();
    let csrf = Arc::clone(&csrf_for_fn);
    move |req: Request<Body>| {
        let ctx  = ctx.clone();
        let csrf = Arc::clone(&csrf);
        async move {
            handle_server_fns_with_context(move || {
                ctx();
                leptos::context::provide_context::<CsrfConfigContext>(Arc::clone(&csrf));
            }, req).await
        }
    }
}))
// SSR renderer — inject config AND a fresh per-request token:
.leptos_routes_with_context(&opts, routes, move || {
    ctx.clone()();
    leptos::context::provide_context::<CsrfConfigContext>(Arc::clone(&csrf_for_ssr));
    leptos::context::provide_context(generate_csrf_token(&csrf_for_ssr));
}, App)
```

> **Important:** when `csrf` feature is enabled, `CsrfConfigContext` **must**
> be provided in both closures.  If it is absent, `submit_contact` returns an
> error (fail-closed behaviour).

See [`examples/axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
for the complete production-ready integration.

---

## Next steps

- [Configuration](./configuration.md) — customise classes, labels, and options
- [Security](./security.md) — rate limiting, deployment checklist
- [CSRF Protection](./csrf.md) — full CSRF guide
- [Styling](./styling.md) — Tailwind / CSS class injection
