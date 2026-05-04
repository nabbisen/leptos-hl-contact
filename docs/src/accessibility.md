# Accessibility

`leptos-hl-contact` is accessible by default.  The following features are
built into the standard `ContactForm` component.

## Labels

Every input has a `<label>` element linked via `for` / `id`.  `placeholder`
attributes are **not** used as label substitutes — they are decorative only.

## Required fields

Required fields carry both the HTML `required` attribute (for native browser
validation) and `aria-required="true"` (for screen readers that do not
surface native validation).

## Per-field error messages

When server-side validation fails, each field with an error renders an inline
`<p>` error message with:

- `role="alert"` and `aria-live="polite"` — announced by screen readers.
- `id="{input-id}-error"` — referenced by `aria-describedby` on the input.

The input itself gains `aria-invalid="true"` when its field has an error.

```html
<!-- Example rendered output -->
<input
  id="contact-name"
  aria-required="true"
  aria-invalid="true"
  aria-describedby="contact-name-error"
/>
<p id="contact-name-error" role="alert" aria-live="polite">
  Name must be 1–80 characters
</p>
```

## Submit button state

While a submission is in flight the button is `disabled` and carries
`aria-busy="true"`.  The button text also changes to the `labels.sending`
string (default: "Sending…") so the state is conveyed in text, not only
visually.

## Honeypot field

The honeypot `<input>` and its `<label>` are wrapped in a container with:

- `aria-hidden="true"` — hidden from all assistive technology.
- `tabindex="-1"` on the input — excluded from keyboard navigation.
- Absolute positioning off-screen — invisible to sighted users.

## Success and generic error messages

- **Success**: `role="status"` + `aria-live="polite"` — announced
  non-intrusively after submission.
- **Generic delivery error**: `role="alert"` + `aria-live="assertive"` —
  announced immediately.

## Keyboard navigation

All interactive elements are native HTML elements (`<input>`, `<textarea>`,
`<button>`), so full keyboard navigation works without any extra effort.
Focus outline is not suppressed.

## Colour contrast

The built-in component has no default colours.  Apply your own CSS and ensure
sufficient contrast ratios (WCAG AA: 4.5:1 for normal text).  State
(error/success) is always conveyed in text as well as colour.

## Testing recommendations

- Run [axe](https://www.deque.com/axe/) or [Lighthouse](https://developer.chrome.com/docs/lighthouse/accessibility/) on your page.
- Test with a screen reader (NVDA + Firefox, VoiceOver + Safari).
- Test keyboard-only navigation.
- Test with JavaScript disabled (progressive enhancement via `<ActionForm/>`).
