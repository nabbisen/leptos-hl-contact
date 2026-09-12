# Handoff 03 — Client-side acquisition and refresh

**RFC.** [004](../../accepted/004-form-token.md), D4.
**Requirements.** FR-UI-12 (client-side navigation case).
**Depends on.** Handoffs 01 and 02.

## Purpose

A form created in the browser without an SSR token gets one; a form left
open past the TTL keeps a valid one.

## Change scope

`form_token.rs` (server function, `FormTokenIssuer` context),
`components.rs` (signal, `NodeRef`, effects), `axum_helpers.rs` (issuer
that sets the cookie), example (second route), docs.

## Explicit non-change scope

Token format and verification; the no-JS path (always SSR, always has a
token).

## Required implementation

1. **Server function.**  In `form_token.rs`, feature `form-token`:
   `#[server(endpoint = "form_token")] pub async fn issue_form_token_fn() -> Result<String, ServerFnError>`.
   Reads `FormTokenContext` (absent → `ServerError(not_configured)`),
   issues a token, calls `use_context::<FormTokenIssuer>()` if present
   with the token, returns the string.
   `pub struct FormTokenIssuer(pub Arc<dyn Fn(&FormToken) + Send + Sync>);`.
2. **`axum_helpers.rs`.**  `pub fn provide_form_token_issuer(cookie: &FormTokenCookie)`:
   provides a `FormTokenIssuer` that appends the same `Set-Cookie` as
   handoff 02 (reuse the pure builder).  Call it in the example's single
   context closure next to `provide_form_token_binding` (RFC 007).
3. **Component.**
   - `let token = RwSignal::new(initial_from_context_or_empty)`; hidden
     input `value=move || token.get()` and `node_ref=token_ref`.
   - `#[cfg(feature = "hydrate")]` `Effect::new`: read
     `token_ref.get().map(|el| el.value())`; if the DOM value is empty,
     dispatch `issue_form_token_fn` (via `ServerAction` or `Resource`,
     your choice, note it) and on `Ok(t)` call `token.set(t)`.
   - Refresh: when the acquired or SSR token's timestamp (first `|`
     segment) plus `ttl - 60` seconds is in the future and `ttl > 120`,
     schedule `set_timeout` to re-issue and `token.set`.  The TTL is not
     known on the client: pass it through a new optional
     `ContactFormOptions::token_refresh_secs: Option<u64>` (default
     `Some(3540)`), documented as "should be `ttl_secs - 60`".
4. **Example.**  Add a `/` landing page with a client-side `<A href="/contact">`
   link and move the form to `/contact`, so client-side navigation to the
   form can be exercised.
5. **Docs.**  `security/form-token.md`: acquisition and refresh; API page:
   `issue_form_token_fn`, `FormTokenIssuer`, `provide_form_token_issuer`,
   the new option; Customization: the option.

## Amendment from the handoff 02 reviews (2026-09-13)

- **Reuse the browser's nonce.**  `issue_form_token_fn` is a POST, so the GET
  gate in `provide_form_token_with_cookie` does not apply.  If it minted a
  fresh nonce and re-set the cookie, every tab that fetched a token would
  invalidate the others — the defect corrected in handoff 02.  The issuer
  must read the request cookie under `effective_name` and use
  `issue_form_token_with_nonce` when it is usable, minting only otherwise,
  and must write the cookie through `set_cookie_value` so the `__Host-`
  prefix is applied.  Add a test: a request carrying a valid cookie gets a
  token with that nonce.  Evidence: render the form in one jar, fetch a token
  client-side in the same jar, then submit the *rendered* token and show it
  accepted.
- **Link caching to the binding limits.**  In `security/form-token.md`, one
  sentence under "What binding cannot stop" pointing at the caching callout:
  a cached page pairs one visitor's token with another visitor's cookie.
- Any new client-side effect uses the `all(feature = "hydrate", not(feature
  = "ssr"))` gate established in RFC 002 handoff 02.

## Corrections C1–C3 required by review (2026-09-13)

C1: `token_refresh_secs` defaults to `None` and gates acquisition as well as
refresh.  C2: the mounted token refreshes once immediately if overdue; fetched
tokens are scheduled from their arrival on the client clock.  C3: CHANGELOG
notes the two new `ContactFormOptions` fields and the off-by-default switch.
Details, required evidence (including a clock-skew loop guard) in
`.git-exclude/reviewed/004-form-token/03-client-acquisition.md`.

## Correction C4 required by review (2026-09-13)

Cap `mounted_refresh_delay` at `refresh`; rename the slow-clock test and
expect the interval; remove the documented slow-clock residual.  Details in
`.git-exclude/reviewed/004-form-token/03-client-acquisition-c1-c3.md`.

## Required tests

`components/tests.rs` (SSR): the hidden input still carries the SSR token
value with the reactive attribute.  `form_token/tests.rs`: timestamp
extraction helper used by the refresh logic.  Unit tests for the option
default.

## Acceptance criteria

- Tests pass; gates green.
- Hydrated example, binding on:
  1. load `/`, click the link, submit valid data → success (token was
     acquired client-side; network panel shows `POST /api/form_token`);
  2. load `/contact` directly, submit → success without any `form_token`
     request (SSR token used; proves the DOM check);
  3. set `ttl 130`, `token_refresh_secs 70`, open `/contact`, wait 80 s,
     submit → success (refresh happened; network panel shows the call).
  Record as GIFs or screenshot series with the network panel visible.

## Prohibited shortcuts

Detecting hydration via global flags; always fetching a token on mount
(doubles requests and breaks the no-cookie case); storing tokens in
`localStorage`.

## Compatibility and security constraints

Additive.  The issuing endpoint is covered by the integrator's rate
limit; document that in `security/form-token.md`.

## Known risks

Reactive `value` attribute on hydration: tachys must not overwrite the
SSR value on first run (verified in `tachys` `attribute/value.rs` for
`&str` under `FROM_SERVER`); if you observe the field being blanked on
hydration, stop and report with the Leptos version.

## Required evidence

Gate outputs; tests; the three recordings.
