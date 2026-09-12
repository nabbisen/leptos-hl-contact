# Styling

The crate ships no CSS.  You pass class strings through
[`ContactFormClasses`](./customization.md#contactformclasses) and the
component places them on the right elements.  The element ids and class
hooks are listed in the [DOM contract](../development/external-design.md#412-dom-contract)
and are stable within a minor version.

## Tailwind CSS

```rust,ignore
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

## Plain CSS

```rust,ignore
ContactFormClasses {
    root:     "contact-form".into(),
    field:    "contact-field".into(),
    label:    "contact-label".into(),
    input:    "contact-input".into(),
    textarea: "contact-textarea".into(),
    button:   "contact-button".into(),
    error:    "contact-error".into(),
    success:  "contact-success".into(),
}
```

```css
.contact-form   { max-width: 600px; }
.contact-field  { margin-bottom: 1rem; display: flex; flex-direction: column; }
.contact-label  { font-weight: bold; margin-bottom: 0.25rem; }
.contact-input,
.contact-textarea { border: 1px solid #ccc; border-radius: 4px; padding: 0.5rem; }
.contact-button { background: #0070f3; color: white; padding: 0.5rem 1.5rem; border: none; border-radius: 4px; cursor: pointer; }
.contact-button:disabled { opacity: 0.6; cursor: default; }
.contact-error  { color: #c00; }
.contact-success { background: #f0fff4; border: 1px solid #38a169; padding: 1rem; }
```

## Things to keep

- **Focus outlines.**  The component does not suppress them; neither
  should your CSS.
- **Contrast.**  You own the colours, so you own WCAG AA contrast (4.5:1
  for normal text).  Error and success states are also conveyed in text.
- **The honeypot wrapper** is hidden with an inline `position:absolute;
  left:-9999px` style.  A global rule that resets `position` on every
  element would reveal it.
- **Attribute selectors** such as `input[aria-invalid="true"]` let you style
  the error state without extra classes.
