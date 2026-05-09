# Frequently Asked Questions

## General

### Does this work without JavaScript?

Yes.  The form uses Leptos `<ActionForm/>`, which submits as a standard HTML
`POST` when WebAssembly is unavailable.  Server-side validation and delivery
work identically in both modes.

### Which Leptos versions are supported?

`leptos-hl-contact` targets **Leptos v0.8**.  Earlier versions are not
supported because the crate uses `#[server]` and `ServerAction` as they exist
in v0.8.

### Can I use this with Actix-Web instead of Axum?

The core crate (`leptos-hl-contact` without the `axum-helpers` feature) is not
tied to any HTTP framework.  You are responsible for providing the
`ContactDeliveryContext` to both your server-function handler and your SSR
renderer, but the mechanism is the same: use your framework's equivalent of
Leptos `provide_context`.

The `axum-helpers` feature is purely additive and optional.

---

## Delivery

### How do I test locally without a real SMTP server?

Use `NoopDelivery`, which discards every submission and logs it at `DEBUG`:

```rust,ignore
use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};
let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
```

For realistic end-to-end testing, run [MailHog](https://github.com/mailhog/MailHog)
locally and point `SmtpConfig` at `localhost:1025` with `SmtpTlsMode::None`.
See [Testing](./testing.md) for details.

### How do I send via SendGrid / SES / Resend instead of SMTP?

Implement the `ContactDelivery` trait:

```rust,ignore
impl ContactDelivery for MyBackend {
    async fn deliver(&self, input: ContactInput) -> Result<(), ContactDeliveryError> {
        // call your API here
        Ok(())
    }
}
```

See [Delivery Backends](./delivery-backends.md) for a complete example.

### Where is the recipient email address configured?

In `SmtpConfig::to_address` on the **server side only**.  This value is never
serialised to WASM or sent to the browser.

---

## Security

### Is the honeypot field enough to stop all bots?

No.  The honeypot filters simple bots that fill every visible field.
For production deployments, add **rate limiting** at the HTTP layer and
consider **Cloudflare Turnstile** for high-value forms.  See
[Security](./security.md) for a deployment checklist.

### Does the crate handle CSRF?

Not directly.  The server function is a public HTTP endpoint.  Protect it in
your application layer with `SameSite=Lax` cookies and, if necessary, an
`Origin` / `Referer` check.  See [Security](./security.md) for a middleware
snippet.

### Can an attacker inject headers into the email?

No.  The `name` and `subject` fields are validated on the server to contain no
`\r` or `\n` characters before being placed in email headers.  User input is
**never** used in the `From` header — only the server-configured address is.

### Are SMTP credentials safe?

Yes, as long as you load them from environment variables (not source code).
They live in `SmtpConfig`, which exists only in your server binary.  Nothing
from `SmtpConfig` is compiled into WASM or returned to the client.

---

## Customisation

### How do I change the form labels to another language?

Override any string via `ContactFormLabels`:

```rust,ignore
ContactFormLabels {
    name:    "お名前".into(),
    email:   "メールアドレス".into(),
    submit:  "送信する".into(),
    sending: "送信中…".into(),
    success: "送信完了しました。".into(),
    error:   "送信できませんでした。".into(),
    ..Default::default()
}
```

### How do I apply Tailwind CSS classes?

Pass them via `ContactFormClasses`.  All fields default to `""`:

```rust,ignore
ContactFormClasses {
    root:   "max-w-lg mx-auto space-y-4".into(),
    button: "bg-blue-600 text-white px-6 py-2 rounded disabled:opacity-50".into(),
    error:  "text-red-600 text-sm".into(),
    ..Default::default()
}
```

See [Styling](./styling.md) for a full Tailwind example.

### Can I hide the subject field?

Yes:

```rust,ignore
ContactFormOptions { show_subject: false, ..Default::default() }
```

### Can I make the subject field required?

```rust,ignore
ContactFormOptions { show_subject: true, require_subject: true, ..Default::default() }
```

---

## Errors

### The form shows a generic error message. How do I see the real error?

Detailed errors are intentionally kept server-side.  Check your server logs
(tracing output).  The log includes the error category and, for SMTP failures,
the transport error.

### I get "Contact form is not configured." What does that mean?

The `ContactDeliveryContext` was not provided to the server function handler.
Check that you are calling `provide_context` (or `delivery_context_fn`) in
**both** the `handle_server_fns_with_context` closure and the
`leptos_routes_with_context` closure.  See [Axum Integration](./axum-integration.md).
