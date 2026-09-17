# RFC 015 — Fields defined by the site, bounded

**Status.** Proposed — 2026-09-17.  Milestone M6 → 0.8.0.  The owner accepted
the feature (P-43) the same day, as a bounded feature with the bounds written
into the requirements.  "A form builder" stays a non-goal.
**Tracks.** Roadmap P-43.  Requirements FR-UI-01 (amended), new FR-FIELD-01
to FR-FIELD-08, §1 scope, FR-OBS-01/02, FR-I18N-02, FR-A11Y-*.
**Touches.** `config.rs` (the definition), `model.rs` (`ContactInput`,
validation), `error.rs` (`ContactFieldErrors`), `server.rs`
(`submit_contact`), `components.rs` (rendering, errors, focus),
`delivery/smtp.rs` (the body), tests at every layer, docs, `CHANGELOG.md`.
**Origin.** The reflerd.com team, 2026-09-17: a business enquiry needs a few
short, structured fields rather than one message box.

## Summary

- **The definition.**  A site defines a few extra fields: a key, a label,
  a kind (one line, several lines, or a choice from a fixed list), whether
  it is required, and a length limit.
- **Both sides use it.**  The component renders from that definition, and
  the server validates against it.
- **The server's allow-list.**  It accepts only defined keys and listed
  choices.
- **Delivery** receives the values in definition order, with their labels.
- **Everything else is unchanged:** every other protection, error code and
  label, and every logging rule.

## Where the bound is (the line against a form builder)

These bounds are requirements, not defaults.  Widening any of them needs its
own RFC.

| Bound | Value |
|-------|-------|
| Kinds | `Line` (one line), `Text` (several lines), `Choice` (one of a fixed list).  No checkbox, radio group, number, date, file, or hidden field |
| Count | at most **8** fields |
| Placement | one fixed place: after the subject, before the message, in definition order |
| Logic | none: no conditional fields, no cross-field rules, no custom validators |
| Keys | `[a-z][a-z0-9_]{0,31}`, unique, and not a name the form already uses: `name`, `email`, `subject`, `message`, `website`, `form_token`, `fields`, or a vendor token name |
| Lengths | `Line` at most 200 characters; `Text` at most `MESSAGE_MAX_LEN`; `Choice`: 2 to 20 options, keys as for field keys, labels non-empty |
| Layout | none beyond `ContactFormClasses`: each field is one row with the existing field classes |

## Design

### Step 0 — Spike, before any other work

`submit_contact` takes fixed named arguments.  Extra fields need one
map-shaped argument:

```rust,ignore
#[server(default)]
fields: Option<BTreeMap<String, String>>,
```

On the wire it would be `fields[topic]=sales`.

**The spike must show, on the current Leptos and `server_fn`:**
1. **Both request forms:** the argument round-trips through the fetch
   request and the no-JavaScript form post, which is URL-encoded and parsed
   by `serde_qs`.
2. **Absent means empty:** a submission with no `fields[…]` still
   deserializes, as `None`.
3. **Keys stay distinct:** a crafted key with brackets or dots, such as
   `fields[a][b]` or `fields.a`, is either refused at deserialization or
   arrives as a key the allow-list refuses.  It must never reach delivery.
4. **No-JavaScript refusals:** the `__err` redirect carries field errors for
   the new keys.
5. **Input preservation:** it behaves as it does for the built-in fields.

**If any point fails, stop and report.**  The shape changes (for example,
one encoded string argument), or the RFC returns to the owner.  This is the
RFC 011 step-0 rule.

### D1 — The definition (`config.rs`)

```rust,ignore
#[derive(Clone, Debug, PartialEq)]
pub struct ContactFields(/* private: Vec<ContactField> */);

impl ContactFields {
    /// Validates every bound in "Where the bound is".
    pub fn new(fields: Vec<ContactField>) -> Result<Self, InvalidContactFields>;
    pub fn empty() -> Self;             // also `Default`
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactField {
    pub key: String,
    pub label: String,
    pub kind: ContactFieldKind,
    pub required: bool,
    /// `Line` and `Text` only; clamped as `max_message_len` is.
    pub max_len: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContactFieldKind {
    Line,
    Text,
    Choice(Vec<ContactChoice>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactChoice { pub key: String, pub label: String }
```

- **Validation happens at construction.**  A `ContactFields` value that
  exists is valid.  `InvalidContactFields` names the rule and the field
  **key**, but never a label.
- **The same definition is used on both sides:**
  - **the component:** a new prop, `fields: ContactFields`, optional and
    empty by default;
  - **the server:** a new field, `ContactServerPolicy::fields`, empty by
    default.
- **Documented pattern: build it once,** in shared code, and pass it to
  both, as the Challenge page does for the widget and the policy.
- **A mismatch fails closed** (D3), and a test pins that.

### D2 — Validation and errors (`model.rs`, `error.rs`)

**Normalisation** is as for the built-in fields: trim.  A blank optional
field is absent.

**Rules and their codes** (existing codes and labels only):

| Case | Code | Label |
|------|------|-------|
| Required and blank | `required` | `labels.errors.required` |
| Over `max_len` | `length` | `labels.errors.length` |
| `Line` with CR or LF | `no_newlines` | `labels.errors.line_breaks` |
| `Choice` value not a listed key | `format` | `labels.errors.format` |

**Unknown key.**
- **Refused,** as the whole submission: `rejected`, the code the filter
  hook already uses.  Its rustdoc ("A `ContactFilter` refused the
  submission") widens to "refused by a filter, or it carried a field the
  site did not define".
- **The log:** `warn`, with the **count** of unknown keys and never the
  keys, which are attacker text.
- **No field error:** there is no field to attach it to.

**`ContactFieldErrors`** gains
`pub fields: BTreeMap<String, FieldError>`.
- **It breaks struct literals,** so it is a 0.8.0 change.
- **The pre-rendered JSON** (`field_errors:{…}`) gains a `fields` object.
  A 0.7 payload has none and still parses (`#[serde(default)]`).

### D3 — The server (`server.rs`)

- **Order:** the definition check runs with field validation, after the
  honeypot and the form token and before the policy, the challenge, the
  filter and delivery.  The pipeline order does not change.
- **Unknown keys** are refused first, before any per-field rule, so a
  crafted request learns nothing about the definition.
- **A server-required field the page did not render** is refused as
  `required`.
  - **In the component,** an error for a key it did not render goes to the
    generic banner, not to a missing field.
  - **So the mismatch is loud** in the site's own testing.

### D4 — Delivery (`model.rs`, `delivery/smtp.rs`)

`ContactInput` gains the values:

```rust,ignore
pub fields: Vec<ContactFieldValue>,

pub struct ContactFieldValue {
    pub key: String,
    pub label: String,
    /// For `Choice`, the choice key.
    pub value: String,
    /// For `Choice`, the choice label; `None` otherwise.
    pub value_label: Option<String>,
}
```

- **Only answered fields** are included, in definition order.  Labels come
  from the server's definition, never from the request.
- **Breaking change.**  Adding the field breaks `ContactInput` struct
  literals, in custom backends' tests and in filters, so it ships in
  0.8.0.
- **The SMTP body:** after `Subject:` and before `Message:`, one block per
  value, in the body's existing style.
  - **Text and line fields:** `Label:` then the value.
  - **Choices:** `Label:` then `choice label (choice-key)`.
  - **Headers:** unchanged.
- **The filter hook** receives the same `ContactInput`, so a site filter
  can read the values.

### D5 — Rendering (`components.rs`)

- **Placement:** after the subject row, before the message row.
- **Structure:** each field is one row with the existing field, label,
  input and error classes.
- **The controls:**
  - `Line`: `<input type="text" maxlength=…>`.
  - `Text`: `<textarea maxlength=…>`.
  - `Choice`: a native `<select>`.
    - **Required:** its first option is an empty placeholder, "—", or a
      label the site passes.
    - **Optional:** the empty option means "no answer".
- **Names:** `fields[key]`.  The `id` is derived from the form's existing id
  scheme and the key.
- **Accessibility**, as for the built-in fields:
  - a `<label for>`;
  - `required` and `aria-required`;
  - on error, `aria-invalid` and `aria-describedby`;
  - `focus_first_error` in document order.
- **Preservation:** typed input and selections are preserved on a failed
  submission, as FR-UI-07 requires.

### D6 — Privacy and logs

- **Values** are personal data, like the message.
- **Never** written to a log line, an error or a label.
- **Logs** may carry field **keys** from the definition, not from the
  request.
- **`Debug`** on `ContactInput` behaves as it does today for `message`.
  Changing that is out of scope.

### D7 — Requirements (written by the architect at acceptance)

- **§1 scope.**  "A form builder" stays out of scope, with a pointer to the
  bounds in FR-FIELD-01.
- **FR-UI-01, amended:** "…and at most eight site-defined fields within
  FR-FIELD-01".
- **FR-FIELD-01 to FR-FIELD-08:**
  1. the bounds table;
  2. single definition;
  3. allow-list: unknown keys refused;
  4. choice values limited to listed keys;
  5. existing codes and labels;
  6. delivery order and labels;
  7. privacy;
  8. accessibility parity with the built-in fields.

## Tests

- **Unit tests:**
  - every `ContactFields::new` bound, including the reserved keys, duplicates
    and over-count;
  - each validation row of D2;
  - `ContactFieldErrors` JSON, with and without `fields`;
  - the SMTP body.
- **L2** (server suite, through the documented router), in both request
  forms:
  - an unknown key → `rejected`, and nothing delivered;
  - a wrong choice → `format`;
  - over-length → `length`;
  - `required` → `required`;
  - a CR/LF in a `Line` → `no_newlines`;
  - a valid submission → delivered, with values in order and labels from
    the server;
  - a server-required field absent from the request → `required`.
- **L3** (browser):
  - rendering and names;
  - errors under the new fields;
  - preservation;
  - focus on the first invalid field, in document order;
  - an error for an unrendered key shows the banner.
- **Break checks:** allow-list removed → the unknown-key test fails;
  choice check removed → the wrong-choice test fails.
- **Mutation run** before the 0.8.0 release candidate, as usual.

## Compatibility

**Breaking, in 0.8.0:** `ContactInput` and `ContactFieldErrors` gain a
public field.  Struct literals must add it, or use the new constructors or
`..Default::default()` where available.  The Migration notes say so.

**Unchanged:**
- **Sites that define no fields** render byte-identical markup.
- **Their wire format** is unchanged.

## Handoffs (planned)

| # | Scope |
|---|-------|
| 01 | Step 0 spike (stop point); D1 definition and its validation; D2 validation and errors, unit tests |
| 02 | D3 server; D5 rendering; L2 and L3 tests |
| 03 | D4 delivery and the SMTP body; D6; docs (Customization, Localization, API, Delivery Backends, Accessibility); traceability; CHANGELOG |

## Owner questions (recommendations first)

1. **The maximum: 8 fields.**  Alternative: 5.  The team uses two or three;
   8 leaves room without inviting long forms.
2. **Choices render only as a `<select>`.**  Alternative: a radio group as
   a second kind.  A select is one control, works at 320 px, and keeps the
   bound small.  Radios can be a later RFC.
3. **Blank optional fields are left out of delivery.**  Alternative: include
   them as empty.  Leaving them out keeps mail short, and the site knows its
   definition.
4. **The placement: after the subject, before the message.**  Alternative:
   after the message.  The message is the free-text summary, so it reads
   best last.
