# Handoff 02 — Build once, react in place, focus after error

**RFC.** [002](../../done/002-form-state-model.md), designs D1, D2.
**Requirements.** FR-UI-04, FR-UI-07, FR-UI-12, FR-UI-13, FR-A11Y-03.
**External Design.** §4.1.2 DOM contract (must not change), §4.1.3 state model.
**Depends on.** RFC 001 handoff 03; handoff 01 of this RFC for browser evidence.

## Purpose

Stop rebuilding the form when the action value changes; make only the
error and success regions reactive; focus the first invalid field.

## Step 0 — clean baseline (added by review of handoff 01)

Before changing anything: in `examples/axum-with-security` run
`rm -rf target/front target/site && cargo leptos build`, serve it, and
confirm in the hydrated browser that an invalid submit renders the field
error, sets `aria-invalid`, shows no banner, keeps the typed values, and
**empties the hidden token**.  That is the true 0.3.4 baseline (verified by
the architect three times).  A stale bundle once made it look as if field
errors did not render; never trust a bundle you did not just build.
Browser tests of server-side validation must set `form.noValidate` first.

## Change scope

- `src/components.rs`, new `src/components/tests.rs`
- `src/error.rs` (`PartialEq` derive only)
- `src/config.rs`, `src/config/tests.rs` (one new option)
- `docs/src/guides/customization.md` (new option), `docs/src/guides/accessibility.md`
  (focus behaviour), `docs/src/development/external-design.md` §4.1.3
  table statuses, `CHANGELOG.md` Unreleased.

## Explicit non-change scope

- Element ids, `name` attributes, class hooks, ARIA attribute names and
  values, the honeypot markup, the hidden token field name.
- `server.rs`, validation, wire format.
- No-JS behaviour (the page still reloads; values are lost there).
- Do not clear errors when the visitor edits a field.

## Required implementation

1. **`error.rs`.**  Add `PartialEq` to the derives of `ContactFieldErrors`.
2. **`config.rs`.**  `ContactFormOptions` gains
   `pub focus_first_error: bool` (default `true`) with rustdoc: "After a
   failed submission, move keyboard focus to the first invalid input.
   Client-side only."  Update the `Default` impl and the `ContactFormOptions`
   rustdoc example if it lists all fields.
3. **`components.rs`.**  Restructure `ContactForm` exactly as follows.

   ```rust
   let submit_action = ServerAction::<SubmitContact>::new();
   let pending = submit_action.pending();
   let value   = submit_action.value();

   // Token: read once at creation (SSR has it; the browser does not, and
   // the SSR-rendered attribute survives hydration because it is static).
   let csrf_token_value: String = /* as today */;

   let succeeded: Memo<bool> = Memo::new(move |_| value.with(|v| matches!(v, Some(Ok(())))));
   let field_errors: Memo<ContactFieldErrors> = Memo::new(move |_| value.with(|v| match v {
       Some(Err(e)) => ContactFieldErrors::from_server_fn_error(e).unwrap_or_default(),
       _ => ContactFieldErrors::default(),
   }));
   let generic_error: Memo<Option<String>> = Memo::new(move |_| value.with(|v| match v {
       Some(Err(e)) if ContactFieldErrors::from_server_fn_error(e).map_or(true, |f| f.is_empty())
           => Some(labels.with_value(|l| l.error.clone())),
       _ => None,
   }));
   ```

   - Success region: `{move || succeeded.get().then(|| view! { <div class=… role="status" aria-live="polite">{…}</div> })}`.
   - Generic error region: `{move || generic_error.get().map(|msg| view! { <div … role="alert" aria-live="assertive">{msg}</div> })}`.
   - Form: `{move || (!succeeded.get()).then(|| view! { <ActionForm action=submit_action> … </ActionForm> })}`.
     **Inside this closure nothing may read `value`, `field_errors`, or
     `generic_error` directly.**  Per-field reactivity goes through small
     closures on the attributes:
     `aria-invalid=move || field_errors.with(|f| f.name.is_some()).then_some("true")`,
     `aria-describedby=move || field_errors.with(|f| f.name.is_some()).then_some("contact-name-error")`,
     and `<FieldError input_id="contact-name" class=… message=Signal::derive(move || field_errors.with(|f| f.name.clone())) />`.
     Same for email, subject, message.
   - `FieldError` prop becomes `message: Signal<Option<String>>`; body:
     `move || message.get().map(|msg| view! { <p id=… class=… role="alert" aria-live="polite">{msg}</p> })`.
     `class` may stay `String`.
   - Hidden token, honeypot, button: unchanged markup.
4. **Focus effect (D2).**  Inside `ContactForm`, gated
   `#[cfg(feature = "hydrate")]`, when `options.focus_first_error`:

   ```rust
   Effect::new(move |_| {
       let id = field_errors.with(|f| {
           [("contact-name", f.name.is_some()), ("contact-email", f.email.is_some()),
            ("contact-subject", f.subject.is_some()), ("contact-message", f.message.is_some())]
           .into_iter().find(|(_, has)| *has).map(|(id, _)| id)
       });
       if let Some(id) = id {
           if let Some(el) = document().get_element_by_id(id) {
               use leptos::wasm_bindgen::JsCast;
               if let Ok(el) = el.dyn_into::<leptos::web_sys::HtmlElement>() { let _ = el.focus(); }
           }
       }
   });
   ```

   Use whatever import paths Leptos 0.8 exposes for `document`, `web_sys`,
   `wasm_bindgen`; do not add direct `web-sys` or `wasm-bindgen`
   dependencies to the crate.

## Required tests

`src/components/tests.rs`, `#[cfg(all(test, feature = "ssr"))]`, using
`leptos::ssr::render_to_string`:

- `form_renders_all_ids_once`: the HTML contains each of `contact-name`,
  `contact-email`, `contact-subject`, `contact-message`, `contact-website`
  exactly once and `name="csrf_token"` once.
- `hidden_token_is_rendered_from_context` (`#[cfg(feature = "csrf")]`):
  with `provide_context(CsrfToken("test-token-value".into()))` inside the
  render closure, the HTML contains `value="test-token-value"`.
- `subject_hidden_when_option_false`: no `contact-subject` in the HTML.

`src/config/tests.rs`: `options_default_focuses_first_error`.

Structural check (not a test): `grep -n "value.with\|field_errors\|generic_error" src/components.rs`
shows no use inside the form closure other than the attribute closures
described above.  Paste it.

## Required documentation updates

- `customization.md`: `focus_first_error` in the options snippet and
  table.
- `accessibility.md`: add a "Focus" row: after a failed submission focus
  moves to the first invalid input; `focus_first_error: false` disables it.
  Remove the 0.3.3 note about per-field errors once RFC 001 handoff 03 is
  merged (it will already be gone if that handoff removed it).
- `external-design.md` §4.1.3 table: field-error row status to "Met (JS);
  no-JS reload loses input by design".
- `CHANGELOG.md` Unreleased: Fixed (token preserved across a failed
  submission), Added (`focus_first_error`).  Do not claim "input
  preserved" as a fix; it already was.
- `development/testing.md`: add the two browser-testing notes — rebuild
  the client bundle (`rm -rf target/front target/site`) before any
  hydrated evidence, and set `form.noValidate` when testing server-side
  validation.
- `help/troubleshooting.md` known issues: remove the P-11 row when this
  lands (the P-10 row was removed by the architect on 2026-09-12 because
  it was false).

## Acceptance criteria

- Tests above pass; gates green.
- Browser evidence on the hydrated example (handoff 01); recorded CDP
  events as in handoff 01's request are the preferred format:
  1. fill all fields with a bad email → error under email, other values
     still present (regression guard), focus on the email input, **hidden
     token still non-empty** (the actual fix);
  2. fix the email, submit → success message, no "reload the page" error
     (`csrf` on);
  3. submit twice with errors → still no token error.
- The DOM contract table in External Design needs no change.

## Prohibited shortcuts

Storing typed values in signals and re-filling inputs; `inner_html`;
reading the token from `localStorage` or a cookie; changing the field
name or ids; adding `web-sys`/`wasm-bindgen` as direct dependencies.

## Module boundaries

`components` only orchestrates rendering; parsing stays in `error`.

## Compatibility constraints

`ContactFormOptions` gains a field (minor, source-compatible via
`..Default::default()`; note it in the CHANGELOG).  `FieldError` is
private.  DOM contract unchanged.

## Security constraints

No new data flow.  Rendered messages are text nodes.

## Known risks

- A `Memo` requires `PartialEq` (step 1) and `Send + Sync` values; strings
  are fine.
- If `Signal::derive` is not available for the `FieldError` prop type in
  your Leptos version, accept `impl Fn() -> Option<String> + Send + Sync + 'static`
  instead and note it.
- `document()` on the server: the effect is cfg-gated to `hydrate`, so it
  is not compiled into the server binary; keep that gate.

## Required evidence

Gate outputs; test names and results; the structural `grep`; the browser
series.
