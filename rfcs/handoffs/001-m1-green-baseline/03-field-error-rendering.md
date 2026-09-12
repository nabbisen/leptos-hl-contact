# Handoff 03 — Per-field errors render in the browser

**RFC.** [001](../../accepted/001-m1-green-baseline.md), decision D1.
**Roadmap.** P-02.  **Requirements.** FR-UI-08, FR-SUB-06, FR-A11Y-03.
**External Design.** §4.2.3 Field-error payload protocol; §4.1.3 State model.
**Depends on.** Handoffs 01 and 02.

## Purpose

Show each validation error beside its field, as the documentation has
promised since 0.2.0.

## Background

`submit_contact` returns `ServerFnError::Args("field_errors:{json}")`.
In `src/components.rs` the closures `field_errors` and `generic_error`
call `e.to_string()` and test `s.starts_with(FIELD_ERROR_PREFIX)`.  The
framework's `Display` for `ServerFnError::Args(s)` is
`"error deserializing server function arguments: {s}"`
(`server_fn` 0.8, `src/error.rs`), so the test is always false.  Result:
the generic banner appears and no field error is ever rendered, in both
WASM and no-JS modes (the no-JS path reconstructs the same variant from
the `__err` URL parameter).

## Change scope

- `src/error.rs`, `src/error/tests.rs`
- `src/components.rs`
- `docs/src/development/architecture.md` (one paragraph, see below)

## Explicit non-change scope

- The wire format stays `field_errors:` + JSON with the same four members
  (error codes are M2, P-14).
- `FieldError` markup, ids, ARIA attributes, class hooks unchanged (DOM
  contract).
- No change to how the form is rebuilt on value change (P-10 is M2).
  You will notice that a failed submission clears the inputs; that is a
  known separate defect.  Do not fix it here.
- `server.rs` unchanged.

## Required implementation

1. **`error.rs`.**
   - Make `ContactFieldErrors::from_error_str` tolerant of leading text:
     locate the sentinel with `find(FIELD_ERROR_PREFIX)`, parse the JSON
     that follows it.  Return `None` when the sentinel is absent or the
     JSON fails to parse.  Update its rustdoc.
   - Add
     `pub fn from_server_fn_error<E>(err: &leptos::server_fn::error::ServerFnError<E>) -> Option<Self>`
     that returns `Self::from_error_str(s)` for the `Args(s)` variant and
     `None` for every other variant.  Document that this is the intended
     way for a client to detect a field-error payload and that
     `from_error_str` is the fallback for callers holding only a string.
2. **`components.rs`.**
   - Replace the string tests in `field_errors` and `generic_error` with
     `ContactFieldErrors::from_server_fn_error(e)`.
   - `field_errors` returns the parsed value, or `Default` when `None` or
     when the parsed value `is_empty()`.
   - `generic_error` returns `labels.error` exactly when the value is
     `Some(Err(e))` and `from_server_fn_error(e)` is `None` or empty; an
     error carrying field errors shows no banner.  This keeps the token
     failure message (`Args` with plain text) and every `ServerError` on
     the banner path.
   - Remove the now-unused `FIELD_ERROR_PREFIX` import if it becomes
     unused.
3. **`architecture.md`, "Error flow to the client".**  Delete the
   parenthetical `(0.3.3 defect: …)` line and state that the component
   matches the `Args` variant.

## Required tests

In `src/error/tests.rs`:

- `from_error_str_accepts_framework_display_prefix`: input
  `"error deserializing server function arguments: field_errors:{\"name\":\"required\"}"`
  parses with `name == Some("required")`.
- `from_error_str_still_accepts_bare_sentinel` (keep the existing
  round-trip test passing).
- `from_error_str_rejects_missing_sentinel` (exists; keep).
- `from_error_str_rejects_bad_json_after_sentinel`: `"field_errors:{not json"`
  returns `None`.
- `from_server_fn_error_parses_args_variant`: construct
  `ServerFnError::<server_fn::error::NoCustomError>::Args(errs.into_server_fn_message())`
  and assert the parsed fields.
- `from_server_fn_error_ignores_args_without_payload`: `Args("Invalid or
  expired security token…")` returns `None`.
- `from_server_fn_error_ignores_server_error_variant`: `ServerError("…")`
  returns `None`.

## Required documentation updates

The `architecture.md` paragraph above.  `help/troubleshooting.md` "Known
issues in 0.3.3" keeps its P-02 row until release; the architect removes
it in the release pass.

## Acceptance criteria

- All tests above pass; gates green.
- **Manual evidence, no-JS path**, using `examples/axum-with-security`
  after handoff 02 (replace the port if different):

  ```bash
  export CSRF_SECRET=$(openssl rand -hex 32) ALLOWED_ORIGIN=http://127.0.0.1:3000
  cargo run &   # in examples/axum-with-security
  TOKEN=$(curl -s http://127.0.0.1:3000/ | grep -o 'name="csrf_token" value="[^"]*"' | cut -d'"' -f4)
  LOC=$(curl -s -o /dev/null -D - -X POST http://127.0.0.1:3000/api/submit_contact \
    -H 'Accept: text/html' -H 'Origin: http://127.0.0.1:3000' -H 'Referer: http://127.0.0.1:3000/' \
    --data-urlencode "name=" --data-urlencode "email=nope" --data-urlencode "message=" \
    --data-urlencode "website=" --data-urlencode "csrf_token=$TOKEN" | grep -i '^location:' | cut -d' ' -f2 | tr -d '\r')
  curl -s "$LOC" | grep -o 'id="contact-[a-z]*-error"[^<]*<\|aria-invalid="true"'
  ```

  Expected: the last command prints `aria-invalid="true"` three times and
  the three error paragraphs for name, email, and message.  Paste the
  output.  If the redirect location is relative, prefix the host.
- **Manual evidence, WASM path** is not required in M1 because the
  examples ship no client bundle; state this in the review request.

## Prohibited shortcuts

String matching on the framework's English display text; changing the
server to emit a different variant; special-casing in `FieldError`.

## Module boundaries

`error` owns parsing; `components` only calls it.  `server` is untouched.

## Compatibility constraints

`from_error_str` becomes more permissive; existing callers see no change
for inputs that previously parsed.  New public function only.  Patch-level.

## Security constraints

Parsed messages are rendered as text nodes by Leptos (escaped).  Do not
introduce `inner_html`.  Field messages remain the server's generic
strings; nothing from the visitor's input is echoed.

## Known risks

If the `ServerAction` value type in the component is not
`ServerFnError<NoCustomError>` (check the `SubmitContact` type generated
by `#[server]`), adjust the generic parameter of `from_server_fn_error`
accordingly and note it in the review request.  Do not change the server
function's error type.

## Required evidence

Gate outputs; test names and results; the no-JS curl transcript above.
