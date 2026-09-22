# Customization

`ContactForm` takes optional props, and `ContactServerPolicy` is provided
through Leptos context on the server.  The types this page covers:

| Type | Controls | Where |
|------|----------|-------|
| `ContactFormClasses` | CSS classes on each element | component prop |
| `ContactFormLabels` | Every visible string | component prop |
| `ContactFormOptions` | Which fields show, UI limits | component prop |
| `SiteFields` | Up to four fields of your own | component prop **and** `ContactServerPolicy` |
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
    honeypot: "contact-honeypot".into(), // the honeypot's wrapper
};
```

`honeypot` is needed only when `honeypot_inline_style` is `false`; see
[Styling → Honeypot](./styling.md#honeypot).  Framework examples are in
[Styling](./styling.md).

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
    token_refresh_secs: None,  // Some(ttl_secs - 60) when the server issues form tokens
    honeypot_inline_style: true, // false under a CSP without 'unsafe-inline'
    native_validation: true,   // false to suppress the browser's own prompting
};
```

| Field | Default | Effect |
|-------|---------|--------|
| `show_subject` | `true` | Render the subject field |
| `require_subject` | `false` | Mark the subject required in the UI |
| `max_message_len` | `4000` | `maxlength` on the textarea |
| `focus_first_error` | `true` | After a failed submission, move keyboard focus to the first invalid input.  Client-side only; set `false` if your page manages focus itself |
| `honeypot_inline_style` | `true` | Put the inline style that hides the honeypot on its wrapper.  Set `false` under a Content Security Policy without `'unsafe-inline'`, and hide the wrapper through `ContactFormClasses::honeypot` with your own CSS, or the field is visible.  [Details](./styling.md#honeypot) |
| `token_refresh_secs` | `None` | When `Some`, the browser fetches a form token for a form reached by client-side navigation and refreshes it before expiry.  Set it to `ttl_secs - 60` whenever the server issues form tokens; without it such a form submits an empty token and shows the token-invalid message.  [Details](../security/form-token.md#tokens-in-the-browser-acquisition-and-refresh) |
| `native_validation` | `true` | Whether the browser's own validation prompting is used.  Set `false` when every message a visitor reads must come from your own labels: the crate then renders `novalidate` on the `<form>`, which suppresses only that prompting — `required`, `type="email"`, `maxlength` and `aria-required` stay on every field, and the crate's own error wiring is unchanged.  The trade is language control against one extra round trip before the visitor is told a required field is empty, since the browser no longer blocks submission client-side.  [Details](./localization.md#the-browsers-own-prompting) |

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

Every successful outcome lands there, not only a delivered message: a
submission caught by the honeypot, or silently dropped by a
[filter](../security/filter.md), gets the same redirect, so the sender cannot
tell from the response that it was caught.

## ContactServerPolicy

Server-side enforcement, independent of what the client claims.  Provide it
through the context closure (see [Axum Integration](./axum-integration.md)).

```rust,ignore
use leptos_hl_contact::ContactServerPolicy;

leptos::context::provide_context(ContactServerPolicy {
    require_subject: true,   // reject a missing or blank subject
    max_message_len: 2000,   // reject longer messages; clamped to 4 000
    ..Default::default()     // site_fields: none
});
```

| Field | Default | Effect |
|-------|---------|--------|
| `require_subject` | `false` | Reject submissions without a subject |
| `max_message_len` | `4000` | Reject messages longer than this, in characters; clamped to 4 000 |
| `site_fields` | none | The [site-defined fields](#site-defined-fields) the server accepts |

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

## Site-defined fields

A site can add a few short fields of its own — an organisation, a topic, a
preferred time — between the subject and the message.  This is **not a form
builder**: the bounds below are fixed, and the server accepts exactly what you
define and nothing else.

```rust,ignore
use leptos_hl_contact::{
    ContactForm, ContactServerPolicy, SiteField, SiteFieldChoice, SiteFieldKind, SiteFields,
};

// Build the definition once, in code both sides can reach.
pub fn site_fields() -> SiteFields {
    SiteFields::new(vec![
        SiteField {
            key: "organisation".into(),
            label: "Organisation".into(),
            kind: SiteFieldKind::Line,          // one line
            required: true,
            max_len: 120,
        },
        SiteField {
            key: "topic".into(),
            label: "Topic".into(),
            kind: SiteFieldKind::Choice(vec![   // one of a fixed list
                SiteFieldChoice { key: "sales".into(), label: "Sales".into() },
                SiteFieldChoice { key: "support".into(), label: "Support".into() },
            ]),
            required: true,
            max_len: 0,                         // ignored for a choice
        },
        SiteField {
            key: "timing".into(),
            label: "Preferred timing".into(),
            kind: SiteFieldKind::Text,          // several lines
            required: false,
            max_len: 500,
        },
    ])
    .expect("the site's own definition is valid")
}

// The page:
view! { <ContactForm site_fields=site_fields() /> }

// The server, in the context closure:
leptos::context::provide_context(ContactServerPolicy {
    site_fields: site_fields(),
    ..Default::default()
});
```

**Pass the one definition to both.**  The form renders from it and the server
validates against it.  If they differ the server fails closed, and loudly:

- **A key the definition lacks** — or more than four keys — refuses the whole
  submission, as the `rejected` message.  Nothing is delivered, and the log
  carries only how many keys were refused, never which.
- **A field the server requires that the form did not render** is reported as
  `required`, and the form, which has no row to show it in, shows its generic
  error message (`labels.error`) instead of dropping it.
- **A malformed request** — a key sent twice, or nested (`fields[a][b]`) —
  fails while the request is decoded, before this crate's code runs.  The
  visitor sees the generic message, and the crate cannot log it.

### The bounds

These are **requirements, not defaults**.  `SiteFields::new` refuses a
definition outside them, naming the rule and the field's key (never its
label), and widening any of them needs a change to the crate.

| Bound | Value |
|-------|-------|
| Kinds | `Line` (one line), `Text` (several lines), `Choice` (one of a fixed list).  No checkbox, radio group, number, date, file or hidden field |
| Count | at most **4** fields |
| Placement | one fixed place: after the subject, before the message, in the order you define them |
| Logic | none: no conditional fields, no cross-field rules, no custom validators |
| Keys | `[a-z][a-z0-9_]{0,31}`, unique, and not a name the form already uses: `name`, `email`, `subject`, `message`, `website`, `form_token`, `fields`, `cf_turnstile_response`, `h_captcha_response`, `g_recaptcha_response` |
| Lengths | a `Line` at most 200 characters; a `Text` at most `MESSAGE_MAX_LEN` (4 000); a `Choice` has 2 to 20 options, whose keys follow the key rule and whose labels are not empty |
| Layout | none beyond [`ContactFormClasses`](#contactformclasses): each field is one row with the existing field, label, input, textarea and error classes |

### What the visitor and the server see

- **Controls.**  A `Line` is `<input type="text">`, a `Text` a `<textarea>`, a
  `Choice` a native `<select>` whose first option is an empty "—".  A
  required choice cannot be submitted on that option.  Keep a choice label
  short enough to read inside a phone's select box: a native select shows
  only what fits, the label is what a visitor sees, and the key is what
  delivery records.
- **Names and ids.**  The control is named `fields[key]` and has the id
  `contact-field-{key}`; its error paragraph is `contact-field-{key}-error`.
- **Trimmed first.**  A value is trimmed before it is checked, so a blank
  optional field counts as not answered, and a trailing newline in a `Line` is
  trimmed rather than refused — as `name` behaves.  Only a line break inside
  a `Line` is an error.
- **Errors** use the codes and labels the built-in fields use: `required`,
  `length` (naming the limit), `line_breaks` for a line break in a `Line`, and
  `format` for a choice that is not one of the listed keys.  See
  [Localization](./localization.md#site-defined-fields).
- **With JavaScript** what the visitor typed or chose stays after a failed
  submission, as for the built-in fields.  Without JavaScript nothing is kept,
  for any field.
- **Delivery** receives the answered fields, in your order, in
  `ContactInput::site_fields`; see
  [Delivery Backends](./delivery-backends.md#site-defined-fields-in-contactinput).
- **A site that defines none** renders exactly the markup, and sends exactly
  the requests, it did before this feature existed.
