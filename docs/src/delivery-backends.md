# Delivery Backends

`leptos-hl-contact` separates message delivery from the form UI via the `ContactDelivery` trait.  You can use the built-in SMTP backend, swap in a no-op stub for testing, or implement your own.

## Built-in backends

### NoopDelivery

Discards every submission.  Suitable for local development, tests, and CI.

```rust
use std::sync::Arc;
use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};

let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

No feature flag required.

### LettreSmtpDelivery

Sends via SMTP using the [`lettre`](https://docs.rs/lettre) crate.  Requires the `smtp-lettre` feature.

```rust,ignore
use leptos_hl_contact::delivery::smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode};

let delivery = LettreSmtpDelivery {
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
};
```

#### TLS modes

| Variant | Port | Notes |
|---------|------|-------|
| `StartTls` | 587 | Recommended for most providers |
| `Tls` | 465 | Implicit TLS |
| `None` | any | **Development only** — plaintext |

## Implementing a custom backend

Implement the `ContactDelivery` trait:

```rust
use leptos_hl_contact::{
    delivery::ContactDelivery,
    error::ContactDeliveryError,
    model::ContactInput,
};

pub struct MyCustomDelivery;

impl ContactDelivery for MyCustomDelivery {
    async fn deliver(&self, input: ContactInput) -> Result<(), ContactDeliveryError> {
        // Send to SendGrid, write to database, post to Slack …
        todo!()
    }
}
```

Then provide it as the context:

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::delivery::ContactDeliveryContext;

let delivery: ContactDeliveryContext = Arc::new(MyCustomDelivery);
```

## Email headers

The built-in SMTP backend sets headers as follows:

| Header | Value |
|--------|-------|
| `From` | Server-configured `from_address` |
| `To` | Server-configured `to_address` |
| `Reply-To` | User-supplied email address |
| `Subject` | `subject_prefix` + sanitised subject |
| `Body` | Plain text |

User input is **never** used for `From`.  This avoids SPF/DKIM failures and email header injection.
