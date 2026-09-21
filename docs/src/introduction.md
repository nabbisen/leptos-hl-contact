# Introduction

`leptos-hl-contact` is a reusable, secure contact form for
[Leptos](https://leptos.dev) v0.8.

## What it provides

Three cooperating layers, shipped together:

| Layer | What it does |
|-------|-------------|
| **`ContactForm`** | Accessible HTML form with class, label, and option injection |
| **`submit_contact`** | Server function: token check → normalise → honeypot → validate → policy → challenge → deliver |
| **`ContactDelivery`** | Delivery trait with SMTP and no-op implementations; implement it for anything else |

### Key properties

- **Progressive enhancement.** The form is an `<ActionForm/>`, so it submits
  as a plain HTML POST when JavaScript or WebAssembly is unavailable.
- **Server-side validation** on every submission, whatever the client sent.
- **Honeypot** bot filtering with no visible CAPTCHA.
- **Optional challenge.**  Turnstile, hCaptcha or reCAPTCHA, rendered by the
  component and verified by the server before delivery; fail-closed.  See
  [Challenge](./security/challenge.md).
- **Email header injection protection.** `name` and `subject` are rejected
  if they contain line breaks, and sanitised again when the message is built.
- **Credential isolation.** SMTP credentials, the recipient address, and the
  token secret exist only in server-side types and are never compiled into
  WASM.
- **Form token** (`form-token` feature): a stateless HMAC-signed token
  that proves the sender fetched the page recently.  See
  [Security](./security/README.md) for exactly what it does and does not
  protect against.
- **Accessible by default.** Labels, `aria-required`, `aria-invalid`,
  `aria-describedby`, live regions.
- **Pluggable delivery.** One trait; SendGrid, SES, a database, or a queue
  can be added without touching the UI or the server function.

## What it does not include

- Submission storage or an admin panel
- Attachments or file upload
- A form builder (a few bounded site-defined fields are supported; see
  [Customization](./guides/customization.md#site-defined-fields))

These are out of scope to keep the crate small and its security surface
minimal.

## Supported environments

- Leptos v0.8, SSR and Islands
- Rust 1.88 or later
- Axum 0.8 through the optional `axum-helpers` feature; other backends work
  through the delivery trait and plain Leptos context

## Where to go next

| You want to… | Read |
|--------------|------|
| Get a form running | [Quick Start](./getting-started/quick-start.md) |
| Go live safely | [Production Checklist](./getting-started/production-checklist.md) |
| Change text, style, or fields | [Customization](./guides/customization.md) |
| Understand the security model | [Security](./security/README.md) |
| Look up a type or function | [API](./reference/api.md) |
| Contribute | [Architecture](./development/architecture.md) |
