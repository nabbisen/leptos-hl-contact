# Localization

The crate ships English defaults.  Every string a visitor can see is the
site's own to set, and nothing on this page — including a contributed
translation below — is a guarantee about a language none of us speak.

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

## The browser's own prompting

Even with every label translated, an empty required field still triggers
the browser's **own** validation message — "Please fill out this field.",
in the browser's language, not the page's — before your labels ever get a
chance.  Set
[`ContactFormOptions::native_validation`](./customization.md#contactformoptions)
to `false` and every message a visitor reads comes from your own labels: the
crate renders `novalidate` on the `<form>`, which suppresses only that
built-in prompting.  `required`, `aria-required` and the crate's own error
wiring are unchanged, so a screen reader and the server-side check both
still see exactly what they did before; the cost is one extra round trip
before the visitor is told a required field is empty, since the browser no
longer blocks submission itself.

## Error messages

The server sends codes, never sentences, so every message a visitor can see
comes from `labels.errors`:

```rust,ignore
use leptos_hl_contact::{ContactErrorLabels, ContactFormLabels};

let labels = ContactFormLabels {
    errors: ContactErrorLabels {
        required:        "この項目は必須です。".into(),
        length:          "{min}〜{max}文字で入力してください。".into(),
        format_email:    "メールアドレスの形式が正しくありません。".into(),
        format:          "入力内容が正しくありません。".into(),
        line_breaks:     "改行は使用できません。".into(),
        token_invalid:   "セッションの有効期限が切れました。ページを再読み込みしてください。".into(),
        too_fast:        "少し時間をおいてから、もう一度お試しください。".into(),
        not_configured:  "現在このフォームはご利用いただけません。".into(),
        delivery_failed: "送信できませんでした。しばらくしてからお試しください。".into(),
        delivery_timeout: "送信に時間がかかりすぎました。メッセージは送信された可能性があります。数分待ってから再度お試しください。".into(),
        challenge_required:    "セキュリティ確認を完了してください。".into(),
        challenge_failed:      "セキュリティ確認に失敗しました。もう一度お試しください。".into(),
        challenge_unavailable: "現在セキュリティ確認を利用できません。しばらくしてからお試しください。".into(),
        challenge_requires_js: "このフォームでは、人による操作であることの確認に JavaScript が必要です。".into(),
        rejected:              "お送りいただいたメッセージは受け付けられませんでした。".into(),
        email_domain:          "そのドメイン宛てのメールサーバーが見つかりませんでした。スペルをご確認ください。".into(),
    },
    ..Default::default()
};
```

| Field | Shown when |
|-------|------------|
| `required` | a required field was left empty, or the server policy's `require_subject` rejected a missing subject |
| `length` | the value is present but too short or too long; `{min}` and `{max}` are replaced with numbers |
| `format_email` | the email field is not a valid address |
| `format` | any other field is syntactically invalid |
| `line_breaks` | a line break appears in `name` or `subject` |
| `token_invalid` | the form token was missing, malformed or expired |
| `too_fast` | the form was submitted sooner after loading than the token's minimum age; retryable |
| `not_configured` | the server is missing a required context value |
| `delivery_failed` | the backend refused or failed, and for any unexpected error |
| `delivery_timeout` | delivery did not finish within its deadline; the message may have been sent, so the text asks the visitor to wait before retrying |
| `challenge_required` | a [challenge](../security/challenge.md) is configured and the submission carried no token |
| `challenge_failed` | the challenge vendor rejected the token, or its score or action did not satisfy the policy |
| `challenge_unavailable` | the challenge vendor could not be reached |
| `challenge_requires_js` | shown inside `<noscript>` next to the widget, under `NoJsPolicy::Reject` |
| `rejected` | a [filter](../security/filter.md) returned `Reject`; deliberately generic |
| `email_domain` | the [email domain check](../security/email-domain.md) found no mail route for the address's domain |

`{min}` and `{max}` are the only placeholders, and they are replaced by plain
string substitution — there is no format syntax, so translated text may put
them in any order or omit them.

`labels.error` remains as the last-resort banner for a payload the client
does not recognise, such as one from a newer server.

### A complete Japanese form

```rust,ignore
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
    errors: ContactErrorLabels {
        required:        "この項目は必須です。".into(),
        length:          "{min}〜{max}文字で入力してください。".into(),
        format_email:    "メールアドレスの形式が正しくありません。".into(),
        format:          "入力内容が正しくありません。".into(),
        line_breaks:     "改行は使用できません。".into(),
        token_invalid:   "セッションの有効期限が切れました。ページを再読み込みしてください。".into(),
        too_fast:        "少し時間をおいてから、もう一度お試しください。".into(),
        not_configured:  "現在このフォームはご利用いただけません。".into(),
        delivery_failed: "送信できませんでした。しばらくしてからお試しください。".into(),
        delivery_timeout: "送信に時間がかかりすぎました。メッセージは送信された可能性があります。数分待ってから再度お試しください。".into(),
        challenge_required:    "セキュリティ確認を完了してください。".into(),
        challenge_failed:      "セキュリティ確認に失敗しました。もう一度お試しください。".into(),
        challenge_unavailable: "現在セキュリティ確認を利用できません。しばらくしてからお試しください。".into(),
        challenge_requires_js: "このフォームでは、人による操作であることの確認に JavaScript が必要です。".into(),
        rejected:              "お送りいただいたメッセージは受け付けられませんでした。".into(),
        email_domain:          "そのドメイン宛てのメールサーバーが見つかりませんでした。スペルをご確認ください。".into(),
    },
};
```

With those labels no English reaches the visitor on any path, including a
submission without JavaScript.

## Site-defined fields

The labels of a [site-defined field](./customization.md#site-defined-fields)
and of each choice are written in the definition, in the language of your
page: the crate translates none of them.  Build the definition per language,
or from your own translation table, before passing it to `ContactForm` and
`ContactServerPolicy`.  The empty first option of a choice, "—", is a symbol,
not a word.

Errors reuse the labels above and add none:

| Case | Label |
|------|-------|
| Required and blank | `errors.required` |
| Over `max_len` | `errors.length`, with `{min}` and `{max}` |
| A line break in a `Line` | `errors.line_breaks` |
| A choice that is not listed | `errors.format` |
| A key the definition lacks, or too many keys | `errors.rejected`, for the whole form |
| An error for a field the form did not render | `error`, the generic message |

## Unicode input

Names, subjects, and messages may contain any Unicode text.  Length limits
are counted in characters, not bytes, by the validator (see the note on
server policy in [Customization](./customization.md#contactserverpolicy)).
Non-ASCII display names and subjects are encoded correctly in email headers
by the SMTP backend.

## Contributing a language

There is no built-in preset for any language beyond the English defaults;
the two blocks above are the whole of what the crate ships.  Anyone can add
a translation to this page for others to copy.

**What a contribution is.**  One complete set — every field of
[`ContactFormLabels`](./customization.md#contactformlabels) and
[`ContactErrorLabels`](#error-messages), no gaps — in the contributor's own
language, as a copyable Rust block on this page, in the same shape as
[the Japanese example above](#a-complete-japanese-form).

**Why complete.**  A partial set leaves a visitor reading two languages in
one form: the fields translated, the errors not, or the reverse. A
contribution stands or falls as a whole set.

**Why the page, and not the crate.**  The crate ships English defaults, and
every visible string is the site's own to set; nothing here carries a
version coupling to a crate release. A translation added to this page
needs no new crate version, and a new error code added to the crate cannot
silently leave a shipped language's preset half-translated, because there
is no shipped preset to leave behind.

**What we check.**  That the set is complete — every field of both structs
present — and that `{min}` and `{max}` are still intact wherever the
English original uses them. **We do not vouch for the wording.** The
contributor's language is theirs; a native speaker reviewing wording is
welcome, but nothing here is proofread against meaning or tone by anyone
who does not speak the language.
