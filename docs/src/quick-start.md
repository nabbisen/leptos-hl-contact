# Quick Start

This guide gets a contact form running in a Leptos + Axum application in a few minutes.

## 1. Add the dependency

In your server binary's `Cargo.toml`:

```toml
[dependencies]
leptos-hl-contact = { version = "0.1", features = ["ssr", "smtp-lettre"] }
```

In your client/WASM binary's `Cargo.toml`:

```toml
[dependencies]
leptos-hl-contact = { version = "0.1", features = ["hydrate"] }
```

## 2. Set environment variables

```bash
SMTP_HOST=smtp.example.com
SMTP_USER=you@example.com
SMTP_PASS=your-password
SMTP_FROM=noreply@example.com
CONTACT_TO=inbox@example.com
```

Never hard-code credentials. Load them from environment variables or a secret store.

## 3. Configure the delivery backend

In your Axum `main.rs`:

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::delivery::{
    ContactDeliveryContext,
    smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode},
};

let delivery: ContactDeliveryContext = Arc::new(LettreSmtpDelivery {
    config: SmtpConfig {
        host:           std::env::var("SMTP_HOST")?,
        port:           587,
        username:       std::env::var("SMTP_USER")?,
        password:       std::env::var("SMTP_PASS")?,
        from_address:   std::env::var("SMTP_FROM")?,
        to_address:     std::env::var("CONTACT_TO")?,
        subject_prefix: "[Contact]".into(),
        tls_mode:       SmtpTlsMode::StartTls,
    },
});
```

## 4. Provide context to both handlers

Axum requires the context to be provided to **both** the server-function handler and the SSR renderer:

```rust,ignore
// Server function handler
.route("/api/*fn_name", post(move |req| {
    let d = Arc::clone(&delivery);
    handle_server_fns_with_context(move || {
        leptos::context::provide_context(Arc::clone(&d));
    }, req)
}))
// SSR renderer
.leptos_routes_with_context(&opts, routes, move || {
    leptos::context::provide_context(Arc::clone(&delivery));
}, App)
```

See [`examples/axum-basic`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-basic) for the complete wiring.

## 5. Add the component

```rust,ignore
use leptos_hl_contact::ContactForm;

view! { <ContactForm /> }
```

That's it. The form is live with English labels, no extra CSS classes, and the default options (subject field visible, not required; max 4 000 characters).

## Next steps

- [Styling](./styling.md) — inject Tailwind / CSS classes
- [Configuration](./configuration.md) — customise labels and options
- [Security](./security.md) — rate limiting, CSRF, deployment checklist
