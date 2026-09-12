# Customization

`ContactForm` takes three optional props.  A fourth type,
`ContactServerPolicy`, is provided through Leptos context on the server.

| Type | Controls | Where |
|------|----------|-------|
| `ContactFormClasses` | CSS classes on each element | component prop |
| `ContactFormLabels` | Every visible string | component prop |
| `ContactFormOptions` | Which fields show, UI limits | component prop |
| `ContactServerPolicy` | Server-enforced limits | Leptos context |

## ContactFormClasses

One class string per structural element.  All default to `""`.

```rust
use leptos_hl_contact::ContactFormClasses;

let classes = ContactFormClasses {
    root:     "contact-root".into(),
    field:    "contact-field".into(),
    label:    "contact-label".into(),
    input:    "contact-input".into(),
    textarea: "contact-textarea".into(),
    button:   "contact-button".into(),
    error:    "contact-error".into(),    // per-field and generic errors
    success:  "contact-success".into(),
};
```

Framework examples are in [Styling](./styling.md).

## ContactFormLabels

Every string the component renders.  English defaults.

```rust
use leptos_hl_contact::ContactFormLabels;

let labels = ContactFormLabels {
    submit:  "Send enquiry".into(),
    success: "Thank you — we will be in touch shortly.".into(),
    ..Default::default()
};
```

| Field | Default |
|-------|---------|
| `name` | Name |
| `email` | Email |
| `subject` | Subject |
| `message` | Message |
| `submit` | Send |
| `sending` | Sending… |
| `success` | Your message has been sent. We will get back to you soon. |
| `error` | Failed to send message. Please try again later. |
| `honeypot_label` | Leave this field blank *(screen-reader text only)* |

Translating the form is covered in [Localization](./localization.md).

## ContactFormOptions

```rust
use leptos_hl_contact::ContactFormOptions;

let options = ContactFormOptions {
    show_subject:    true,   // render the subject field
    require_subject: false,  // mark it required in the UI (no effect if hidden)
    max_message_len: 4000,   // textarea maxlength; must not exceed 4000
};
```

These control the browser only.  Anyone can POST to the server function
directly, so options are **not** a security boundary.

## ContactServerPolicy

Server-side enforcement, independent of what the client claims.  Provide it
through Leptos context in both the server-function handler and the SSR
renderer (see [Axum Integration](./axum-integration.md)).

```rust,ignore
use leptos_hl_contact::ContactServerPolicy;

leptos::context::provide_context(ContactServerPolicy {
    require_subject: true,   // reject a missing or blank subject
    max_message_len: 2000,   // reject longer messages; must not exceed 4000
});
```

| Field | Default | Effect |
|-------|---------|--------|
| `require_subject` | `false` | Reject submissions without a subject |
| `max_message_len` | `4000` | Reject messages longer than this |

Rule of thumb: options for experience, policy for enforcement.  Setting
`require_subject` in options alone lets a direct POST skip the subject; set
it in both to get an immediate UI hint **and** a server-side guarantee.

> In 0.3.3 the policy compares message length in bytes while the validator
> counts characters.  Non-ASCII messages can therefore hit the policy limit
> earlier than the textarea limit suggests.  Tracked in the roadmap (P-04).
