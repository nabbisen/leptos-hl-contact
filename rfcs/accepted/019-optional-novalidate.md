# RFC 019 — An opt-in `novalidate`, so a site's own error text is what visitors read

**Status.** Accepted — 2026-09-22, as proposed: `native_validation`,
default `true`, one form-wide switch.  Milestone M8 (P-41).
**Tracks.** Roadmap P-41.  Requirements FR-UI-02, FR-I18N-02, FR-A11Y-*,
FR-PE-01/02.
**Handoffs.** [`../handoffs/019-optional-novalidate/README.md`](../handoffs/019-optional-novalidate/README.md)
**Touches.** `config.rs` (one option), `components.rs` (one attribute),
tests (unit, browser), docs, `CHANGELOG.md`.
**Origin.** The reflerd.com team, 2026-09-17: on a Japanese page in an
English browser, the browser's own pop-up appears in English, and no label
the site passes can change it.

## Summary

`ContactFormOptions::native_validation`, default `true` (today's
behaviour).  With `false`, the form renders `novalidate`, the browser stops
prompting, and every message the visitor reads comes from the site's own
`ContactErrorLabels`.

## Why it is worth an option rather than a default

- **The browser's prompt is good** when the page and the browser share a
  language: it is instant, needs no round trip, and is what visitors expect.
- **It is wrong** when they do not: a translated form answers in the
  browser's language, which the site cannot touch.  That is the case the
  option exists for.
- **So the default stays `true`.**  A site that has not thought about it
  keeps the faster, familiar behaviour.

## Design

### D1 — The option

```rust,ignore
// ContactFormOptions
/// Let the browser check `required`, `type="email"` and `maxlength` before
/// the form is submitted.  `true` by default.
///
/// With `false` the form carries `novalidate`: the browser prompts for
/// nothing, every submission reaches the server, and the visitor reads the
/// site's own `ContactErrorLabels` text instead of the browser's.
pub native_validation: bool,
```

- **`#[serde(default = …)]` yielding `true`,** as `honeypot_inline_style`
  does, so a 0.9-shaped value still deserializes.
- **It is a new public field** on a struct integrators write as a literal:
  one migration line, exactly as RFC 012's two fields needed.

### D2 — What changes in the markup

- **`false`:** the `<form>` gains `novalidate`.  **Nothing else changes.**
- **The inputs keep `required`, `type`, `maxlength` and
  `aria-required`.**  They are what assistive technology reads, and
  `novalidate` suppresses only the browser's own prompting.
- **`true`:** the markup is byte-identical to 0.9's.  A test pins that.

### D3 — What the visitor experiences with `false`

- **With JavaScript:** submit, the server answers, the field errors appear
  under the fields with `aria-invalid` and focus moves to the first one —
  the path the crate already has and tests.
- **Without JavaScript:** submit, the `__err` redirect, the errors on the
  landing page.  Also unchanged.
- **The cost** is one round trip before a visitor learns the field is empty.
  The documentation must say so plainly: this option trades immediacy for
  language control.

### D4 — What this RFC does **not** add

- **No client-side validation of our own.**  Re-implementing the browser's
  checks in Rust means two sources of truth for the same rule, and the
  server already validates everything.
- **No per-field opt-out.**  One form, one choice.

## Tests

- **Unit/render:** `true` renders no `novalidate` and markup identical to
  0.9's; `false` renders it exactly once, on the `<form>`.
- **Browser:** with `false`, submitting an empty required field reaches the
  stubbed server and the site's own error text appears under the field, with
  `aria-invalid` and focus; with `true`, the existing behaviour is unchanged.
- **Serde:** a 0.9-shaped `ContactFormOptions` still deserializes, with the
  option `true`.
- **Break check:** render `novalidate` unconditionally → the byte-identical
  test fails.

## Documentation

- **Customization:** the option, the trade (language control against one
  round trip), and that `required` and the ARIA attributes stay.
- **Localization:** the sentence that answers the original report — with
  `false`, every message a visitor reads comes from your labels.
- **Accessibility:** what `novalidate` does and does not affect.

## Compatibility

Breaking only for a `ContactFormOptions { … }` literal without
`..Default::default()`: one migration line.  Default behaviour is unchanged.

## Handoffs (planned)

One: the option, the attribute, the tests, the documentation.

## Owner decisions (2026-09-22)

Accepted as proposed; the questions as put, with the answers, were:

1. **Default `true`.**  Alternative: `false`, which would change every
   existing site's behaviour to gain language control they may not need.
2. **The name `native_validation`,** read as "let the browser validate".
   Alternative: `novalidate`, which names the HTML attribute but reads as a
   double negative when set to `false`.
3. **One form-wide switch,** not per field.
