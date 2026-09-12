# Handoff 02 — Cookie binding in `axum-helpers`

**RFC.** [004](../../accepted/004-form-token.md), D3.
**Requirements.** FR-ABUSE-02, FR-ABUSE-04.  **Depends on.** Handoff 01.

## Purpose

Give Axum users a genuine double-submit CSRF property by binding the
token to an `HttpOnly`, `SameSite=Lax` cookie.

## Change scope

`form_token.rs` (`FormTokenBinding` context, binding checks already
present), `server.rs` (pass the bound value), `axum_helpers.rs` (+ tests),
`Cargo.toml` (`axum-helpers` may add `dep:http` if not already
transitively available), `examples/axum-with-security`, docs.

## Explicit non-change scope

Core stays free of cookie parsing; no session concept; token format.

## Required implementation

0. **First edit (added by the handoff 01 review).**  Delete the paragraph on
   `Binding::Cookie` saying the helpers do not exist yet; this handoff makes
   it untrue.  Nothing else in that rustdoc changes.


1. **Core.**  `pub struct FormTokenBinding(pub Option<String>);` in
   `form_token.rs`.  In `submit_contact`, when the config has
   `Binding::Cookie`, read `use_context::<FormTokenBinding>()` and pass
   its inner value (or `None` when the context is absent) to
   `verify_form_token`.
2. **`axum_helpers.rs`.**
   - `pub struct FormTokenCookie { pub name: String, pub secure: bool, pub path: String }`
     with `Default` = `("hl_contact_ft", true, "/")`.
   - `pub fn provide_form_token_with_cookie(config: &FormTokenContext, cookie: &FormTokenCookie)`:
     read `http::request::Parts` from context; **only when the method is
     `GET`** (a page render): `issue_form_token`, `provide_context(FormToken)`,
     and append a `Set-Cookie` header to `leptos_axum::ResponseOptions`
     with value = the token's nonce (the middle segment) and attributes
     `HttpOnly; SameSite=Lax; Path=<path>; Max-Age=<ttl_secs>` plus
     `Secure` when `cookie.secure`.  Use `append_header`, not `insert`,
     so other cookies survive.  On any other method do nothing: one
     closure serves both page renders and server-function calls (RFC 007),
     and a POST response must not overwrite the cookie the form carries.
   - `pub fn provide_form_token_binding(cookie: &FormTokenCookie)`: read
     `http::request::Parts` from context, parse the `Cookie` header for
     `cookie.name`, `provide_context(FormTokenBinding(value))`.  Absent
     parts or cookie → `FormTokenBinding(None)`.
   - A private `fn cookie_value(header: &str, name: &str) -> Option<String>`
     for parsing (split on `;`, trim, exact name match, first wins).
3. **Example.**  `FormTokenConfig::new(secret).with_binding(Binding::Cookie)`;
   the single context closure calls both `provide_form_token_with_cookie`
   and `provide_form_token_binding`; `secure` from env
   `FORM_TOKEN_COOKIE_SECURE` defaulting to `true` (set `false` for local
   HTTP).
4. **Docs.**  `security/form-token.md` binding section becomes current;
   `guides/axum-integration.md` "All context values together" updated;
   Production Checklist row for the token now says "binding on".

## Correction required by review (2026-09-13)

`provide_form_token_with_cookie` must reuse the nonce the browser already
has: read the request cookie first, and when it is present and well formed
(32 hex characters) issue the token bound to that nonce, re-sending
`Set-Cookie` so `Max-Age` refreshes.  Only a request without a usable cookie
mints a new one.  Rationale, required tests and required evidence are in
`.git-exclude/reviewed/004-form-token/02-cookie-binding.md`.

## Corrections C2 and C3 required by review (2026-09-13)

C2: apply the `__Host-` cookie-name prefix when `secure` is true and `path`
is `/`, deriving the effective name in one place used by both helpers, with
the tests and the documentation sentences named in
`.git-exclude/reviewed/004-form-token/02-cookie-binding-c1.md`.
C3: add `RUSTDOCFLAGS: -D warnings` to the `doc` step in CI.

## Required tests

`axum_helpers/tests.rs`: `cookie_value` finds the value among several
cookies, ignores prefix matches (`hl_contact_ft2`), handles missing;
`provide_form_token_with_cookie` is a no-op for a POST (test with a
constructed `Parts`); `Set-Cookie` string builder produces exactly
`hl_contact_ft=<nonce>; HttpOnly; SameSite=Lax; Path=/; Max-Age=3600; Secure`
(factor the string building into a pure function to test it).
`form_token/tests.rs`: with `Binding::Cookie`, `None` → `BindingMissing`,
wrong value → `BindingMismatch`, nonce → `Ok`.

## Acceptance criteria

- Tests pass; gates green.
- Hydrated example with binding on: normal submit succeeds; the browser
  shows the cookie with `HttpOnly` and `SameSite=Lax`.
- curl: fetch `/` capturing the cookie and token; POST with the token but
  **without** the cookie → `token_invalid`; POST with both → success.
  Paste both transcripts.

## Prohibited shortcuts

Putting the secret or the whole token in the cookie; `SameSite=None`;
parsing cookies with a regex that accepts prefixes.

## Compatibility and security constraints

Additive; binding is off by default.  Cookie is not readable by scripts.
Multi-instance deployments already share the secret; the nonce needs no
server state.

## Known risks

Reverse proxies that strip `Set-Cookie` on cached pages: the contact page
must not be cached; add a sentence to the docs.

## Required evidence

Gate outputs; tests; browser cookie screenshot; the two curl transcripts.
