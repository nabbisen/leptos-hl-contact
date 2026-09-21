# Handoff 015-02 — The definition, its validation, and field validation

**RFC.** [RFC 015](../../done/015-site-defined-fields.md): the bounds table, D1, D2, and the **Amendment** (A1, A2, A3, A5), which supersedes D1 and D2 where they differ
**Roadmap.** P-43
**Depends on.** 01 (spike, reviewed)

## Goal

**The types and pure functions** that the server (03) and the component (03)
will use, with unit tests.
- **Not wired in yet:** no change to `submit_contact`, `ContactForm`,
  `ContactInput`, or delivery.
- **Additive only:** every existing test passes unchanged.

## Change scope

### 1. `src/config.rs` — the definition (A1)

```rust,ignore
#[derive(Clone, Debug, PartialEq)]
pub struct SiteFields(Vec<SiteField>);          // private field

impl SiteFields {
    pub const MAX: usize = 4;
    pub fn new(fields: Vec<SiteField>) -> Result<Self, InvalidSiteFields>;
    pub fn empty() -> Self;                      // and `Default`
    pub fn iter(&self) -> impl Iterator<Item = &SiteField>;
    pub fn get(&self, key: &str) -> Option<&SiteField>;
    pub fn is_empty(&self) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct SiteField { pub key: String, pub label: String, pub kind: SiteFieldKind, pub required: bool, pub max_len: usize }

#[derive(Clone, Debug, PartialEq)]
pub enum SiteFieldKind { Line, Text, Choice(Vec<SiteFieldChoice>) }

#[derive(Clone, Debug, PartialEq)]
pub struct SiteFieldChoice { pub key: String, pub label: String }

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct InvalidSiteFields(/* private */ String);
```

**`SiteFields::new` enforces every row of the bounds table:**

| Rule | Error message names |
|------|---------------------|
| at most `MAX` fields | the count |
| key matches `[a-z][a-z0-9_]{0,31}` | the key |
| keys unique | the key |
| key not reserved: `name`, `email`, `subject`, `message`, `website`, `form_token`, `fields`, `cf_turnstile_response`, `h_captcha_response`, `g_recaptcha_response` (and the hyphenated forms are already excluded by the charset) | the key |
| label non-empty after trimming | the key |
| `Line`: `1 ..= 200` for `max_len` | the key |
| `Text`: `1 ..= MESSAGE_MAX_LEN` for `max_len` | the key |
| `Choice`: 2 to 20 choices; choice keys match the key charset and are unique; choice labels non-empty; `max_len` ignored | the key (and the choice key) |

**Where the vendor token names come from.**  Check them against
`submit_contact`'s argument names in `server.rs`, and list the exact
reserved set in the rustdoc.

**`InvalidSiteFields` messages** name the rule and the **field key** from the
definition, never a label.

**Rustdoc on `SiteFields`:**
- the bounds;
- that they are requirements, not defaults;
- a small example with the three kinds, the example the RFC's origin gives:
  `organisation`, `topic`, `timing`.

**Re-export** all six types from `lib.rs`, next to the other `config`
re-exports.

### 2. `src/model.rs` — field validation (A2, A3)

```rust,ignore
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteFieldValue { pub key: String, pub label: String, pub value: String, pub value_label: Option<String> }

pub enum SiteFieldsOutcome {
    /// Every value is valid: answered fields, in definition order.
    Valid(Vec<SiteFieldValue>),
    /// Per-field errors, keyed by the definition's key.
    Invalid(BTreeMap<String, FieldError>),
    /// Too many keys, or a key the definition does not have (A3).
    Refused,
}

pub fn validate_site_fields(def: &SiteFields, raw: Option<&BTreeMap<String, String>>) -> SiteFieldsOutcome;
```

The names are a suggestion.  If a clearer shape fits the existing code,
use it and say why.

**In order:**
1. **`raw` is `None` or empty:** treat every field as absent.
2. **`raw.len() > SiteFields::MAX`:** `Refused`, with a `warn` log carrying
   the count only.
3. **Any key not in `def`:** `Refused`, with a `warn` log carrying the count
   of unknown keys only.  **Never log a request key or value** (A5).
4. **Each defined field, in definition order:** trim the value, and treat
   blank as absent.
   - **Absent and required:** `Required`.
   - **Absent and optional:** skipped, so it is not in `Valid`.
   - **`Line`:**
     - a CR or LF → `LineBreaks`;
     - over `max_len` characters (Unicode scalar values, as today) →
       `Length { min, max }`, where `min` is 1 if required, else 0.
   - **`Text`:** over `max_len` → `Length`.
   - **`Choice`:** a value that is not a listed choice key → `Format`.  The
     value is the choice key, and `value_label` is the choice's label.
   - **Order of checks:** where several apply to one field, `Length` first,
     as `validate_fields` does (RFC 010 D2).
5. **Any errors:** `Invalid`.  Otherwise `Valid`.
6. **Labels in `SiteFieldValue`** come from `def`, never from `raw`.

### 3. `src/error.rs` — `ContactFieldErrors` (A1)

- **New field:** `pub site_fields: BTreeMap<String, FieldError>`, with
  `#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]`.
  - **Old JSON.**  A 0.7 payload still parses.
  - **New JSON without site-field errors** is byte-identical to 0.7's.  A
    test must show it.
- **`is_empty()`,** or whatever the type uses to decide "no errors," must
  include the new map.

**This is the breaking change** for struct literals of `ContactFieldErrors`.
Add the CHANGELOG *Migration* line now (§5).

### 4. Tests

**`src/config/tests.rs`:**
- one test per bounds row: accepted at the limit, refused one past it;
- every reserved key;
- a duplicate key;
- a duplicate choice key;
- the valid example from the rustdoc.

**`src/model/tests.rs`** (or a new `tests/site_fields.rs` under it):
- **Each outcome** of §2: `Refused` for 5 keys; `Refused` for an unknown
  key.
- **A key with `\n` in it:** `Refused`, not a crash.
- **Per field:** required, length at and over the limit, line breaks, an
  unlisted choice.
- **Absent optional:** omitted from `Valid`.
- **Ordering:** `Valid` follows definition order, even though the
  `BTreeMap` iterates alphabetically.  Use keys whose alphabetical order
  differs from definition order.
- **Labels:** they come from the definition.

**Log assertions** (reuse the crate's log-capture pattern):
- the refusal logs contain the count;
- they do **not** contain the unknown key.
- **Use a distinctive fake key**, such as `zz_probe_key`, and assert that
  it is absent from the captured output.

**`src/error/tests.rs`:**
- JSON round trip with `site_fields`;
- a 0.7 payload parses;
- no `site_fields` in the JSON when the map is empty.

**Break checks, required:**
- remove the unknown-key refusal → the unknown-key test fails;
- remove the choice-key check → the unlisted-choice test fails.

### 5. `CHANGELOG.md` `[Unreleased]`

- **Added:** "`SiteFields`: fields defined by the site (RFC 015); not yet
  rendered or accepted by `submit_contact`."  Handoff 03 replaces this line.
- **Migration:** "`ContactFieldErrors` has a new field, `site_fields`;
  struct literals add it or use `..Default::default()`."

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The MSRV checks** (the `msrv` job's three commands) on 1.88.
- **The browser and worker suites:** counts unchanged.
- **CI** on the pushed commit.

## Review request

`.git-exclude/review-request/015-site-defined-fields/02-definition-and-validation.md`:
- the commit;
- the reserved-key list and its source;
- the outcome function's final shape;
- the break checks;
- the log assertions;
- the gates;
- CI.
