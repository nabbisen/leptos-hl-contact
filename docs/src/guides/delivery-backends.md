# Delivery Backends

Delivery is separated from the form through the `ContactDelivery` trait.
The server function hands every validated submission to whatever
implementation you registered as `ContactDeliveryContext`.

## NoopDelivery

Discards every submission and logs one line at `debug`.  No feature flag.
Use it for local development, tests, and CI.

```rust
use std::sync::Arc;
use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};

let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

## LettreSmtpDelivery

Sends through an SMTP relay using [`lettre`](https://docs.rs/lettre).
Requires the `smtp-lettre` feature.

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

### SmtpConfig

| Field | Meaning |
|-------|---------|
| `host`, `port` | Relay address |
| `username`, `password` | Relay credentials; `password` is redacted in `Debug` output |
| `from_address` | `From` header; must be authorised to send through the relay |
| `to_address` | Where enquiries are delivered |
| `subject_prefix` | Prepended to every subject, e.g. `[Contact]` |
| `tls_mode` | See below |

### TLS modes

| Variant | Typical port | Use |
|---------|--------------|-----|
| `StartTls` (default) | 587 | Most providers |
| `Tls` | 465 | Implicit TLS |
| `DangerousPlaintext` | 1025 | Local development only; credentials and content are sent in clear |

### The email that is sent

| Header | Value |
|--------|-------|
| `From` | `from_address` |
| `To` | `to_address` |
| `Reply-To` | Visitor's name and address, RFC 5322-encoded |
| `Subject` | `subject_prefix` + subject (or "(no subject)"), line breaks removed |
| Body | Plain text UTF-8: name, email, subject, message |

Visitor input is never used for `From`, which keeps SPF and DKIM alignment
intact and prevents sender spoofing.

## Writing your own backend

Implement the trait.  It returns a boxed future so it can be used as
`Arc<dyn ContactDelivery>`.

```rust
use std::{future::Future, pin::Pin};
use leptos_hl_contact::{
    delivery::ContactDelivery,
    error::ContactDeliveryError,
    model::ContactInput,
};

pub struct MyCustomDelivery;

impl ContactDelivery for MyCustomDelivery {
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        Box::pin(async move {
            // Call SendGrid, write to a database, post to Slack …
            let _ = input;
            Ok(())
        })
    }
}
```

Register it exactly like the built-in ones:

```rust,ignore
let delivery: ContactDeliveryContext = Arc::new(MyCustomDelivery);
```

Contract for implementations:

- `input` is already trimmed, validated, and honeypot-checked.
- Return one of `ContactDeliveryError::{Configuration, Transport,
  MessageBuild, Internal}`; the crate logs the detail and shows the visitor
  a generic message.
- Do not log the visitor's name, email, or message.
- The call is not time-limited by the crate.  Wrap slow APIs in a timeout.

## Testing delivery locally

Run [MailHog](https://github.com/mailhog/MailHog) and point the SMTP backend
at it:

```bash
docker run -p 1025:1025 -p 8025:8025 mailhog/mailhog
```

```rust,ignore
SmtpConfig {
    host:     "localhost".into(),
    port:     1025,
    username: String::new(),
    password: String::new(),
    tls_mode: SmtpTlsMode::DangerousPlaintext,
    // …
}
```

Open <http://localhost:8025> to see delivered messages.
