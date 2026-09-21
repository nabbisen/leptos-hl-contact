# Handoff 015-03 — The server and the component

**RFC.** [RFC 015](../../done/015-site-defined-fields.md): D3, D5, and the Amendment (A1, A3, A4, A6, A7)
**Roadmap.** P-43
**Depends on.** 02 (approved)

## Goal

**Site fields work end to end, up to delivery.**
- **The component** renders them.
- **`submit_contact`** accepts and validates them.
- **Errors** show under the right field.
- **`ContactInput`** carries the values to delivery.

The SMTP body and the documentation come in 04.

## Change scope

### 1. `src/config.rs`

- **`ContactServerPolicy`:** gains `pub site_fields: SiteFields`, default
  empty.
- **`ContactForm`:** a new prop, `site_fields: SiteFields`, optional and
  empty by default, documented next to `challenge`.

### 2. `src/server.rs`

**The argument, last,** exactly as spiked:

```rust
#[server(rename = "fields")]
#[server(default)]
site_fields: Option<BTreeMap<String, String>>,
```

- **Wire name:** `fields` (A1).  If `rename` cannot be combined as above,
  name the argument `fields` and say so.
- **Order in the pipeline:** at the same point as field validation.  After
  the honeypot and the form token; before the server policy, the challenge,
  the filter and delivery.  The pipeline order does not change.
- **The outcomes:**
  - `Refused` → `rejected`.
  - `Invalid` → merged into the same `ContactFieldErrors` as the built-in
    fields' errors, so one response carries all field errors.
  - `Valid` → the values go into `ContactInput`.
- **Honeypot parity.**  A honeypot hit still returns silent success before
  any site-field check, so a bot learns nothing.  A test pins it.

### 3. `src/model.rs` — `ContactInput`

- **New field:** `pub site_fields: Vec<SiteFieldValue>`, with
  `#[serde(default)]`.
- **`from_raw`'s signature does not change.**  It sets an empty `Vec`, and
  the server fills the field after validation.
- **Breaking change:** struct literals of `ContactInput`.  Add the
  *Migration* line (§6).

### 4. `src/error.rs` — `ContactErrorCode::Rejected`

**Widen the rustdoc** to "refused by a filter, or it carried a field the
site did not define" (RFC D2).

### 5. `src/components.rs` (D5, A6, A7)

**Placement:** after the subject row, before the message row, in definition
order.

**The rows,** built like the built-in rows:
- **`Line`:** `<input type="text">`, with `maxlength=max_len`.
- **`Text`:** `<textarea>`, with `maxlength=max_len`.
- **`Choice`:** a `<select>`.
  - **The first option is `value=""`, with its text `—`.**
  - **Required:** `required`.  The browser then refuses the empty option,
    and the server check still runs.
- **Names and ids:** `name="fields[{key}]"` and `id="contact-field-{key}"`.
  The label is the definition's `label`.
- **The rest of each row:**
  - `required` and `aria-required` when required;
  - on error, `aria-invalid` and `aria-describedby="contact-field-{key}-error"`;
  - the error paragraph with that id, text from `ContactErrorLabels`, the
    same as for the built-in fields.
- **Classes:** the existing field, label, input and error classes, and
  nothing new.

**Errors for a key the component did not render** go to the generic
banner.
- **The condition:** an entry in `site_fields` whose key is not in the
  prop.
- **Why:** it shows a server/component mismatch instead of dropping the
  error.

**`focus_first_error`:** first invalid field in document order, including
site fields between the subject and the message.

**`hydrate`:** the markup is identical on the server and the client, as for
the other rows.

### 6. `CHANGELOG.md` `[Unreleased]`

- **Replace** 02's *Added* line with the feature as shipped: the three
  kinds, at most 4 fields, one definition for `ContactForm` and
  `ContactServerPolicy`, unknown keys refused.
- **Migration:** "`ContactInput` has a new field, `site_fields`."

## Tests

### L2 (server suite) — both request forms, through `expect_both` or its pattern

**Setup:** a policy with the three-field example.

**Cases and expected results:**

| Submission | Expected |
|------------|----------|
| valid | delivered, and the recorded `ContactInput.site_fields` in definition order, with server labels |
| 5 keys | `rejected`, not delivered |
| an unknown key | `rejected`, not delivered |
| a key with `%0A` | `rejected` |
| required blank | `required`, under that key |
| over-length | `length` |
| a CR or LF in a `Line` | `line_breaks` |
| an unlisted choice | `format` |
| honeypot filled, with invalid site fields | silent success, nothing delivered |
| **A4:** duplicate key `fields[topic]=a&fields[topic]=b` | the fetch request's error is not a `contact_error:` or `field_errors:` payload; the no-JS landing page shows `labels.error` |
| no `fields[…]` at all, with an empty definition | exactly today's behaviour (the existing tests cover it; name one) |

**Log assertion:** the refusals do not contain the fake unknown key.

### L3 (browser suite)

- **Rendering:** the three rows render with the names, ids, `required` and
  the `—` option.
- **The submitted body** (stubbed `fetch`, as the spike did) contains
  `fields[topic]=…`.
- **A field error for `topic`** shows under it, with `aria-invalid`.
- **Focus:** it goes to the first invalid site field when it precedes the
  message.
- **Preservation:** after a failed submission, the typed `organisation` and
  the selected `topic` are still there (A6).
- **Mismatch:** an error for an unrendered key shows the generic banner.

### Break checks, required

- **Allow-list:** make the server skip the refusal.  The unknown-key L2 test
  fails.
- **Component:** render the `Choice` without the empty first option.  The
  L3 rendering test fails.

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The MSRV checks** on 1.88.
- **Both wasm suites:** counts.
- **The examples,** as CI runs them.  They do not use site fields; they must
  compile unchanged, or report what changed.
- **CI.**

## Review request

`.git-exclude/review-request/015-site-defined-fields/03-server-and-rendering.md`:
- the commit;
- the pipeline position with line references;
- the rendered markup of the example, one row per kind;
- the tests table with results;
- the break checks;
- the gates;
- CI.
