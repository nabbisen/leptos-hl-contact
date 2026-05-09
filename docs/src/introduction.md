# Introduction

`leptos-hl-contact` is a reusable, secure contact form plugin for
[Leptos](https://leptos.dev) v0.8.

## What it provides

### A complete three-layer solution

Rather than shipping a bare UI component, `leptos-hl-contact` bundles three
cooperating layers:

| Layer | What it does |
|-------|-------------|
| **`ContactForm`** | Accessible HTML form with class/label/option injection |
| **`submit_contact`** | Server-only function: normalise → honeypot → validate → deliver |
| **`ContactDelivery`** | Abstract trait; ships SMTP and no-op implementations |

### Key features

**Progressive enhancement** — `<ActionForm/>` means the form submits as a
standard HTML POST even when JavaScript or WebAssembly is unavailable.

**Server-side validation** — Every submission is re-validated on the server
regardless of what the client sends.  Per-field error messages are returned
and displayed next to the relevant input.

**Honeypot bot protection** — A hidden `website` field silently filters most
automated submissions without requiring a visible CAPTCHA.

**Email header injection protection** — The `name` and `subject` fields are
validated to contain no newline characters before being used in email headers.

**Secure credential handling** — SMTP passwords and recipient addresses live
only in server-side configuration.  They are never compiled into WASM or
returned to the client.

**Accessible by default** — Every input has a `<label>`, required fields carry
`aria-required`, validation errors carry `aria-invalid` and `aria-describedby`,
and status messages use `role="status"` / `role="alert"`.

**Pluggable delivery** — `ContactDelivery` is a plain async trait.  Add
SendGrid, AWS SES, a database writer, or any other backend without touching
the UI or server function.

## What it does not include

- CAPTCHA (Turnstile integration is documented in the [Security guide](./security.md))
- Submission history or admin panel
- Attachment / file upload
- GUI form builder

These are intentionally out of scope to keep the crate small and the
security surface minimal.

## Supported environments

- Leptos v0.8 SSR applications
- Leptos v0.8 Islands applications
- Axum backend (other backends work with the delivery trait; Axum helpers are optional)

## Next steps

→ [Quick Start](./quick-start.md) — get a form running in minutes  
→ [Security](./security.md) — what you must configure before going public
