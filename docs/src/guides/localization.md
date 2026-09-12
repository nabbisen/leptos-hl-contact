# Localization

## Component strings

Every string the component renders comes from
[`ContactFormLabels`](./customization.md#contactformlabels).  Pass a
translated set per instance:

```rust,ignore
use leptos_hl_contact::ContactFormLabels;

let labels = ContactFormLabels {
    name:    "お名前".into(),
    email:   "メールアドレス".into(),
    subject: "件名".into(),
    message: "お問い合わせ内容".into(),
    submit:  "送信する".into(),
    sending: "送信中…".into(),
    success: "送信完了しました。折り返しご連絡いたします。".into(),
    error:   "送信できませんでした。しばらくしてからお試しください。".into(),
    honeypot_label: "このフィールドは空欄にしてください".into(),
};
```

Language, text direction, and locale belong to the page: set `lang` and
`dir` on your document or on a wrapper element.  The component never sets
them.

## What is not localisable yet

Messages composed on the server are English and cannot yet be overridden
through labels:

- Per-field validation messages ("Name must be 1–80 characters", …)
- Server-policy messages ("Subject is required.", …)
- The token failure message ("Invalid or expired security token…")

The roadmap item P-14 replaces these with error codes that the component
maps to label entries, which makes the whole form localisable.  Until then,
the generic `labels.error` text is what visitors see for non-field errors.

## Unicode input

Names, subjects, and messages may contain any Unicode text.  Length limits
are counted in characters, not bytes, by the validator (see the note on
server policy in [Customization](./customization.md#contactserverpolicy)).
Non-ASCII display names and subjects are encoded correctly in email headers
by the SMTP backend.

## Presets

A set of ready-made label presets for common languages is planned (roadmap
P-20).  Until then, keep your translations in your own crate and pass them
as shown above.
