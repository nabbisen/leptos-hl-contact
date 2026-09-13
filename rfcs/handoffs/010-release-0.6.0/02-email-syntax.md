# Handoff 010-02 — Email addresses a form can reply to

**RFC.** [RFC 010](../../accepted/010-release-0.6.0.md) D2
**Roadmap.** P-34
**Requirements.** FR-VAL-02, FR-VAL-07
**Depends on.** Handoff 01 merged.

## Goal

`submit_contact` refuses email addresses that no public contact could reply
to: an address literal, a domain without a dot, or more than 254
characters.  Each is refused with an existing field code, so every site's
existing labels still apply.

## Change scope

### `src/model.rs`

On `ContactInput::email`:
- **Keep** `#[validate(email)]`.
- **Add** `length(max = 254)`.
- **Add** a custom validator, for example
  `custom(function = "reply_to_domain", code = "email")`, rejecting:
  - a domain part starting with `[` (an address literal);
  - a domain part with fewer than two labels when split on `.`;
  - any empty label (`a@example.`, `a@.com`, `a@example..com`).

**Split at the last `@`.**  `validator` has already checked the syntax, so
the domain is everything after it.

**Codes.**
- **Domain rules.**  Their failures map to `FieldErrorCode::Format`, through
  the code `email` or the existing fallback in `field_error_code`.
- **Length.**  The length rule maps to `FieldErrorCode::Length { min: 0, max: 254 }`,
  the same shape as `subject`.

**When both fail** (a 300-character invalid address): the field shows the
`Length` code.
- **Ordering.**  If `validator`'s error order does not give you that from
  attribute order, choose the length error explicitly where the field's
  first error is picked.
- **Report.**  Say which of the two you did.

**Blank.**  A blank email keeps its current code.  Assert it in a test, so
this change cannot move it.

### Tests

**L1, `src/model/tests.rs`**, one test per row:

| Input | Expected |
|-------|----------|
| `ada@example.com`, `ada@mail.example.co.jp`, `ada@例え.jp` | valid |
| `abc@bar` | `Format` |
| `a@[127.0.0.1]`, `a@[2001:db8::1]` | `Format` |
| `a@example.`, `a@.com`, `a@example..com` | `Format` |
| a 254-character valid address | valid |
| a 255-character address | `Length { min: 0, max: 254 }` |
| blank | the current code (unchanged) |

**L2, `tests/server/validation.rs`.**  Add `abc@bar` → `email` `Format` and a
255-character address → `email` `Length { min: 0, max: 254 }` to
`each_rule_rejects_with_its_field_code`, in both forms like the other rows.

### Documentation

| File | Change |
|------|--------|
| `docs/src/development/requirements.md` | FR-VAL-02 text as in RFC 010 D2, status "Met (0.6.0)"; change-history row (next draft number) |
| `docs/src/development/external-design.md` | the `email` row of §4.2.1 Request fields (currently "trim; valid address"): the four rules |
| `docs/src/reference/api.md` | the `email` row: the same rules, briefly |
| `docs/src/development/testing.md` | FR-VAL-02 row gains the new tests; FR-VAL-07's note about `maxlength="254"` without a server limit is removed |

### CHANGELOG

Under `[Unreleased]` → `### Changed`: the rule, with one migration line.
Addresses such as `user@localhost` and `user@[127.0.0.1]` are now refused;
they cannot be replied to from a public mailbox.

## Acceptance

1. All tests above pass.
2. **Deliberate break.**  Remove the two-label rule and show the `abc@bar`
   unit and server tests fail.  Then restore.
3. **Browser.**  `type="email"` accepts `abc@bar`.  Show one no-JS round trip
   from the server suite where the rendered page carries the `format_email`
   text under the email field.
4. Gates and both suites pass.

## Review request

`.git-exclude/review-request/010-release-0.6.0/02-email-syntax.md`
