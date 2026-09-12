# Handoff 01 — Codes on the wire, labels on the client

**RFC.** [003](../../accepted/003-error-codes.md), all of D1–D4.
**Requirements.** FR-I18N-02, FR-SUB-06, FR-UI-08, FR-UI-09.

## Purpose

Remove every visitor-facing English string from the server; render all
messages from `ContactFormLabels`.

## Change scope

`error.rs` + tests, `model.rs` (`validate_fields`) + tests, `config.rs`
(labels, policy check) + tests, `server.rs`, `components.rs`, `lib.rs`
re-exports, and the documentation pages listed below.

## Explicit non-change scope

Validation rules; the `field_errors:` sentinel and the `ServerFnError`
variants used; DOM contract; log messages (stay English).

## Required implementation

1. **`error.rs`.**  Add exactly the types from RFC 003 D1:
   `FieldErrorCode` (tagged `kind`, snake_case), `FieldError` (untagged
   `Code | Text`), `ContactErrorCode` (snake_case: `token_invalid`,
   `not_configured`, `delivery_failed`, `unexpected`),
   `CONTACT_ERROR_PREFIX = "contact_error:"`, and a `ContactField` enum
   `{ Name, Email, Subject, Message }`.  Change the four
   `ContactFieldErrors` fields to `Option<FieldError>`.  Add:
   - `ContactErrorCode::as_str(self) -> &'static str`, `from_str_code(&str) -> Option<Self>`,
     `into_server_fn_message(self) -> String`,
     `from_server_fn_error<E>(&ServerFnError<E>) -> Option<Self>` (looks in
     `Args` and `ServerError`, tolerant of leading text like
     `from_error_str`).
   - `ContactFieldErrors::get(&self, ContactField) -> Option<&FieldError>`.
2. **`model.rs`.**  `validate_fields` maps `validator` errors: code
   `"length"` → `Length { min, max }` from `params["min"]` / `params["max"]`
   (`as_u64`, missing `min` → 0); `"email"` → `Format`; `"no_newlines"` →
   `LineBreaks`; otherwise `Format`.  Remove the `message = "…"` strings
   from the `#[validate]` attributes; they are no longer used.
3. **`config.rs`.**
   - `ContactErrorLabels` with the eight fields and English defaults from
     RFC 003 D3.  `ContactFormLabels` gains `pub errors: ContactErrorLabels`.
   - `impl ContactErrorLabels { pub fn field_text(&self, field: ContactField, err: &FieldError) -> String; pub fn code_text(&self, code: ContactErrorCode) -> String }`.
     `field_text`: `Text(s)` → `s`; `Required` → `required`; `Length{min,max}`
     → `length` with `{min}`/`{max}` replaced by decimal numbers; `Format`
     → `format_email` when `field == Email`, else `format`; `LineBreaks` →
     `line_breaks`.  `code_text`: the matching label; `Unexpected` →
     `delivery_failed`.
   - `ContactServerPolicy::check` emits `Code(Required)` and
     `Code(Length { min: 1, max: limit })`.
4. **`server.rs`.**  Replace the literal strings: token failure →
   `Args(ContactErrorCode::TokenInvalid.into_server_fn_message())`; token
   config missing and delivery context missing → `ServerError(NotConfigured…)`;
   delivery failure → `ServerError(DeliveryFailed…)`; unexpected honeypot
   error → `ServerError(Unexpected…)`.  Keep the log lines.
5. **`components.rs`.**  `FieldError` message signals use
   `labels.errors.field_text(field, err)`.  The generic banner memo:
   field payload present → `None`; `ContactErrorCode::from_server_fn_error`
   → `Some(labels.errors.code_text(code))`; else `Some(labels.error)`.
6. **`lib.rs`.**  Re-export `FieldErrorCode`, `FieldError`,
   `ContactErrorCode`, `ContactField`, `ContactErrorLabels`.

## Required tests

- `error/tests.rs`: serde round-trip for each `FieldErrorCode` variant;
  `FieldError` from `"text"` and from `{"kind":"length","min":1,"max":80}`;
  `ContactErrorCode` prefix parsing from `Args` and `ServerError`, with
  and without leading framework text; `unexpected` string → `None` for
  unknown codes.
- `model/tests.rs`: empty name → `Length{min:1,max:80}`; 81-char name →
  same code; bad email → `Format`; newline in name → `LineBreaks`;
  121-char subject → `Length{min:0,max:120}`; 4 001-char message →
  `Length{min:1,max:4000}`.
- `config/tests.rs`: `field_text` renders `{min}`/`{max}`; email `Format`
  uses `format_email`; policy emits the two codes; defaults non-empty.
- Existing tests updated for the new field type.

## Required documentation updates

- `guides/localization.md`: replace "What is not localisable yet" with the
  `errors` block and a full Japanese example including error labels.
- `guides/customization.md`: `errors` in the labels table.
- `reference/api.md`: new types; `ContactFieldErrors` field type; wire
  examples.
- `help/troubleshooting.md` and `security/csrf.md`: quoted default texts
  updated to the new defaults.
- `development/external-design.md` §4.2.2–4.2.3: mark the code form
  current; §6 table.
- `CHANGELOG.md` Unreleased: Changed (breaking for struct-literal users
  of `ContactFieldErrors` / `ContactFormLabels`) with a migration note.

## Acceptance criteria

- Tests pass; gates green.
- With a fully Japanese `ContactFormLabels` on the hydrated example:
  screenshots of a field error, the token failure banner (set a 1-second
  TTL and wait), and the delivery failure banner (point SMTP at a closed
  port) show no English.  No-JS curl path (as in RFC 001 handoff 03)
  shows the Japanese field text in the HTML.
- `grep -rn '"[A-Z][a-z].* ' crates/leptos-hl-contact/src/server.rs`
  finds no visitor-facing English sentence (log strings excepted; list
  them).

## Prohibited shortcuts

Keeping English fallbacks on the server "just in case"; format macros
with positional arguments in labels; changing validation limits.

## Compatibility and security constraints

Per RFC 003.  Placeholder substitution is plain replace on integrator
text rendered as a text node.

## Known risks

- `server_fn` 0.8.13 deprecates `NoCustomError` and `WrappedServerError`
  ahead of removal in 0.9.  Do not name either; rely on `ServerFnError`'s
  default type parameter (as `error/tests.rs` does since handoff 001-03).

`validator` may report an empty required string as `length` with only
`min`; handle a missing `max` as `usize::MAX` and render `length` anyway
(the default English text still reads correctly for `max = 4000`; note
any oddity in the review request).

## Required evidence

Gate outputs; test results; screenshots; the curl transcript; the `grep`.
