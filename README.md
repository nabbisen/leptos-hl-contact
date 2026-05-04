# leptos-hl-contact

[![crates.io](https://img.shields.io/crates/v/leptos-hl-contact?label=leptos-hl-contact)](https://crates.io/crates/leptos-hl-contact)
[![Rust Documentation](https://docs.rs/leptos-hl-contact/badge.svg?version=latest)](https://docs.rs/leptos-hl-contact)
[![Dependency Status](https://deps.rs/crate/leptos-hl-contact/latest/status.svg)](https://deps.rs/crate/leptos-hl-contact)
[![License](https://img.shields.io/github/license/nabbisen/leptos-hl-contact)](https://github.com/nabbisen/leptos-hl-contact/blob/main/LICENSE)

**A reusable, secure contact form plugin for [Leptos](https://leptos.dev) v0.8.**

Drop a single component into your Leptos app, wire up an SMTP server, and your visitors have a working contact form — with server-side validation, honeypot bot protection, progressive enhancement, and full accessibility out of the box.

---

## Overview

`leptos-hl-contact` is not just a UI component.  It ships three cooperating layers:

```
UI Component  →  Server Function  →  Delivery Backend
```

The **UI component** (`ContactForm`) renders an accessible HTML form and uses `<ActionForm/>` so it degrades gracefully to a plain POST when JavaScript is unavailable.

The **server function** (`submit_contact`) runs entirely on the server: it normalises input, checks the honeypot, validates every field, then dispatches to the delivery backend.

The **delivery backend** is an abstract trait (`ContactDelivery`).  The crate ships an SMTP implementation (`LettreSmtpDelivery`) and a no-op stub (`NoopDelivery`) for local development.

---

## When to use this

- You need a contact form in a Leptos SSR or Islands app.
- You want SMTP email delivery with zero boilerplate.
- You care about security: no credentials in WASM, server-side validation, honeypot, and proper header sanitisation.
- You want to swap the delivery backend later (SendGrid, SES, Resend, database …).

---

## Quick Start

### 1. Add to `Cargo.toml`

```toml
[dependencies]
leptos-hl-contact = { version = "0.1", features = ["ssr", "smtp-lettre"] }

# client-side binary
[dev-dependencies]
leptos-hl-contact = { version = "0.1", features = ["hydrate"] }
```

### 2. Provide the delivery backend (Axum example)

```rust,ignore
use std::sync::Arc;
use leptos_contact_form::{
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
    },
});

// Provide to both the server-function handler and the SSR renderer.
```

See [`examples/axum-basic`](./examples/axum-basic) for the full Axum wiring.

### 3. Place the component

```rust,ignore
use leptos_contact_form::ContactForm;

view! { <ContactForm /> }
```

Customise classes, labels, and options via props — see the [Styling](./docs/src/styling.md) and [Configuration](./docs/src/configuration.md) docs.

---

## Feature flags

| Flag           | What it enables                                    |
|----------------|----------------------------------------------------|
| `hydrate`      | Client-side hydration                              |
| `ssr`          | Server-side rendering + server functions           |
| `islands`      | Leptos Islands architecture                        |
| `smtp-lettre`  | SMTP delivery via `lettre` (implies `ssr`)         |
| `axum-helpers` | Axum integration helpers (implies `ssr`)           |

---

## Design notes

- **Secure by default** — SMTP credentials never touch the client.
- **Progressive enhancement** — works without WASM via `<ActionForm/>`.
- **Pluggable delivery** — implement `ContactDelivery` to add any backend.
- **Accessible by default** — labels, ARIA attributes, keyboard navigation.
- **No framework lock-in** — delivery trait is framework-neutral; Axum helpers are opt-in.

---

## Full documentation

[**docs.rs/leptos-hl-contact**](https://docs.rs/leptos-hl-contact)

Key chapters:

- [Quick Start](./docs/src/quick-start.md)
- [Security](./docs/src/security.md)
- [Delivery Backends](./docs/src/delivery-backends.md)
- [Styling](./docs/src/styling.md)
- [Accessibility](./docs/src/accessibility.md)
