# Delivery Backends

Delivery is separated from the form through the `ContactDelivery` trait.
The server function hands every validated submission to whatever
implementation you registered as `ContactDeliveryContext`.

**Which backend to use:**

| Backend | Runs on | Needs |
|---------|---------|-------|
| `LettreSmtpDelivery` | native only | an SMTP relay you run or rent |
| `ResendDelivery` | native **and** Cloudflare Workers | an API key at [Resend](https://resend.com) |
| Your own | any target your code targets | whatever your backend needs |

On a Worker, SMTP cannot run at all (no `tokio` runtime); `ResendDelivery` is
the built-in option there — see
[Cloudflare Workers](./cloudflare-workers.md#delivery-on-a-worker).

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
        timeout:        SmtpConfig::DEFAULT_TIMEOUT,
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
| `timeout` | Deadline for one delivery, from connecting to the relay's final reply.  Use `SmtpConfig::DEFAULT_TIMEOUT` (30 s) unless the relay is known to be slow; it must be greater than zero.  When it passes the visitor sees `delivery_timeout`, which says the message may have been sent |

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
| Body | Plain text UTF-8: name, email, subject, then one block per [site-defined field](./customization.md#site-defined-fields), then message |

Visitor input is never used for `From`, which keeps SPF and DKIM alignment
intact and prevents sender spoofing.  A site field's value appears in the body
only, never in a header.

A site field's block sits after `Subject:` and before `Message:`, in the style
of the others; a choice reads `choice label (choice key)`:

```text
Organisation:
Example Ltd

Topic:
Sales (sales)

Message:
…
```

A site that defines no fields sends the same body as before.

## ResendDelivery

Sends through [Resend](https://resend.com)'s HTTP API.  Requires the
`delivery-resend` feature.  Unlike SMTP, it needs no `tokio` runtime, so it
also runs on a Cloudflare Workers server.

```rust,ignore
use std::sync::Arc;
use leptos_hl_contact::delivery::{
    ContactDeliveryContext,
    resend::{ResendConfig, ResendDelivery},
};

let delivery: ContactDeliveryContext = Arc::new(ResendDelivery::new(
    ResendConfig::new(
        std::env::var("RESEND_API_KEY")?,
        std::env::var("RESEND_FROM")?,
        std::env::var("CONTACT_TO")?,
    )
    .with_subject_prefix("[Contact]"),
));
```

`ResendConfig::new` takes only the three values it cannot work without —
the API key, the sender, and the recipient.  Everything else has a default
and is changed with a builder (`with_subject_prefix`, `with_timeout`), so a
field added to the config later needs no migration line, unlike
`SmtpConfig`'s plain fields.

| What | Where |
|------|-------|
| The API key | an environment variable — never source; see [Hardening](../security/hardening.md) |
| The sender (`from`) | a domain verified with Resend; an unverified one is rejected by Resend itself, not checked here |
| The recipient (`to`) | wherever enquiries should land |
| A missing value | `api_key`, `from_address` or `to_address` left empty is accepted at construction and reported as `Configuration` on the first delivery — naming which one, never a value — so a missing environment variable fails closed rather than at startup, which a Worker does not have |
| The deadline | `ResendConfig::DEFAULT_TIMEOUT` (10 s) bounds the one HTTP request this backend makes.  `DeliveryTimeout` still composes around it (an outer bound), but the adapter's own limit is what normally applies first |
| The body | the same plain text `LettreSmtpDelivery` sends, including the site's own fields |
| Errors | 401/403 → `Configuration`; 422 → `MessageBuild`; 429, 5xx, or anything else non-2xx → `Transport`.  The message carries the status code (`"HTTP 429"`) and nothing else — never the vendor's own error text |
| An overridden endpoint (`with_url`) | for a forwarding proxy or a test server, like `HttpChallengeVerifier::with_verify_url`; **must be `https`** outside local testing — the crate warns, once, when it is not (see [Hardening](../security/hardening.md)) |

On success the provider's message id is logged at `info`, for tracing an
enquiry the recipient says never arrived; nothing else from the answer is
read.

## Writing your own backend

Implement the trait.  It returns a boxed future, `DeliveryFuture`, so it can
be used as `Arc<dyn ContactDelivery>`.  The alias is `Send` natively and
drops `Send` on a wasm32 server build (Cloudflare Workers), so the same code
compiles for both.

```rust
use leptos_hl_contact::{
    delivery::{ContactDelivery, DeliveryFuture},
    model::ContactInput,
};

pub struct MyCustomDelivery;

impl ContactDelivery for MyCustomDelivery {
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_> {
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
- `input.site_fields` holds the answers to the site's own fields; see
  [below](#site-defined-fields-in-contactinput).
- Return one of `ContactDeliveryError::{Configuration, Transport,
  MessageBuild, Internal}`; the crate logs the detail and shows the visitor
  a generic message.
- The text of a returned `ContactDeliveryError` is written to the server log
  for operators.  Put the category and the transport detail in it — status
  codes, the relay's reply.  Never put the submission in it, and never log it
  yourself: no name, email address, subject, message, token or credential.
- A delivery may be cancelled at any `.await` when it is wrapped in a
  timeout.  Do not leave shared state half-updated across an `.await`.  An
  HTTP API call cancelled mid-flight may still complete on the vendor's side.

The built-in SMTP backend already has a deadline (`SmtpConfig::timeout`).
Give your own backend the same bound by wrapping it (feature
`delivery-timeout`, which `smtp-lettre` turns on):

```rust,ignore
use std::time::Duration;
use leptos_hl_contact::DeliveryTimeout;

let delivery: ContactDeliveryContext =
    Arc::new(DeliveryTimeout::new(MyCustomDelivery, Duration::from_secs(30)));
```

When the deadline passes, the delivery is dropped and the visitor is told the
message may have been sent (`delivery_timeout`), not that it failed.

### Site-defined fields in `ContactInput`

`ContactInput::site_fields` is a `Vec<SiteFieldValue>`:

| Member | Holds |
|--------|-------|
| `key` | the field's key in the site's definition |
| `label` | the field's label, **from the server's definition**, never from the request |
| `value` | the trimmed value; for a choice, the choice's key |
| `value_label` | for a choice, the choice's label; `None` otherwise |

- **Order:** the order the site defined the fields in, not the order of the
  request.
- **Answered fields only:** a blank optional field is left out, so the list
  may be empty, and is empty for a site that defines none.
- **A filter** receives the same `ContactInput`, so it can read the values.

**Treat the values like `message`:** they are personal data.  Never log
them, and never put one in a `ContactDeliveryError`'s text.  A value that goes
into a header, a URL or a query needs the escaping that header, URL or query
needs; the SMTP backend puts them in the body only.

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
    timeout:  SmtpConfig::DEFAULT_TIMEOUT,
    // …
}
```

Open <http://localhost:8025> to see delivered messages.
