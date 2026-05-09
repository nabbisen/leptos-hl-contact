# Configuration

## ContactFormClasses

Override CSS classes for every structural element:

```rust
use leptos_hl_contact::ContactFormClasses;

let classes = ContactFormClasses {
    root:     "contact-root".into(),
    field:    "contact-field".into(),
    label:    "contact-label".into(),
    input:    "contact-input".into(),
    textarea: "contact-textarea".into(),
    button:   "contact-button".into(),
    error:    "contact-error".into(),   // used for both field and generic errors
    success:  "contact-success".into(),
};
```

All fields default to `""`.

## ContactFormLabels

Override any user-visible string:

```rust
use leptos_hl_contact::ContactFormLabels;

let labels = ContactFormLabels {
    name:          "Your name".into(),
    email:         "Your email".into(),
    subject:       "Subject".into(),
    message:       "Message".into(),
    submit:        "Send".into(),
    sending:       "Sending…".into(),
    success:       "Your message has been sent.".into(),
    error:         "Failed to send. Please try again later.".into(),
    honeypot_label:"Leave this field blank".into(),  // screen-reader text only
};
```

## ContactFormOptions

```rust
use leptos_hl_contact::ContactFormOptions;

let options = ContactFormOptions {
    show_subject:    true,   // show the subject field
    require_subject: false,  // make subject required (no effect if show_subject=false)
    max_message_len: 4000,   // must not exceed 4000
};
```

## ContactServerPolicy

**Server-side** enforcement of constraints, independent of `ContactFormOptions`.

> **Important distinction:**
> `ContactFormOptions` controls the *UI* (what is shown, required, or capped
> client-side).  Because the server function is a public HTTP endpoint that any
> HTTP client can call directly, `ContactFormOptions` is **not a security boundary**.
> `ContactServerPolicy` is the server-authoritative source of truth.

```rust,ignore
use leptos_hl_contact::ContactServerPolicy;

// Provide in both SSR renderer and server-fn handler closures:
leptos::context::provide_context(ContactServerPolicy {
    require_subject: true,   // enforced regardless of ContactFormOptions
    max_message_len: 2000,   // overrides the UI maxlength attribute
});
```

If `require_subject = true` is set only in `ContactFormOptions` but not in
`ContactServerPolicy`, a direct POST request can bypass the requirement.

### Fields

| Field | Default | Effect |
|-------|---------|--------|
| `require_subject` | `false` | Reject submissions with an absent or blank subject |
| `max_message_len` | `4000` | Reject messages longer than this (must be ≤ 4000) |


## SmtpConfig

All fields are server-side only.  Load credentials from environment variables.

```rust,ignore
use leptos_hl_contact::delivery::smtp::{SmtpConfig, SmtpTlsMode};

let config = SmtpConfig {
    host:           std::env::var("SMTP_HOST")?,
    port:           587,
    username:       std::env::var("SMTP_USER")?,
    password:       std::env::var("SMTP_PASS")?,   // never hard-code
    from_address:   std::env::var("SMTP_FROM")?,
    to_address:     std::env::var("CONTACT_TO")?,
    subject_prefix: "[Contact]".into(),
    tls_mode:       SmtpTlsMode::StartTls,
};
```

### TLS modes

| Variant | Default port | Notes |
|---------|-------------|-------|
| `StartTls` | 587 | Recommended for most providers |
| `Tls` | 465 | Implicit TLS |
| `DangerousPlaintext` | any | **Development only** — plaintext, never use in production |
