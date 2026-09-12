# RFC 002 — Form state model

**Status.** Accepted — proposed and accepted by the owner on 2026-09-12.
**Handoffs.** [`../handoffs/002-form-state-model/README.md`](../handoffs/002-form-state-model/README.md)
**Tracks.** Roadmap M2 items P-10 (input preserved on error), P-11 (token
survives re-render), P-13 (no-JS success signal), P-16 (focus after
error).  Requirements FR-UI-04, FR-UI-06, FR-UI-07, FR-UI-12, FR-UI-13,
FR-PE-03, FR-A11Y-03.  External Design §4.1.3, §4.1.4.
**Touches.** `components.rs`, `error.rs` (one derive), `config.rs` (one
option, one new type), `server.rs` (success path), `axum_helpers.rs`
(one helper), `examples/axum-with-security` (becomes a hydrated example),
documentation.

## Summary

`ContactForm` rebuilds its entire form subtree whenever the server
action's value changes.  That single fact causes three visible defects:
typed input is discarded after a validation error, the hidden form token
is emptied on the client, and there is no clean place to move focus.  This
RFC changes the component so the form is built once and only the error and
success regions react.  It also gives the no-JavaScript path a success
signal by letting the integrator configure a success page to which the
server function redirects.

## Motivation

Observed in `0.3.3` (`components.rs`): the closure
`move || (!succeeded()).then(|| view! { <ActionForm> … })` reads the
action value through `succeeded()` and `field_errors()`, so Leptos re-runs
it on every value change and creates new DOM.  Consequences:

1. **P-10.**  New `<input>` elements have no value; the visitor retypes
   everything after a single validation error.  Violates FR-UI-07 (MUST).
2. **P-11.**  The hidden `csrf_token` input is recreated with the value
   read from `use_context::<CsrfToken>()`, which is `None` in the browser,
   so the next submit fails with "reload the page".  Violates FR-UI-12.
3. **P-16.**  Focus cannot be managed on elements that are about to be
   replaced.
4. **P-13.**  Without JavaScript, server_fn answers a form POST with a 302
   to the Referer; errors travel in the `__err` query parameter and are
   rendered by the server-side action value (verified in `leptos_server`
   `action.rs`), but success carries no signal at all.  Violates FR-PE-03.

## Goals

- Inputs and the hidden token are created once per page render and never
  rebuilt by a result change.
- Field errors, the generic banner, `aria-invalid`, `aria-describedby`,
  and the busy state react in place.
- After a failed submission focus moves to the first invalid input
  (opt-out available).
- A configured success page receives the visitor after a successful
  submission, in both JavaScript and no-JavaScript modes.
- No change to the DOM contract (ids, names, class hooks, ARIA).

## Non-goals

- Client-side-navigation token acquisition (a form created in the browser
  without SSR has no token).  Deferred to RFC 004 with the token redesign;
  recorded as a known limitation.
- Preserving input across the **no-JS** error reload.  The framework
  reloads the page; values would have to travel in the URL, which puts
  PII in logs.  Not done.
- Error codes and localisation of server messages (RFC 003).
- Any change to validation, policy, delivery, or the wire format.

## Design

### D1 — Build once, react in place

```text
<div root>
  {success region}         reactive on `succeeded` (Memo<bool>)
  {generic error region}   reactive on `generic_error` (Memo<Option<String>>)
  {move || (!succeeded.get()).then(form_view)}   re-runs only when the memo flips
     <ActionForm>
        inputs …           static; never recreated by a result change
        aria-invalid=      reactive per field on `field_errors` (Memo<ContactFieldErrors>)
        aria-describedby=  reactive per field
        <FieldError message=Signal<Option<String>> />   reactive
        <input type=hidden name=csrf_token value=…>     static, from SSR context
        honeypot           static
        <button disabled=pending aria-busy=…>            reactive (already)
```

- `succeeded: Memo<bool>` derived from the action value.  A `Memo` only
  notifies when its value changes, so the form closure runs once at
  creation and once more when success arrives.  Result changes that stay
  `false` (errors) do not touch it.
- `field_errors: Memo<ContactFieldErrors>` derived from the action value
  via `ContactFieldErrors::from_server_fn_error` (introduced by RFC 001
  handoff 03).  `ContactFieldErrors` gains `PartialEq` so it can be a
  `Memo` value.
- `generic_error: Memo<Option<String>>`: `Some(labels.error)` when the
  value is an error that carries no field payload.
- `FieldError` takes `message: Signal<Option<String>>` and renders
  `{move || message.get().map(|m| view! { <p …>{m}</p> })}`.  Its ids and
  attributes are unchanged.
- Each input's `aria-invalid` and `aria-describedby` are closures over the
  memo returning `Option<&'static str>`.

Hydration: the hidden token's `value` remains a plain string attribute.
tachys does not rewrite static attributes when hydrating from
server-rendered HTML (`tachys` `html/attribute/value.rs`, `hydrate` with
`FROM_SERVER = true`), so the SSR token stays in the DOM.  Because the form
is never rebuilt, it also survives every later result change.  This closes
P-11 for the SSR-then-hydrate case.

Errors stay visible until the next submit; editing a field does not clear
them.  This is the simplest rule and matches the no-JS behaviour.

### D2 — Focus after error

`ContactFormOptions` gains `focus_first_error: bool` (default `true`).
When set, a client-side `Effect` watching `field_errors` focuses the first
element among `contact-name`, `contact-email`, `contact-subject`,
`contact-message` whose error is `Some`, using `document()` and
`web_sys::HtmlElement::focus` through Leptos' re-exports.  Effects do not
run during SSR, so no server cost.  When the payload is empty the effect
does nothing.

### D3 — Success page

New type in `config`:

```rust
pub struct ContactSuccessRedirect { path: String, redirect: Arc<dyn Fn(&str) + Send + Sync> }
impl ContactSuccessRedirect {
    /// `path` must be site-relative: start with `/`, not `//`, no scheme.
    pub fn new(path: impl Into<String>, redirect: impl Fn(&str) + Send + Sync + 'static)
        -> Result<Self, InvalidRedirectPath>;
    pub fn path(&self) -> &str;
}
```

Provided through Leptos context in the **server-function handler** (the
SSR renderer does not need it).  On success, before returning `Ok(())`,
`submit_contact` calls `redirect(path)` if the context is present.

`axum_helpers::success_redirect(path) -> ContactSuccessRedirect` supplies
`leptos_axum::redirect` as the executor and panics on an invalid path
(startup misconfiguration).  Other integrations pass their framework's
redirect function.

Behaviour, verified against `server_fn` 0.8.13 `lib.rs` and `leptos_axum`
0.8.9 `lib.rs`:

| Client | Framework behaviour | Result |
|--------|--------------------|--------|
| `<form>` POST (`Accept: text/html`) | server_fn sets `302 Location: <Referer>`; leptos_axum then applies `ResponseOptions`, whose `Location` replaces it | browser lands on the success page |
| WASM `fetch` | `leptos_axum::redirect` sets the client redirect header instead of a 302; `ActionForm`'s redirect hook navigates | client navigates to the success page |

Rule: **when a success redirect is configured, every successful submission
goes there and the inline success message is not shown; when none is
configured, JS clients see the inline message and no-JS clients see the
current reload.**  One rule, both modes consistent.  The documentation
states the limitation of the unconfigured no-JS case plainly.

Path validation prevents open redirects: absolute URLs, protocol-relative
`//host`, and paths containing `\` or control characters are rejected.

### D4 — A hydrated example

P-10 and P-11 are hydration defects.  Neither example ships a client
bundle, so nothing in the repository can demonstrate them or their fix.
`examples/axum-with-security` becomes a `cargo-leptos` project with a
`hydrate` library target and `[package.metadata.leptos]`.  The CI
`examples` job keeps `cargo check` for the server target; a full
`cargo leptos build` in CI is optional and left to the handoff to decide
based on build time.

## Amendment 2026-09-12 — observations from the first hydrated build (handoff 01)

The hydrated example (handoff 01) corrected two assumptions in
§Motivation and added one defect:

1. **Typed input survives** a failed submission.  tachys rebuilds the
   re-run closure's output *in place*, keeping the existing input elements
   and their DOM values.  P-10 as originally stated ("input discarded") is
   withdrawn.  D1 stands: the same in-place rebuild rewrites *attributes*,
   which is exactly how the hidden token is emptied (P-11, confirmed:
   `tokenAfter: ""`).
2. **A report that per-field errors do not render under hydration** was
   investigated by the architect and **not reproduced** on a clean client
   build: the decode path was verified natively, and both an instrumented
   and a clean hydrated run rendered the field error with no banner.
   Attributed to a stale WASM bundle (roadmap P-27, withdrawn).  Handoff 02
   starts with a clean rebuild and keeps "field errors render under
   hydration" as a regression guard.
3. Browser tests of server-side validation must set `form.noValidate`
   or use inputs the browser accepts, otherwise native validation blocks
   the submit and the server is never reached.

Goals and acceptance criteria are unchanged except that "inputs
preserved" and "field errors render under hydration" are regression guards
rather than fixes.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| Keep one closure, stash typed values in signals, restore them after rebuild | Doubles the state, still recreates DOM, still loses the token, fragile |
| Controlled inputs bound to `RwSignal<String>` each | Larger change, changes the no-JS story (values would be rendered from signals), no benefit over D1 |
| Success marker `?sent=1` read at SSR | The component would have to read the request URL, which needs a router or framework type in the core; D3 keeps the core free of URL handling |
| Success cookie | Needs response access from the core; same issue, plus cookie consent noise |
| Fetch a token from a server function when the client has none | Right idea, wrong RFC: it changes what the token means; RFC 004 |

## Compatibility

Minor release.  Additions: `ContactFormOptions::focus_first_error`
(struct literal users must add the field or use `..Default::default()`;
this is the one source-compatibility break and is the reason this is a
minor, not a patch), `ContactSuccessRedirect`, `InvalidRedirectPath`,
`axum_helpers::success_redirect`, `PartialEq` on `ContactFieldErrors`,
`FieldError` prop type change (private component).  DOM contract
unchanged.  Wire format unchanged.

## Security considerations

- Redirect target is validated as site-relative; no open redirect.
- The token is preserved, not re-issued; no change to its properties.
- Focus management touches only elements the crate rendered.
- Threat model: no new row; T5/T6 unchanged.

## Operational considerations

None on the server.  The hydrated example adds `cargo-leptos` as a
development tool for the example only.

## Testing and verification

- Unit: `ContactSuccessRedirect::new` accepts `/thanks`, `/a/b?x=1`;
  rejects `https://evil.test`, `//evil.test`, `/a\b`, empty.
- Unit: `ContactFieldErrors: PartialEq` round-trip equality.
- SSR render test (`leptos::ssr::render_to_string` under `ssr`): the form
  renders once with all ids; with `CsrfToken` in context the hidden input
  carries the token.
- Browser evidence on the hydrated example, recorded as a short GIF or a
  numbered screenshot series in the review request:
  1. type values, submit with an invalid email → field error under email,
     other values still present, focus on the email input;
  2. correct the email, submit → success (inline or redirect per config)
     without "reload the page";
  3. with JavaScript disabled, submit invalid → errors after reload;
     submit valid with a success redirect configured → success page.
- Gates green.

## Acceptance criteria

- FR-UI-07, FR-UI-12 (SSR+hydrate case), FR-UI-13, FR-PE-03 (configured
  case) demonstrably met on the hydrated example.
- No change to the DOM contract table in External Design §4.1.2 except
  the note that `FieldError` content is reactive.
- Documentation updated: Customization (new option, success redirect),
  Axum Integration (helper), FAQ no-JS answer, Troubleshooting known
  issues rows P-10, P-11, P-13, P-16 removed at release, External Design
  §4.1.3–4.1.4 marked current.

## Implementation boundaries

Three handoffs are expected: (1) component state model and focus, (2)
success redirect, (3) hydrated example and browser evidence.  Handoff 3
can start in parallel with 1.

## Open questions

None blocking.  The CSR-navigation token case is recorded for RFC 004.

## Release implications

Part of the proposed `0.4.0`.
