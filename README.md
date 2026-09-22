# leptos-hl-contact

[![crates.io](https://img.shields.io/crates/v/leptos-hl-contact?label=leptos-hl-contact)](https://crates.io/crates/leptos-hl-contact)
[![Rust Documentation](https://docs.rs/leptos-hl-contact/badge.svg?version=latest)](https://docs.rs/leptos-hl-contact)
[![Dependency Status](https://deps.rs/crate/leptos-hl-contact/latest/status.svg)](https://deps.rs/crate/leptos-hl-contact)
[![CI](https://github.com/nabbisen/leptos-hl-contact/actions/workflows/ci.yml/badge.svg)](https://github.com/nabbisen/leptos-hl-contact/actions)
[![License](https://img.shields.io/github/license/nabbisen/leptos-hl-contact)](https://github.com/nabbisen/leptos-hl-contact/blob/main/LICENSE)

**A reusable, secure contact form for [Leptos](https://leptos.dev) v0.8.**

One component, one server function, one delivery trait.  Server-side
validation, honeypot, header-injection protection, progressive
enhancement, and accessibility come built in.

---

## Overview

```
ContactForm  →  submit_contact (server fn)  →  ContactDelivery (trait)
```

- **`ContactForm`** renders an accessible `<ActionForm/>` that still works
  as a plain POST without JavaScript.
- **`submit_contact`** runs on the server only: token check, normalise,
  honeypot, validate, policy, deliver.
- **`ContactDelivery`** is a trait.  SMTP (`LettreSmtpDelivery`), Resend's
  HTTP API (`ResendDelivery`, native and Cloudflare Workers), and a no-op
  backend ship with the crate; anything else is one `impl` away.

---

## When to use it

- A Leptos SSR or Islands app needs a contact form.
- You want SMTP or Resend delivery with minimal wiring and no credentials in
  WASM.
- You may swap the backend later (SendGrid, SES, a database).

---

## Quick Start

**1. Dependencies**

```toml
# server binary
leptos-hl-contact = { version = "0.8", features = ["ssr", "smtp-lettre", "axum-helpers"] }

# WASM binary
leptos-hl-contact = { version = "0.8", features = ["hydrate"] }
```

**2. Delivery backend and Axum wiring**

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::{
    axum_helpers::delivery_context_fn,
    delivery::{ContactDeliveryContext, smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode}},
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
        timeout:        SmtpConfig::DEFAULT_TIMEOUT,
    },
});
let ctx = delivery_context_fn(delivery);
// pass `ctx` to leptos_routes_with_context; it serves page renders and
// server functions alike
```

**3. Component**

```rust,ignore
use leptos_hl_contact::ContactForm;

view! { <ContactForm /> }
```

Full walkthrough: [Quick Start](./docs/src/getting-started/quick-start.md).
Before going public, work through the
[Production Checklist](./docs/src/getting-started/production-checklist.md);
[`examples/axum-with-security`](./examples/axum-with-security) implements it.

---

## Design notes

- **Secure by default.**  Credentials, recipient, and token secret exist
  only in server-side types.
- **Progressive enhancement.**  Plain POST without JavaScript.
- **Accessible by default.**  Labels, ARIA, live regions, keyboard.
- **Pluggable delivery.**  Framework-neutral trait; Axum helpers are opt-in.
- **A few fields of your own.**  Up to four site-defined fields (one line,
  several lines, or a choice), validated by the server against the same
  definition the form renders from; not a form builder.  See
  [Customization](./docs/src/guides/customization.md#site-defined-fields).
- **Runs on Cloudflare Workers.**  The server path builds for wasm32 with no
  tokio runtime; see the [Cloudflare Workers guide](./docs/src/guides/cloudflare-workers.md).
- **Honest security model.**  The `form-token` feature's token is an
  anti-automation measure with a minimum age; Origin validation in your
  middleware is the CSRF control.  Both are documented, with examples.

---

## Documentation

The full book lives in [`docs/src`](./docs/src/SUMMARY.md) (mdBook):

| Section | Start here |
|---------|------------|
| Getting Started | [Quick Start](./docs/src/getting-started/quick-start.md), [Production Checklist](./docs/src/getting-started/production-checklist.md) |
| Guides | [Customization](./docs/src/guides/customization.md), [Delivery Backends](./docs/src/guides/delivery-backends.md), [Axum Integration](./docs/src/guides/axum-integration.md) |
| Security | [Overview](./docs/src/security/README.md), [Hardening](./docs/src/security/hardening.md) |
| Reference | [API](./docs/src/reference/api.md), [Feature Flags](./docs/src/reference/feature-flags.md) |
| Development | [Requirements](./docs/src/development/requirements.md), [External Design](./docs/src/development/external-design.md), [Architecture](./docs/src/development/architecture.md) |

Rustdoc: [docs.rs/leptos-hl-contact](https://docs.rs/leptos-hl-contact).
Plans: [`ROADMAP.md`](./ROADMAP.md) and [`rfcs/`](./rfcs/README.md).
