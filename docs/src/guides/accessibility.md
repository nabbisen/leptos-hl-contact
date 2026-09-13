# Accessibility

`ContactForm` is accessible without configuration.  What the component
guarantees:

| Aspect | Behaviour |
|--------|-----------|
| Labels | Every input has a `<label for>`; placeholders are never used as labels |
| Required fields | `required` for native validation plus `aria-required="true"` |
| Field errors | Inline `<p role="alert" aria-live="polite" id="{input-id}-error">`; the input gains `aria-invalid="true"` and `aria-describedby` |
| Submit state | While pending the button is `disabled`, carries `aria-busy="true"`, and its text changes to `labels.sending` |
| Success | `role="status"` + `aria-live="polite"` |
| Generic error | `role="alert"` + `aria-live="assertive"` |
| Honeypot | Wrapper is `aria-hidden="true"`; the input has `tabindex="-1"` and sits off-screen |
| Focus | After a failed submission focus moves to the first invalid input, so a keyboard or screen-reader user lands on the field to correct.  Set `focus_first_error: false` in [`ContactFormOptions`](./customization.md#contactformoptions) to disable it |
| Keyboard | Native `<input>`, `<textarea>`, `<button>` only; focus outlines untouched |
| Colour | None shipped; state is always conveyed in text |

Example of the rendered error state:

```html
<input id="contact-name" aria-required="true" aria-invalid="true"
       aria-describedby="contact-name-error" />
<p id="contact-name-error" role="alert" aria-live="polite">
  Must be between 1 and 80 characters.
</p>
```

With a [success page](./customization.md#success-page) configured the
confirmation is a navigation to a new page rather than a live region.  Both
are accessible; make the page's first heading say what happened, so a screen
reader announces the outcome on arrival.

## Your responsibilities

- Colour contrast of the classes you supply (WCAG AA: 4.5:1).
- `lang` on the document when you translate the labels.
- Not overriding `position` on the honeypot wrapper.

## Testing

- Run [axe](https://www.deque.com/axe/) or Lighthouse on the page.
- Try NVDA + Firefox and VoiceOver + Safari.
- Navigate by keyboard only.
- Submit with JavaScript disabled.
