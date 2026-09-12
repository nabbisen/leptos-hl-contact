# Handoff 03 — Success redirect

**RFC.** [002](../../accepted/002-form-state-model.md), design D3.
**Requirements.** FR-UI-06, FR-PE-03.  **External Design.** §4.1.4.
**Depends on.** Handoffs 01 and 02 of this RFC.

## Purpose

Let the integrator name a success page; send every successful submission
there, with or without JavaScript.

## Change scope

- `src/config.rs`, `src/config/tests.rs`: `ContactSuccessRedirect`, `InvalidRedirectPath`
- `src/server.rs`: success path
- `src/axum_helpers.rs`: `success_redirect`
- `src/lib.rs`: re-exports
- `examples/axum-with-security`: configure `/thanks` and add the page
- Docs: `guides/customization.md`, `guides/axum-integration.md`, `help/faq.md`,
  `reference/api.md`, `development/external-design.md` §4.1.4, `CHANGELOG.md`

## Explicit non-change scope

No change when the context is absent: JS clients see the inline message,
no-JS clients see the current reload.  No change to error paths.

## Required implementation

1. **`config.rs`.**

   ```rust
   /// Where to send the visitor after a successful submission.
   pub struct ContactSuccessRedirect { path: String, redirect: Arc<dyn Fn(&str) + Send + Sync> }

   #[derive(Debug, thiserror::Error)]
   #[error("invalid redirect path: must be site-relative (start with '/', not '//', no scheme, no control characters)")]
   pub struct InvalidRedirectPath;

   impl ContactSuccessRedirect {
       pub fn new(path: impl Into<String>, redirect: impl Fn(&str) + Send + Sync + 'static) -> Result<Self, InvalidRedirectPath>;
       pub fn path(&self) -> &str;
       /// Called by `submit_contact` on success.
       pub fn apply(&self);
   }
   impl Clone for ContactSuccessRedirect { … }          // Arc clone
   impl std::fmt::Debug for ContactSuccessRedirect { … } // path only
   ```

   Validation in `new`: non-empty; starts with `/`; does not start with
   `//`; contains no `\`, no ASCII control characters, no whitespace; does
   not contain `://`.  Query strings and fragments are allowed.
2. **`server.rs`.**  After a successful `deliver`, before `return Ok(())`:
   `if let Some(r) = use_context::<ContactSuccessRedirect>() { r.apply(); }`.
   Add a rustdoc paragraph "Success redirect" to `submit_contact`.
3. **`axum_helpers.rs`.**

   ```rust
   /// Build a success redirect that uses `leptos_axum::redirect`.
   /// Panics on an invalid path: this is startup configuration.
   pub fn success_redirect(path: impl Into<String>) -> ContactSuccessRedirect
   ```

   Provide it in the **server-function handler** closure (it is harmless
   in the SSR closure but not needed there).
4. **Example.**  In `axum-with-security`: provide
   `success_redirect("/thanks")` and add a `/thanks` route rendering a
   short confirmation.
5. **`lib.rs`.**  Re-export the two new types.

## Required tests

`src/config/tests.rs`:

- `redirect_accepts_site_relative_paths`: `/thanks`, `/a/b?x=1#top`.
- `redirect_rejects_absolute_and_protocol_relative`: `https://evil.test`,
  `//evil.test`, `evil.test/x`, `/a\b`, `""`, `"/a b"`, `"/a\n"`.
- `redirect_apply_calls_executor_with_path`: executor records the path
  into an `Arc<Mutex<Option<String>>>`.

## Required documentation updates

- `customization.md`: new section "Success page" with the rule from the
  RFC (configured → always redirect; not configured → inline / reload).
- `axum-integration.md`: `success_redirect("/thanks")` in the
  server-function closure of the "All context values together" example.
- `faq.md` "Does it work without JavaScript?": success is shown when a
  success page is configured; otherwise the page reloads.
- `api.md`: the new items under `config` and `axum_helpers`.
- `external-design.md` §4.1.4: mark the success row current.
- `CHANGELOG.md` Unreleased: Added.

## Acceptance criteria

- Tests pass; gates green.
- Evidence on the hydrated example with `/thanks` configured:
  1. JS on: valid submit → browser navigates to `/thanks` (no full reload
     before navigation is required; either is acceptable, record which).
  2. JS off: valid submit → `302` to `/thanks` (curl transcript as in RFC
     001 handoff 03 but with valid data; show the `Location` header).
  3. JS off, invalid data → `302` back to the form page with `__err`
     (unchanged behaviour).
- With the context removed, behaviour is identical to before this handoff.

## Prohibited shortcuts

Accepting absolute URLs "for flexibility"; reading the target from a form
field or query parameter; calling `leptos_axum::redirect` from the core
crate.

## Module boundaries

Core knows a path and an opaque executor; only `axum_helpers` knows
`leptos_axum`.

## Compatibility constraints

Additive.  Minor.

## Security constraints

Open-redirect prevention through the validation in `new`; the executor is
integrator code.  The path is not logged with any visitor data.

## Known risks

If `leptos_axum::redirect` does not take effect for the WASM client
because the `ActionForm` redirect hook is set only when a router is
present, record the observed behaviour; the no-JS path is the one this
handoff must deliver.

## Required evidence

Gate outputs; test results; the three transcripts / screenshots.
