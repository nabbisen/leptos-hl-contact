# RFC 004 — Form token: rename, minimum age, cookie binding, client acquisition

**Status.** Accepted — proposed and accepted by the owner on 2026-09-12,
including the rename, `min_age_secs = 2`, and the hidden-field rename in
0.5.0 with both names accepted server-side for one minor.
**Handoffs.** [`../handoffs/004-form-token/README.md`](../handoffs/004-form-token/README.md)
**Tracks.** Roadmap M3 item P-12 (extended).  Requirements FR-ABUSE-02
(Decision), FR-ABUSE-03, FR-ABUSE-04, FR-ABUSE-13 (minimum age), FR-UI-12
(client-side navigation case).  External Design §5.3, §5.4.
**Touches.** `csrf.rs` → `form_token.rs`, `components.rs`, `server.rs`,
`axum_helpers.rs`, `config.rs`, feature list, documentation, both
examples.

## Summary

The `csrf` feature's token proves that its bearer fetched a page from this
server recently.  It does not prove who the bearer is, so it is not a
CSRF control, and its name says otherwise.  This RFC (1) renames the
feature and its API to **form token** with deprecated aliases for one
minor release, (2) adds a **minimum age** check so submissions faster than
a human can type are rejected, (3) adds optional **cookie binding** in
`axum-helpers`, which turns the token into a genuine double-submit CSRF
control for Axum users, and (4) lets a form that was created in the
browser without an SSR token **acquire one** from a server function, so
client-side navigation to the contact page works.

## Motivation

- Truthfulness: the current name invites integrators to skip Origin
  validation.  External Design §5.4 documents the gap; the docs already
  say "anti-automation token".
- Bot friction at zero cost: the token already carries its issue time; a
  submission two seconds after render is not a person.
- Correctness: a form created by client-side routing has no token and
  every submit fails until reload (found during RFC 002).
- Real CSRF for the common case: Axum users can have the double-submit
  property without sessions.

## Goals

- Names describe guarantees.
- Minimum age enforced with a retryable error, default two seconds.
- Optional cookie binding; when on, a submission without the matching
  cookie is rejected.
- Client-side token acquisition and pre-expiry refresh.
- Fail-closed everywhere; one-minor deprecation path for the old names.

## Non-goals

- Session binding (integrators with sessions can implement `Binding`
  themselves later; not in this RFC).
- Single-use tokens (would need server state).
- Challenge providers (RFC 005).

## Design

### D1 — Rename with aliases

| Old | New |
|-----|-----|
| feature `csrf` | `form-token`; `csrf = ["form-token"]` kept one minor, documented as deprecated |
| module `csrf` | `form_token`; `pub mod csrf` re-exports with `#[deprecated]` |
| `CsrfConfig` / `CsrfToken` / `CsrfConfigContext` | `FormTokenConfig` / `FormToken` / `FormTokenContext` (+ deprecated type aliases) |
| `generate_csrf_token` / `verify_csrf_token` | `issue_form_token` / `verify_form_token` (+ deprecated fns) |
| hidden field `csrf_token` | `form_token`; `submit_contact` accepts both `form_token: Option<String>` and `csrf_token: Option<String>` (deprecated) and uses whichever is present |
| env var in docs `CSRF_SECRET` | `FORM_TOKEN_SECRET` (docs and examples only) |

The DOM contract changes (field name); this is the breaking change that
makes the release a minor.

### D2 — Token format and verification

Format unchanged: `{unix_seconds}|{nonce_hex}|{hmac_hex}`.

```rust
pub struct FormTokenConfig {
    pub secret_key: Vec<u8>,
    pub ttl_secs: u64,        // default 3600
    pub min_age_secs: u64,    // default 2; 0 disables
    pub binding: Binding,     // default Binding::None
}
pub enum Binding { None, Cookie }

pub enum FormTokenError { Malformed, BadSignature, Expired, FromFuture, TooYoung, BindingMissing, BindingMismatch }

pub fn verify_form_token(token: &str, bound_value: Option<&str>, config: &FormTokenConfig) -> Result<(), FormTokenError>;
```

Checks, in order: format, timestamp parse, future skew (60 s), expiry,
**minimum age** (`now - ts < min_age_secs` → `TooYoung`), HMAC (constant
time), binding (`Binding::Cookie`: `bound_value` must equal the token's
nonce; `None` → `BindingMissing`, different → `BindingMismatch`).

Server mapping (with RFC 003 codes): `TooYoung` → `Args("contact_error:too_fast")`
(new `ContactErrorCode::TooFast`, label "Please wait a moment and try
again."); everything else → `token_invalid`.  `TooYoung` is retryable by
design: the same token becomes valid two seconds later, so a person who
autofilled and clicked fast is not punished, only delayed.

Logs: `warn` with the error variant name, never the token.

### D3 — Cookie binding (`axum-helpers`)

```rust
pub struct FormTokenCookie { pub name: String /* "hl_contact_ft" */, pub secure: bool, pub path: String /* "/" */ }

/// SSR renderer closure: issue token, provide `FormToken`, set the cookie.
pub fn provide_form_token_with_cookie(config: &FormTokenContext, cookie: &FormTokenCookie);
/// Server-function handler closure: read the cookie and provide `FormTokenBinding`.
pub fn provide_form_token_binding(cookie: &FormTokenCookie);
```

Cookie value = the token's nonce; attributes `HttpOnly; SameSite=Lax;
Path=<path>; Max-Age=<ttl>; Secure` (Secure on by default; the example
sets it from an env var for local HTTP).  Set through
`leptos_axum::ResponseOptions`; read through `http::request::Parts`.

Core: `pub struct FormTokenBinding(pub Option<String>);` context.  When
`config.binding == Cookie`, `submit_contact` reads it and passes the value
to `verify_form_token`; when the context is absent it passes `None`,
which fails closed.

Property gained: a cross-site form cannot present the victim's cookie
value in the field; with `SameSite=Lax` the cookie is not even sent on a
cross-site POST.  Together with the Origin check this is defence in depth.
Property not gained: a scripted client that fetches the page can still
read its own cookie; the minimum age and rate limiting cover that.

### D4 — Client-side acquisition and refresh

- New server function `#[server(endpoint = "form_token")] pub async fn issue_form_token_fn() -> Result<String, ServerFnError>`
  (feature `form-token`): issues a token; if a `FormTokenIssuer` context
  is present (provided by `provide_form_token_binding`'s sibling in the
  server-function closure for Axum), it also sets the binding cookie.
- Component: the hidden input's `value` becomes a signal initialised from
  the SSR context (or empty).  A `NodeRef` on the input and a client-side
  `Effect` check the **DOM** value after mount: non-empty → hydrated from
  SSR, nothing to do; empty → dispatch the server function and set the
  signal.  Reading the DOM instead of asking "am I hydrating" avoids
  hydration APIs and works for both cases.  tachys does not overwrite the
  SSR attribute on first hydration of a reactive attribute, so the SSR
  token is kept until the signal changes.
- Refresh: a client-side timer at `ttl_secs - 60` re-issues the token
  while the page stays open, so a slow writer never sees "expired".
  Disabled when `ttl_secs ≤ 120`.
- The issuing endpoint is public and rate-limited by the integrator's
  layer like every other route.  It grants nothing the page itself did
  not.

### D5 — Documentation

`security/csrf.md` → `security/form-token.md` with the guarantees table
rewritten (binding on / off), the minimum age, and a migration section
from 0.3 names.  Production Checklist, FAQ, API, Feature Flags updated.
Examples switched to the new names, `axum-with-security` uses cookie
binding.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Keep the `csrf` name, document the gap | Name still claims a property the default does not have; the docs already had to say otherwise |
| Remove the token | Loses free bot friction and the minimum-age signal |
| Session binding in core | The crate has no session concept; would force one on integrators |
| Encrypted timestamp cookie instead of nonce | Nonce is already random and signed; simpler |
| Detect hydration via `SharedContext::during_hydration` | DOM check is simpler and tolerant of Islands and CSR |

## Compatibility

Minor, with a one-minor deprecation window: old feature name, module,
types, functions, and the `csrf_token` field all keep working with
deprecation warnings.  Integrators who do nothing keep working (binding
off by default, minimum age two seconds — the one behavioural change,
documented).  DOM contract: field renamed; both accepted server-side.

## Security considerations

- Cookie binding off by default keeps the framework-neutral core simple;
  the docs steer Axum users to turn it on.
- Minimum age two seconds: a real person with autofill can hit it; the
  retryable code and label handle that.  Owner may lower the default.
- The issuing endpoint: rate limited by the app layer; tokens it issues
  are equivalent to page-fetched tokens.
- Threat model rows T5 (CSRF) and T6 (replay) updated: T5 mitigated by
  binding + Origin; T6 bounded by TTL, minimum age, rate limit.
- Secret handling unchanged; `Debug` redaction kept under the new names.

## Operational considerations

One new endpoint; one cookie; both documented.  Multi-instance
deployments need the same secret on all instances (already true).

## Testing

- Unit: every `FormTokenError` path, including `TooYoung` at `min_age - 1`
  and pass at `min_age`; binding match / missing / mismatch.
- Unit: deprecated aliases compile and resolve (a test using the old
  names under `#[allow(deprecated)]`).
- SSR test: hidden input named `form_token` with the SSR value.
- Browser evidence on the hydrated example: client-side navigate to the
  form (add a second route in the example) → submit works; wait past the
  refresh timer (use a short TTL in dev) → submit still works; submit
  within two seconds → "wait a moment" then success on retry; cross-site
  POST via curl without the cookie → rejected with binding on.

## Acceptance criteria

FR-ABUSE-02 resolved (guarantees named truthfully; binding available),
FR-ABUSE-13 Met, FR-UI-12 Met including client-side navigation.

## Implementation boundaries

Three handoffs: (1) rename with aliases and minimum age; (2) cookie
binding in `axum-helpers`; (3) client acquisition and refresh.  (2) and
(3) are independent after (1).

## Owner decisions

1. Default `min_age_secs` is **2** (decided 2026-09-12).
2. The hidden-field rename `csrf_token` → `form_token` in 0.5.0 is
   **accepted**, both names accepted server-side for one minor.

## Release implications

Proposed `0.5.0` (first M3 release).  The deprecated names are removed in
the following minor.
