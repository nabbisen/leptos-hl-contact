# Handoff 019-01 — `ContactFormOptions::native_validation`

**RFC.** [RFC 019](../../done/019-optional-novalidate.md) D1–D4
**Roadmap.** P-41
**Requirements.** FR-UI-02, FR-I18N-02, FR-A11Y-03, FR-PE-01/02

## Goal

A site can turn off the browser's own prompting, so every message a visitor
reads comes from its own labels.  Every other site renders exactly what it
renders today.

## Change scope

### 1. `src/config.rs`

- **`ContactFormOptions`** gains `pub native_validation: bool`.
  - **Default `true`,** in the manual `Default` impl, beside
    `honeypot_inline_style`.
  - **Serde:** `#[serde(default = …)]` yielding `true`, so a 0.9-shaped
    value still deserializes.  Follow `honeypot_inline_style` exactly.
  - **Rustdoc:** what `false` does, that the inputs keep `required`,
    `type` and `maxlength`, and the trade — language control against one
    round trip before the visitor is told a field is empty.

### 2. `src/components.rs`

- **The `<form>`** gains `novalidate` **only** when the option is `false`.
  - **Absent means absent:** no empty attribute when the option is `true`.
  - Follow how `honeypot_inline_style` omits the `style` attribute.
- **Nothing else changes.**  `required`, `type="email"`, `maxlength`,
  `aria-required` and the error wiring stay exactly as they are, on every
  field including site-defined ones.

### 3. Tests

**Unit/render (`src/components/tests.rs`):**
- the default renders **no** `novalidate`, and the whole `<form>` open tag
  is byte-identical to 0.9.0's — capture the current string first and assert
  it, as `no_site_fields_render_the_markup_of_0_7_byte_for_byte` does;
- `false` renders `novalidate` exactly once, on the `<form>`;
- the inputs keep `required` and `aria-required` in both cases.

**Serde (`src/config/tests.rs`):** a 0.9-shaped `ContactFormOptions` JSON
deserializes with `native_validation` true.

**Browser (`tests/browser/`):**
- with `false`, submitting an empty required field reaches the stubbed
  server, and the site's own error text appears under the field with
  `aria-invalid`, focus on the first invalid field;
- with the default, the existing behaviour is unchanged (name the existing
  test that already covers it, rather than duplicating it).

**Break check, required.**  Render `novalidate` unconditionally.
- **Expected:** the byte-identical default-markup test fails.
- Report the output, then restore.

### 4. Documentation

| Page | Change |
|------|--------|
| `guides/customization.md` | the option in the `ContactFormOptions` table, and a short paragraph: what `false` does, what it costs (one round trip), and that `required` and the ARIA attributes stay |
| `guides/localization.md` | the sentence that answers the original report: with `false`, every message a visitor reads comes from your labels — the browser's own pop-up appears in the browser's language, which no label can change |
| `guides/accessibility.md` | one line: `novalidate` suppresses the browser's prompting only; the inputs keep `required` and `aria-required`, and the crate's own error wiring is unchanged |
| `reference/api.md` | the new field and its default |

### 5. `CHANGELOG.md` `[Unreleased]`

- **Added:** the option, what it is for, and that the default is unchanged.
- **Migration:** a `ContactFormOptions { … }` literal without
  `..Default::default()` must add the field.

## Gates

- The shared gates on both toolchains; the MSRV checks; both wasm suites
  with counts; the examples with `--locked`; `mdbook build docs`; CI.

## Review request

`.git-exclude/review-request/019-optional-novalidate/01-native-validation-option.md`:
the commit; the rendered `<form>` tag in both states; the tests with
results; the break check; the gates and CI.
