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
    },
};
```

With those labels no English reaches the visitor on any path, including a
submission without JavaScript.

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
