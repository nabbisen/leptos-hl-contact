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
    ..Default::default()
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
    ..Default::default()
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

## Honeypot

The honeypot field is hidden by an inline `style` attribute on its wrapper,
so it works with no CSS at all.  A Content Security Policy without
`'unsafe-inline'` blocks that attribute, and the field then shows.  Under
such a policy, hide the wrapper with your own class instead:

```rust,ignore
ContactFormClasses {
    honeypot: "contact-honeypot".into(),
    ..Default::default()
}

ContactFormOptions {
    honeypot_inline_style: false,   // render no style attribute on the wrapper
    ..Default::default()
}
```

```css
.contact-honeypot { position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; }
```

> **Warning.**  With `honeypot_inline_style: false` and no such rule, the
> field is visible.  A visitor who fills it in is treated as a bot: the form
> reports success and the message is silently discarded.

`aria-hidden="true"` on the wrapper, and `tabindex="-1"` and
`autocomplete="off"` on the input, are rendered in both modes, so the field
stays out of assistive technology, the tab order and autofill whatever your
CSS does.  With the inline style kept (the default), the class is optional
and is added beside it.

## Things to keep

- **Focus outlines.**  The component does not suppress them; neither
  should your CSS.
- **Contrast.**  You own the colours, so you own WCAG AA contrast (4.5:1
  for normal text).  Error and success states are also conveyed in text.
- **The honeypot wrapper** is hidden with an inline `position:absolute;
  left:-9999px` style by default, or by your own rule when
  `honeypot_inline_style` is `false` (see [Honeypot](#honeypot)).  A global
  rule that resets `position` on every element would reveal it.
- **Attribute selectors** such as `input[aria-invalid="true"]` let you style
  the error state without extra classes.
