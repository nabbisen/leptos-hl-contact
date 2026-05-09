# leptos-hl-contact

[![crates.io](https://img.shields.io/crates/v/leptos-hl-contact?label=leptos-hl-contact)](https://crates.io/crates/leptos-hl-contact)
[![Rust Documentation](https://docs.rs/leptos-hl-contact/badge.svg?version=latest)](https://docs.rs/leptos-hl-contact)
[![Dependency Status](https://deps.rs/crate/leptos-hl-contact/latest/status.svg)](https://deps.rs/crate/leptos-hl-contact)
[![CI](https://github.com/nabbisen/leptos-hl-contact/actions/workflows/ci.yml/badge.svg)](https://github.com/nabbisen/leptos-hl-contact/actions)
[![License](https://img.shields.io/github/license/nabbisen/leptos-hl-contact)](https://github.com/nabbisen/leptos-hl-contact/blob/main/LICENSE)

**A reusable, secure contact form plugin for [Leptos](https://leptos.dev) v0.8.**

Drop a single component into your Leptos app, wire up SMTP, and your visitors
have a working contact form — with server-side validation, honeypot bot
protection, progressive enhancement, and full accessibility out of the box.

---

## Overview

`leptos-hl-contact` is three layers working together:

```
ContactForm  →  submit_contact (server fn)  →  ContactDelivery (trait)
```

The **UI component** renders an accessible HTML form using `<ActionForm/>`,
which degrades gracefully to a plain POST without JavaScript.

The **server function** runs on the server only: it normalises input, checks
the honeypot, validates every field, then hands off to the delivery backend.

The **delivery backend** is an abstract trait. The crate ships SMTP
(`LettreSmtpDelivery`) and a no-op stub (`NoopDelivery`) for local development.

---

## When to use this

- You have a Leptos SSR or Islands app and need a contact form.
- You want SMTP delivery with minimal boilerplate.
- Security matters: no credentials in WASM, server-side validation, honeypot,
  header-injection protection.
- You may need to swap the delivery backend later (SendGrid, SES, Resend, DB…).

---

> **Production checklist:** rate limiting, CSRF (`csrf` feature), HTTPS, and Origin
> validation are required before going public.  See [Security](./docs/src/security.md)
> and [`examples/axum-with-security`](./examples/axum-with-security).

## Quick Start

**1. Add the dependency**

```toml
# server binary
leptos-hl-contact = { version = "0.3", features = ["ssr", "smtp-lettre", "axum-helpers", "csrf"] }

# WASM binary
leptos-hl-contact = { version = "0.3", features = ["hydrate"] }
```

**2. Wire up the delivery backend (Axum)**

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
    },
});
let ctx = delivery_context_fn(delivery);
// provide ctx to both handle_server_fns_with_context and leptos_routes_with_context
```

See [`examples/axum-with-security`](./examples/axum-with-security) for production-ready
wiring (rate limiting + CSRF + Origin validation).  For a minimal local-dev skeleton,
see [`examples/axum-basic`](./examples/axum-basic) (**not production safe**).

**3. Place the component**

```rust,ignore
use leptos_hl_contact::ContactForm;

view! { <ContactForm /> }
```

---

## Feature flags

| Flag | What it enables |
|------|----------------|
| `hydrate` | Client-side hydration |
| `ssr` | Server-side rendering + server functions |
| `islands` | Leptos Islands architecture |
| `smtp-lettre` | SMTP delivery via `lettre` (implies `ssr`) |
| `axum-helpers` | `delivery_context_fn` helper (implies `ssr`) |
| `csrf`         | HMAC-SHA256 CSRF token generation + verification (implies `ssr`) |

---

## Design notes

- **Secure by default** — SMTP credentials never reach the client or WASM.
- **Progressive enhancement** — `<ActionForm/>` works without JavaScript.
- **Pluggable delivery** — implement `ContactDelivery` for any backend.
- **Accessible by default** — labels, ARIA, per-field errors, keyboard nav.
- **CSRF protection built-in** — stateless HMAC-SHA256 tokens, no session required (`csrf` feature).
- **Framework-neutral core** — Axum helpers are opt-in; the delivery trait is
  not tied to any HTTP framework.

> **Security notice:** Rate limiting and CSRF protection must be configured in
> your application layer. See the [Security guide](./docs/src/security.md).

---

## Full documentation

For guides, API reference, and architecture notes, see the
[**full documentation**](https://docs.rs/leptos-hl-contact).

Key chapters:
- [Quick Start](./docs/src/quick-start.md) — step-by-step tutorial
- [CSRF Protection](./docs/src/csrf.md) — stateless HMAC token setup
- [Security](./docs/src/security.md) — CSRF, rate limiting, deployment checklist
- [Delivery Backends](./docs/src/delivery-backends.md) — custom backends
- [Axum Integration](./docs/src/axum-integration.md) — context injection patterns
- [Styling](./docs/src/styling.md) — class injection and i18n
- [API Reference](./docs/src/api-reference.md) — complete type reference
