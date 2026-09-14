# RFC 012 — The honeypot without an inline style

**Status.** Proposed — 2026-09-15.  Part of milestone M5 → 0.7.0, as a
small separate item (owner, 2026-09-15).
**Tracks.** Roadmap P-39.  Requirements FR-UI-10, FR-UI-03, FR-SUB-04,
NFR-COMPAT-05 (the DOM contract).
**Touches.** `config.rs`, `components.rs`, their tests, the styling and
challenge guides, External Design §4.1.2.
**Origin.** A request from the reflerd.com team, 2026-09-13.

## Summary

The honeypot's wrapper carries an inline `style` attribute that moves it
off screen.  A Content Security Policy without `'unsafe-inline'` blocks
that attribute.  The field then becomes visible, and a person who types
into it is silently discarded as a bot (FR-SUB-04).

This RFC lets a site hide the honeypot with its own CSS instead:
- **A class hook** on the wrapper.
- **An option** to omit the inline style.
- **The default is unchanged,** so no existing site is affected.

## Motivation

`components.rs:795` renders:

```html
<div aria-hidden="true" style="position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden">
  <input id="contact-website" name="website" tabindex="-1" autocomplete="off" …>
</div>
```

Under `style-src` without `'unsafe-inline'`, browsers ignore the attribute.
A site can allow it by hash only with `'unsafe-hashes'`.  That is what the
reflerd.com site does today, together with its own CSS as a second layer.

**The failure harms real visitors, not bots.**  A sighted visitor sees an
unlabelled extra field, may fill it in, and their message is dropped with a
success response.  Nobody learns about it.

FR-UI-03 says the crate ships no mandatory CSS.  The inline style exists so
that FR-UI-10 (the honeypot is invisible) holds with no CSS at all.  Both
must keep holding.

## Design

### D1 — A class hook

`ContactFormClasses` gains `honeypot: String`, default empty, applied to the
wrapper `<div>`.

### D2 — An option to omit the inline style

`ContactFormOptions` gains `honeypot_inline_style: bool`, **default `true`**.

| Value | Wrapper renders | Who hides it |
|-------|-----------------|--------------|
| `true` (default) | the inline style, as today, plus the class if set | the crate |
| `false` | the class only; **no** `style` attribute | the site's CSS, through the class |

Unchanged in both modes: `aria-hidden="true"` on the wrapper, and
`tabindex="-1"` and `autocomplete="off"` on the input.  Those keep the field
out of assistive technology, the tab order and autofill whatever CSS does.

### D3 — Documentation

- **`guides/styling.md`.**  The option, and a CSS rule to copy:
  `.your-honeypot { position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; }`.
  Also a warning: with the option `false` and no rule, the field is
  visible.
- **`security/challenge.md` (CSP section, with RFC 011 D8).**  Under a
  policy without `'unsafe-inline'`, set the option to `false` and hide the
  class.
- **External Design §4.1.2 (the DOM contract).**  The wrapper's class hook
  and the conditional `style` attribute.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Remove the inline style by default | Every site without that CSS would show the field, and visitors who fill it in would be silently discarded — the failure this RFC exists to prevent |
| The `hidden` attribute | Many bots skip hidden fields, which weakens the honeypot for every site |
| A `<style nonce>` element | Needs a nonce threaded into the form even without a challenge, and a `style-src` nonce policy; more mechanism than a class and a flag |
| Leave it to the site's CSS with `!important` overrides | Cannot remove an attribute the CSP has already rejected; the site still needs `'unsafe-hashes'` |

## Compatibility

Additive, with the default unchanged.  A struct literal of
`ContactFormClasses` or `ContactFormOptions` without `..Default::default()`
must add the new field.  The CHANGELOG gives a migration line.  The DOM
contract changes only when a site opts in.

**Release:** 0.7.0, with RFC 011.

## Security considerations

The honeypot's detection is unchanged in both modes.  The opt-in moves the
responsibility for hiding the field to the site's CSS.  The documentation
states that responsibility plainly, with the rule to copy.

## Testing

- **`components::attributes::the_honeypot_is_hidden_from_everyone`** runs in
  both modes:
  - **default:** the style is present;
  - **opted out:** no `style` attribute, the class is present, and
    `aria-hidden`, `tabindex` and `autocomplete` are unchanged.
- **Deliberate break:** render the style regardless of the option; the
  opted-out case fails.
- **Traceability:** the FR-UI-10 and FR-A11Y-06 rows cite both cases.

## Owner decision requested

1. **The inline style stays the default** (`honeypot_inline_style: true`),
   and sites under a strict CSP opt out.  *Recommended: yes.*
