# Styling

`leptos-hl-contact` ships without built-in CSS.  All visual styling is applied through class injection — you provide CSS class strings, the crate applies them to the right elements.

## ContactFormClasses

```rust
use leptos_hl_contact::ContactFormClasses;

let classes = ContactFormClasses {
    root:     "my-form".into(),
    field:    "form-field".into(),
    label:    "form-label".into(),
    input:    "form-input".into(),
    textarea: "form-textarea".into(),
    button:   "form-button".into(),
    error:    "form-error".into(),
    success:  "form-success".into(),
};
```

All fields default to `""` (no class applied) when using `ContactFormClasses::default()`.

## Tailwind CSS example

```rust
ContactFormClasses {
    root:     "max-w-lg mx-auto space-y-4".into(),
    field:    "flex flex-col gap-1".into(),
    label:    "text-sm font-medium text-gray-700".into(),
    input:    "border border-gray-300 rounded px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500".into(),
    textarea: "border border-gray-300 rounded px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 min-h-[8rem]".into(),
    button:   "bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700 disabled:opacity-50".into(),
    error:    "text-red-600 text-sm".into(),
    success:  "bg-green-50 border border-green-300 text-green-800 p-4 rounded".into(),
}
```

## Custom CSS example

```css
.contact-form { max-width: 600px; }
.contact-field { margin-bottom: 1rem; display: flex; flex-direction: column; }
.contact-label { font-weight: bold; margin-bottom: 0.25rem; }
.contact-input,
.contact-textarea { border: 1px solid #ccc; border-radius: 4px; padding: 0.5rem; }
.contact-button { background: #0070f3; color: white; padding: 0.5rem 1.5rem; border: none; border-radius: 4px; cursor: pointer; }
.contact-button:disabled { opacity: 0.6; cursor: default; }
.contact-error  { color: #c00; }
.contact-success { background: #f0fff4; border: 1px solid #38a169; padding: 1rem; }
```

## Labels and i18n

Override any label text with [`ContactFormLabels`](./configuration.md):

```rust
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
