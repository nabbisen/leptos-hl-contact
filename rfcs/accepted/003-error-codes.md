# RFC 003 — Error codes and localisable server messages

**Status.** Accepted — proposed and accepted by the owner on 2026-09-12.
**Handoffs.** [`../handoffs/003-error-codes/README.md`](../handoffs/003-error-codes/README.md)
**Tracks.** Roadmap M2 item P-14.  Requirements FR-I18N-02 (MUST, Gap),
FR-SUB-06, FR-UI-08.  External Design §4.2.3 target form, §6.
**Touches.** `error.rs`, `model.rs` (`validate_fields`), `config.rs`
(labels, policy check), `server.rs` (message strings), `components.rs`
(code → text), documentation.

## Summary

Every visitor-visible string that the server composes today is English
and cannot be overridden: field validation messages, policy messages, the
token failure, the "not configured" and "failed to send" texts.  This RFC
moves the server to **codes** and makes the component turn codes into text
from `ContactFormLabels`.  After it, the crate never composes visitor-
facing text on the server, and a fully translated form is possible.

## Motivation

The GUI rule requires multilingual support.  FR-I18N-01 (component
strings) is met; FR-I18N-02 is not.  A Japanese site today shows Japanese
labels and English error messages.

## Goals

- All visitor-visible messages localisable through one struct.
- Codes carry the parameters a message needs (`min`, `max`).
- The client accepts the old text form for one minor release.
- Logs stay English and unchanged.

## Non-goals

- Translating anything ourselves (presets are P-20).
- Changing which conditions produce an error.
- Changing the sentinel-based transport (a separate error type on the
  wire would be a larger, riskier change with no benefit to visitors).

## Design

### D1 — Codes

```rust
// error.rs
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldErrorCode {
    Required,
    Length { min: usize, max: usize },   // characters
    Format,                              // e.g. email syntax
    LineBreaks,                          // CR/LF in name or subject
}

/// One field's error: a code (current servers) or pre-rendered text (0.3 servers).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldError { Code(FieldErrorCode), Text(String) }

pub struct ContactFieldErrors {
    pub name: Option<FieldError>, pub email: Option<FieldError>,
    pub subject: Option<FieldError>, pub message: Option<FieldError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactErrorCode {
    TokenInvalid,      // bad, expired, or missing form token
    NotConfigured,     // delivery or token context missing
    DeliveryFailed,
    Unexpected,
}
pub const CONTACT_ERROR_PREFIX: &str = "contact_error:";
```

Wire format:

| Situation | Variant | String |
|-----------|---------|--------|
| field errors | `Args` | `field_errors:{"name":{"kind":"length","min":1,"max":80},"email":{"kind":"format"}}` |
| token | `Args` | `contact_error:token_invalid` |
| not configured, delivery failed, unexpected | `ServerError` | `contact_error:not_configured` etc. |

Serde `untagged` on `FieldError` lets a new client read a 0.3 server's
`{"name":"Name must be 1–80 characters"}` as `Text`.  A 0.3 client reading
a new server fails to parse the object as a string and falls back to the
generic banner, which is what it shows today anyway.

### D2 — Producing codes

- `ContactInput::validate_fields` maps `validator` results by their
  `code`: `"length"` → `Length { min, max }` taken from the error's
  `params` (`min` may be absent → `0`); `"email"` → `Format`;
  `"no_newlines"` → `LineBreaks`; anything else → `Format`.  Empty
  required string fields are reported by `validator` as `length` with
  `min: 1`; the client renders `Required` text when `min ≥ 1` and the
  code is `Length` with an empty field — no: keep it simple, the client
  renders `Length` as "between {min} and {max} characters".  The server
  emits `Required` only from policy (`require_subject`).
- `ContactServerPolicy::check` (RFC 001) emits `Required` and
  `Length { min: 1, max: limit }`.
- `server.rs` replaces the four literal strings with codes.

### D3 — Rendering text

```rust
// config.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactErrorLabels {
    pub required:       String, // "This field is required."
    pub length:         String, // "Must be between {min} and {max} characters."
    pub format_email:   String, // "Enter a valid email address."
    pub format:         String, // "Invalid value."
    pub line_breaks:    String, // "Line breaks are not allowed here."
    pub token_invalid:  String, // "Your session token expired. Please reload the page and try again."
    pub not_configured: String, // "This form is not available right now."
    pub delivery_failed:String, // "Failed to send message. Please try again later."
}
pub struct ContactFormLabels { …existing…, pub errors: ContactErrorLabels }
```

- `{min}` and `{max}` placeholders are replaced by the component; no
  format library.
- `Format` on the email field renders `format_email`; on other fields
  `format`.
- `Text(s)` renders `s` unchanged.
- `labels.error` stays as the last-resort fallback for unrecognised
  payloads and is now documented as such; `delivery_failed` is what
  visitors see for that case.

### D4 — Component

`ContactFieldErrors::from_server_fn_error` is unchanged in signature.  A
new `ContactErrorCode::from_server_fn_error(&ServerFnError<E>) -> Option<Self>`
parses the `contact_error:` prefix from `Args` or `ServerError`.  The
generic banner memo becomes: field payload present → none; contact code
present → its label; otherwise `labels.error`.

## Amendment 2026-09-13 — `Required` for blank values

D2 said the client renders `Length` for every length violation, because
`validator` reports a blank required string that way.  Handoff 01
implemented that faithfully and it reads badly: an empty box is told it
must be "between 1 and 80 characters".  D2 is amended — the crate emits
`Required` when the trimmed value is empty and the rule has `min ≥ 1`,
which `validator` cannot distinguish but the model can.  No new code
variant, no wire change.  Handoff 02 (roadmap P-29).

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Server-side localisation with a `lang` argument | The server would need every translation; contradicts "the crate never chooses a language" |
| A custom `ServerFnError` type | Changes the server function's error type, a public contract, and needs `FromServerFnError`; heavier, same result |
| Codes as plain strings without parameters | Length messages could not name the limit |

## Compatibility

Minor.  Breaking for code that constructs or matches `ContactFieldErrors`
fields (`Option<String>` → `Option<FieldError>`) and for struct-literal
construction of `ContactFormLabels` (new `errors` field; `..Default::default()`
users unaffected).  Migration notes in the CHANGELOG.  Wire format:
dual-accept as described.  DOM contract unchanged.

## Security considerations

Codes reveal only which validation rule failed, as the texts already did.
No visitor input is echoed.  Placeholder replacement is plain string
substitution on integrator-supplied text rendered as a text node.

## Testing

- `error/tests.rs`: serde round-trip for every `FieldErrorCode` variant;
  `FieldError` deserialises from a string and from an object; prefix
  parsing for `ContactErrorCode` from both variants.
- `model/tests.rs`: each validation failure yields the expected code and
  parameters.
- `config/tests.rs`: policy emits `Required` / `Length`; placeholder
  substitution helper renders `{min}`/`{max}`.
- `components/tests.rs` (SSR): not applicable to dynamic errors; covered
  by the browser evidence of RFC 002 handoff 02 re-run with Japanese
  labels (evidence requirement in the handoff).

## Acceptance criteria

FR-I18N-02 Met: a form with `ContactFormLabels` fully overridden shows no
English string in any state.  Verified with a Japanese label set on the
hydrated example and the no-JS curl path.

## Implementation boundaries

One handoff: codes, labels, component, docs.  Depends on RFC 002
handoff 02.

## Open questions

None.

## Release implications

Part of the proposed `0.4.0`, together with RFC 002.
