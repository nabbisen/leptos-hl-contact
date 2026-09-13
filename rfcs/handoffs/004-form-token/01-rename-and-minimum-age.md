# Handoff 01 — Rename with aliases; minimum age

**RFC.** [004](../../done/004-form-token.md), D1, D2, D5.
**Requirements.** FR-ABUSE-02, FR-ABUSE-03, FR-ABUSE-13.

## Purpose

Name the feature for what it guarantees; reject submissions younger than
two seconds with a retryable message; keep 0.4 code compiling.

## Change scope

`Cargo.toml` features; `src/csrf.rs` → `src/form_token.rs` (+ tests dir);
new `src/csrf.rs` alias module; `components.rs` (field name);
`server.rs` (arguments, mapping); `error.rs` (`TooFast`); `config.rs`
(`too_fast` label); `lib.rs`; both examples; documentation.

## Explicit non-change scope

Token format; HMAC; TTL semantics; cookie binding and client acquisition
(handoffs 02, 03).

## Required implementation

1. **Features.**  `form-token = ["ssr", "dep:hmac", "dep:sha2", "dep:rand", "dep:hex"]`;
   `csrf = ["form-token"]` with a comment "deprecated alias, removed in the
   next minor".  Every `#[cfg(feature = "csrf")]` becomes
   `#[cfg(feature = "form-token")]`.
2. **Module.**  `git mv src/csrf.rs src/form_token.rs` and the tests
   directory.  Rename: `CsrfConfig` → `FormTokenConfig` with fields
   `secret_key`, `ttl_secs`, `min_age_secs` (default 2), `binding`
   (`Binding::None`); `CsrfToken` → `FormToken`; `CsrfConfigContext` →
   `FormTokenContext`; `generate_csrf_token` → `issue_form_token`;
   `verify_csrf_token` → `verify_form_token(token, bound_value: Option<&str>, config) -> Result<(), FormTokenError>`
   with the check order from RFC D2 (binding checks return `BindingMissing`
   / `BindingMismatch` only when `binding == Cookie`; this handoff never
   sets `Cookie`).  `FormTokenConfig::new(secret)` keeps the defaults;
   builder methods `with_ttl(secs)`, `with_min_age(secs)`, `with_binding(b)`.
   `Debug` redacts the key.
3. **Alias module `src/csrf.rs`.**  `#[deprecated(since = "0.5.0", note = "renamed to `form_token`; removed in the next minor")]`
   on: `pub type CsrfConfig = FormTokenConfig; pub type CsrfToken = FormToken; pub type CsrfConfigContext = FormTokenContext;`
   `pub fn generate_csrf_token(&FormTokenConfig) -> FormToken` and
   `pub fn verify_csrf_token(&str, &FormTokenConfig) -> bool` (calls the
   new function with `None` and maps `Ok` → `true`).  Note: `CsrfConfig { secret_key, token_ttl_secs }`
   struct literals from 0.4 will not compile (field rename plus new
   fields); the CHANGELOG migration says to use `FormTokenConfig::new`.
4. **Component.**  Hidden input `name="form_token"`.  Nothing else.
5. **Server function.**  Arguments `form_token: Option<String>` and,
   kept for one minor, `csrf_token: Option<String>` (rustdoc
   `Deprecated`).  Token = `form_token.or(csrf_token)`.  Mapping:
   `Err(TooYoung)` → `Args(ContactErrorCode::TooFast…)`; every other
   `Err` → `token_invalid`; `warn!` logs the variant name via `{:?}`.
6. **Codes and labels.**  `ContactErrorCode::TooFast` (`"too_fast"`);
   `ContactErrorLabels.too_fast = "Please wait a moment and try again."`.
7. **Examples.**  New names; env var `FORM_TOKEN_SECRET`; the startup
   `expect` messages updated.
8. **Docs.**  `git mv docs/src/security/csrf.md docs/src/security/form-token.md`;
   rewrite per RFC D5 (guarantees table with binding on/off rows, minimum
   age, migration from 0.3/0.4 names); `SUMMARY.md`; add to `book.toml`
   `[output.html.redirect] "/security/csrf.html" = "form-token.html"`;
   update Quick Start, Production Checklist, Axum Integration, FAQ, API,
   Feature Flags, Security overview, Troubleshooting; `CHANGELOG.md`
   Unreleased with a **Migration** subsection listing every old → new
   name.

9. **`rand` 0.9.**  While `form_token.rs` is being rewritten, move the
   crate's optional `rand` dependency from 0.8 to 0.9 (`rand::rng()`,
   `RngCore::fill_bytes`), so the lock file no longer carries two `rand`
   lines (the dev team noted 0.8.6 beside a transitive 0.9.4 on
   2026-09-12).  `hmac` 0.13 / `sha2` 0.11 stay: the only `sha2` 0.10 in
   the tree comes from a proc-macro dependency of Leptos and cannot be
   unified from our side.

## Required tests

`form_token/tests.rs`: every `FormTokenError` variant reachable from the
public function (`Malformed`, `BadSignature`, `Expired`, `FromFuture`,
`TooYoung` at `now - 1` with `min_age 2`, pass at `now - 2`); default
config has `min_age_secs == 2`; `with_min_age(0)` disables the check.
`csrf/tests.rs` (new, `#[allow(deprecated)]`): the aliases compile,
`generate_csrf_token` + `verify_csrf_token` round-trip is `true`.
`components/tests.rs`: hidden input is `name="form_token"`.
`server/tests.rs`: the `too_fast` message has the `contact_error:` prefix.

## Required documentation updates

Listed in item 8.

## Acceptance criteria

- Tests pass; gates green with **and without** `-F csrf` (`cargo test -F csrf`
  must also pass, proving the alias feature).
- Hydrated example: submit within two seconds of load → "wait a moment"
  banner; retry after two seconds → success.  Record a short GIF.
- A 0.4 form (hidden `csrf_token`) posted with curl is still accepted.

## Prohibited shortcuts

Deleting the old names outright; keeping `csrf` as the primary name in
docs; making `too_fast` a silent success.

## Compatibility and security constraints

Per RFC 004.  Deprecations warn, never break, in 0.5.

## Known risks

`#[deprecated]` on a `pub type` alias warns at use sites in recent
compilers; if a use site inside the crate (for example the component)
still uses an old name, clippy `-D warnings` fails: use the new names
internally everywhere.

## Required evidence

Gate outputs for both feature sets; test results; GIF; curl transcript.
