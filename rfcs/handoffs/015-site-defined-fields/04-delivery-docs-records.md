# Handoff 015-04 — The SMTP body, documentation, traceability

**RFC.** [RFC 015](../../done/015-site-defined-fields.md): D4, D6, D7 (docs only; the architect writes the requirements), Amendment A4–A6
**Roadmap.** P-43
**Depends on.** 03 (approved)

## Change scope

### 1. `src/delivery/smtp.rs` — the body (D4)

`build_plain_text_body` adds one block per `SiteFieldValue`, after
`Subject:` and before `Message:`, in the existing style:

```text
Organisation:
Example Ltd

Topic:
Sales (sales)

Message:
…
```

- **Choices:** `{value_label} ({value})`.
- **Line and text fields:** the value.
- **No site fields:** the body is byte-identical to 0.7's.  A test pins the
  whole string.
- **Headers:** unchanged.  No site value reaches a header.
- **Unit tests:** with values; without values (byte-identical); and a
  multi-line `Text` value kept intact.

### 2. Documentation

| Page | Change |
|------|--------|
| `guides/customization.md` | a "Site-defined fields" section: that a value is trimmed first, so a trailing newline in a `Line` is trimmed rather than refused, as for `name`; the bounds (as requirements, not defaults), the example, building one `SiteFields` and passing it to both `ContactForm` and `ContactServerPolicy`, and what a mismatch looks like (unknown keys refused; an error for an unrendered field shows the generic message) |
| `guides/localization.md` | labels and choice labels come from the definition, in the page language; errors reuse `ContactErrorLabels` |
| `guides/delivery-backends.md` | `ContactInput::site_fields`: order, labels from the server, choice key and label, answered fields only; for a custom backend, treat values like the message (personal data, never logged) |
| `guides/accessibility.md` | site fields get the same label, `required`, `aria-*` and focus behaviour; a choice is a native `<select>` |
| `reference/api.md` | the six types, the prop, the policy field, `ContactInput::site_fields`, `ContactFieldErrors::site_fields` |
| `security/README.md` or the "which layer decides what" table | a row: unknown site-field keys and unlisted choices are refused by the crate |
| `development/external-design.md` | the form's field table: the site-field rows with names, ids and codes |
| `development/architecture.md` | if the source layout changed, match it exactly |
| `development/testing.md`, "Running the examples" | one clause: CI checks the examples with `--locked`, so a stale example lock fails at push time (carried from the RFC 016 handoff 02 review) |
| `introduction.md` | "A form builder" stays in the non-goals; add "(a few bounded site-defined fields are supported; see Customization)" |

**Grep:** after editing, `git grep -n "ContactField\b" -- docs/src`.  Every
hit must mean the existing error enum, not the new definition.

### 3. Traceability (`development/testing.md`)

- **New rows:** FR-FIELD-01 to FR-FIELD-08, as the architect adds them to
  `requirements.md` in this handoff's review.  Map each to the tests from
  02 and 03 using the RFC's D7 list.
- **FR-UI-01:** add the site-field L3 rendering test.

### 4. `CHANGELOG.md` `[Unreleased]`

- **Documentation:** the pages above.
- **Check the whole RFC 015 entry set:** *Added* and *Migration*
  (`ContactFieldErrors`, `ContactInput`).

## Gates

- **The shared gates.**
- **The MSRV checks.**
- **Both wasm suites.**
- **`mdbook build docs`:** no warnings.
- **CI.**

## Review request

`.git-exclude/review-request/015-site-defined-fields/04-delivery-docs-records.md`:
- the commit;
- the body with and without values;
- each page's change;
- the grep;
- the traceability rows;
- the gates;
- CI.
