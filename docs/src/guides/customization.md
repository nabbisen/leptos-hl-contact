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
| `errors` | [`ContactErrorLabels`](./localization.md#error-messages) — text for every error the server reports |

Translating the form is covered in [Localization](./localization.md).

## ContactFormOptions

```rust
use leptos_hl_contact::ContactFormOptions;

let options = ContactFormOptions {
    show_subject:      true,   // render the subject field
    require_subject:   false,  // mark it required in the UI (no effect if hidden)
    max_message_len:   4000,   // textarea maxlength; clamped to 4 000
    focus_first_error: true,   // focus the first invalid input after a failure
};
```

| Field | Default | Effect |
|-------|---------|--------|
| `show_subject` | `true` | Render the subject field |
| `require_subject` | `false` | Mark the subject required in the UI |
| `max_message_len` | `4000` | `maxlength` on the textarea |
| `focus_first_error` | `true` | After a failed submission, move keyboard focus to the first invalid input.  Client-side only; set `false` if your page manages focus itself |

These control the browser only.  Anyone can POST to the server function
directly, so options are **not** a security boundary.

`max_message_len` is counted in characters, and a value above `MESSAGE_MAX_LEN`
(4 000) is clamped to it.  `ContactFormOptions::effective_max_message_len()`
returns the value the component actually renders as `maxlength`.

## Success page

By default a successful submission shows the inline success message with
JavaScript, and reloads the form page without it — so a no-JavaScript visitor
gets no confirmation at all.  Name a success page and both paths land there:

```rust,ignore
use leptos_hl_contact::axum_helpers::success_redirect;

// Build it before the router so a bad path panics at boot …
let redirect = success_redirect("/thanks");

// … then provide it in the context closure (see Axum Integration):
leptos::context::provide_context(redirect.clone());
```

| Configured? | With JavaScript | Without JavaScript |
|-------------|-----------------|--------------------|
| yes | navigates to the page | `302` to the page |
| no | inline success message | page reloads, no confirmation |

The path must be site-relative: it must start with a single `/`, and may not
contain a scheme, a backslash, whitespace or a control character.  Query
strings and fragments are allowed.  `success_redirect` panics on anything
else, because this is startup configuration; use
[`ContactSuccessRedirect::new`] if you would rather handle the error.  The
rule exists so a misconfiguration cannot turn the form into an open redirect,
and the destination is never read from form input or a query parameter.

Without `axum-helpers`, build the value yourself: `ContactSuccessRedirect::new`
takes the path and a closure that performs the redirect in your framework.

[`ContactSuccessRedirect::new`]: https://docs.rs/leptos-hl-contact/latest/leptos_hl_contact/config/struct.ContactSuccessRedirect.html

## ContactServerPolicy

Server-side enforcement, independent of what the client claims.  Provide it
through the context closure (see [Axum Integration](./axum-integration.md)).

```rust,ignore
use leptos_hl_contact::ContactServerPolicy;

leptos::context::provide_context(ContactServerPolicy {
    require_subject: true,   // reject a missing or blank subject
    max_message_len: 2000,   // reject longer messages; clamped to 4 000
});
```

| Field | Default | Effect |
|-------|---------|--------|
| `require_subject` | `false` | Reject submissions without a subject |
| `max_message_len` | `4000` | Reject messages longer than this, in characters; clamped to 4 000 |

Rule of thumb: options for experience, policy for enforcement.  Setting
`require_subject` in options alone lets a direct POST skip the subject; set
it in both to get an immediate UI hint **and** a server-side guarantee.

Lengths are counted in characters (Unicode scalar values) everywhere — the
textarea's `maxlength`, the validator, and the policy — so a 2 000-character
Japanese message is accepted by a limit of 2 000 regardless of how many bytes
it occupies.

The policy can only tighten the validator's limit.  `MESSAGE_MAX_LEN` (4 000)
is the hard ceiling: a larger `max_message_len` is clamped to it rather than
raising it.  `ContactServerPolicy::effective_max_message_len()` returns the
limit in force, and `ContactServerPolicy::check()` applies the whole policy to
a normalised input.
