# Handoff 012-01 — Class hook and inline-style opt-out

**RFC.** [RFC 012](../../accepted/012-honeypot-without-inline-style.md) D1–D3
**Roadmap.** P-39
**Requirements.** FR-UI-10, FR-UI-03, FR-A11Y-06, NFR-COMPAT-05
**Owner decision.** The inline style stays the default.

## Goal

A site under a strict Content Security Policy can hide the honeypot with its
own CSS: the crate renders a class on the wrapper and no `style` attribute.
Every other site renders exactly what it does today.

## Change scope

### 1. `src/config.rs`

**`ContactFormClasses`** gains `honeypot: String`.
- **Rustdoc:** applied to the honeypot's wrapper `<div>`.  Needed when
  `honeypot_inline_style` is `false`, and optional otherwise.
- **Default:** empty.  The struct derives `Default`.

**`ContactFormOptions`** gains `honeypot_inline_style: bool`.
- **Default:** `true`, in the manual `Default` impl at `config.rs:541`.
- **Rustdoc:** with `false`, the crate renders no `style` attribute, so the
  site must hide the element through `ContactFormClasses::honeypot`, or the
  field becomes visible.  Include the CSS rule from RFC 012 D3.

**Serde.**  Both structs derive `Serialize`/`Deserialize`.  A value
serialized by 0.6 has no new field.
- **Classes:** give `honeypot` `#[serde(default)]`.
- **Options:** give `honeypot_inline_style` a `#[serde(default = …)]` that
  yields `true`.
- **Test it:** 0.6-shaped JSON still deserializes, with the defaults.

### 2. `src/components.rs` — the wrapper (currently lines 793–796)

- **`class`:** the classes' `honeypot` value, following how the other
  wrappers render their class (`class=""` when empty is consistent with
  them).
- **`style`:** present with today's exact value when
  `honeypot_inline_style` is `true`, absent when `false`.  Absent means no
  attribute at all, not an empty one.
- **Unchanged:** `aria-hidden="true"` on the wrapper, the `<label>`, and the
  input's `id`, `name`, `type`, `tabindex="-1"` and `autocomplete="off"`.

### 3. Documentation (D3)

| File | Change |
|------|--------|
| `docs/src/guides/styling.md` | a "Honeypot" section: the class, the option, the CSS rule to copy, and a warning that `false` without the rule shows the field |
| `docs/src/guides/customization.md` | the new field in both tables (classes and options) |
| `docs/src/reference/api.md` | both fields |
| `docs/src/development/external-design.md` | §4.1.2 honeypot wrapper row: class hook `classes.honeypot`; `aria-hidden=true`; off-screen inline style unless `honeypot_inline_style` is `false` |
| `docs/src/security/challenge.md` | **nothing yet**: RFC 011 handoff 04 writes the CSP section and links this option |

### 4. CHANGELOG `[Unreleased]`

- **Added:** `ContactFormClasses::honeypot` and
  `ContactFormOptions::honeypot_inline_style`, for a Content Security Policy
  without `'unsafe-inline'`.
- **Migration:** a `ContactFormClasses { … }` or `ContactFormOptions { … }`
  literal without `..Default::default()` must add the field.  The rendered
  form is unchanged unless you opt out.

## Tests

**`components::attributes::the_honeypot_is_hidden_from_everyone`** splits
into two tests, or one test covering both modes.

**Default options:** as today.  The inline style is present with its exact
value.

**Opted out** (`honeypot_inline_style: false`, `honeypot: "hp"`):
- the wrapper has **no** `style` attribute;
- `class="hp"`;
- `aria-hidden="true"`;
- the input still has `tabindex="-1"` and `autocomplete="off"`;
- the input is still inside the wrapper.

**`config::`:**
- the defaults: empty class, `true`;
- 0.6-shaped JSON for both structs deserializes to the defaults.

**Traceability:** the FR-UI-10 and FR-A11Y-06 rows cite both modes; FR-UI-03
gains the class test.

## Acceptance

1. Gates, both suites and both examples pass.
2. **Deliberate break.**  Render the `style` attribute regardless of the
   option.  The opted-out assertion fails.  Then restore.
3. **The default render** is byte-identical to 0.6 for the honeypot wrapper.
   Paste the wrapper's HTML from the default test, before and after.

## Review request

`.git-exclude/review-request/012-honeypot-without-inline-style/01-class-and-opt-out.md`
