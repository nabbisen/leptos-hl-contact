# Handoff 02 — Component: widget, scripts, no-JS

**RFC.** [005](../../accepted/005-challenge-providers.md), D1, amendments.
**Requirements.** FR-ABUSE-10, FR-ABUSE-11, FR-I18N-04.

## Purpose

Render the vendor widget inside the form from configuration alone.

## Change scope

`src/config.rs` (+ tests): `ChallengeProvider`, `ChallengeTheme`,
`ChallengeWidget`; `src/components.rs` (+ SSR tests).

## Explicit non-change scope

Server code; existing markup and ids; any behaviour when the prop is
absent.

## Required implementation

1. **Types (`config.rs`, always compiled).**

   ```rust
   pub enum ChallengeProvider { Turnstile, HCaptcha, RecaptchaV2, RecaptchaV3 { action: String } }
   pub enum ChallengeTheme { Auto, Light, Dark }   // default Auto
   pub struct ChallengeWidget { provider, site_key: String, theme, language: Option<String>, load_script: bool /* true */, script_nonce: Option<String>, no_js: NoJsPolicy }
   #[derive(Debug, thiserror::Error)] pub struct InvalidChallengeConfig(pub &'static str);
   impl ChallengeWidget {
       /// Validates `site_key` (and v3 `action`) against `[A-Za-z0-9_-]+`.
       pub fn new(provider: ChallengeProvider, site_key: impl Into<String>) -> Result<Self, InvalidChallengeConfig>;
       pub fn with_theme(self, ChallengeTheme) -> Self; pub fn with_language(self, impl Into<String>) -> Self;
       pub fn without_script(self) -> Self; pub fn with_script_nonce(self, impl Into<String>) -> Self; pub fn with_no_js(self, NoJsPolicy) -> Self;
   }
   ```

   `language`, if set, must match `[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*`;
   `script_nonce` must be base64 characters.  Reject otherwise in the
   builder (`Result` on `with_language` / `with_script_nonce`, or validate
   in `new` and panic-free `debug_assert!`: choose `Result` for both).
2. **Prop.**  `#[prop(optional)] challenge: Option<ChallengeWidget>` on
   `ContactForm`.
3. **Markup**, rendered inside the `<ActionForm>` between the message
   field and the hidden token, in a `<div class=classes.field>`:

   | Provider | Element |
   |----------|---------|
   | Turnstile | `<div class="cf-turnstile" data-sitekey=… data-theme="auto|light|dark" data-language=…?>` |
   | hCaptcha | `<div class="h-captcha" data-sitekey=… data-theme="light|dark"?>` (omit for Auto) |
   | reCAPTCHA v2 | `<div class="g-recaptcha" data-sitekey=… data-theme="light|dark"?>` |
   | reCAPTCHA v3 | `<input type="hidden" name="g-recaptcha-response">` + the inline script below |

   Script tag when `load_script`: rendered once, immediately after the
   widget element, `async defer`, `nonce` when given:
   Turnstile `https://challenges.cloudflare.com/turnstile/v0/api.js`;
   hCaptcha `https://js.hcaptcha.com/1/api.js` + `?hl=<language>` when set;
   reCAPTCHA v2 `https://www.google.com/recaptcha/api.js` + `?hl=`;
   v3 `https://www.google.com/recaptcha/api.js?render=<site_key>` (+ `&hl=`).

   v3 inline script (exact behaviour; formatting free), `nonce` when given:

   ```js
   (function(){
     var s=document.currentScript,f=s.closest('form'),i=f.querySelector('input[name="g-recaptcha-response"]');
     f.addEventListener('submit',function(e){
       if(i.dataset.fresh==='1'){i.dataset.fresh='';return;}
       e.preventDefault();
       grecaptcha.ready(function(){grecaptcha.execute('SITE_KEY',{action:'ACTION'}).then(function(t){i.value=t;i.dataset.fresh='1';f.requestSubmit();});});
     },true);
   })();
   ```

   `SITE_KEY` and `ACTION` are safe by construction (validated charset).
   Leptos' `ActionForm` returns early when `default_prevented()` is set,
   so the first submit is cancelled and the second, with the token,
   proceeds; verify this against the installed Leptos version and note it.
4. **No-JS.**  When `no_js == Reject`, render a `<noscript>` right after the
   widget element containing `<p class="{classes.error}" role="alert">{labels.errors.challenge_requires_js}</p>`.
   **Render its content with `inner_html`, not as child views** (amended
   2026-09-13).  With scripting enabled the browser's HTML parser keeps
   `<noscript>` content as a single text node, so hydration walking an
   expected `<p>` child would find text and fail.  tachys does not walk the
   children of an element whose content is `inner_html`, and leaves it alone
   when hydrating from the server (`html/element/inner_html.rs`).  The label
   and the class are integrator text inside raw HTML, so escape both
   (`&`, `<`, `>`, `"`) with a small private helper and unit-test it.
5. **Escaping.**  All attribute values go through Leptos attributes (auto
   escaped).  The inline script is the only raw insertion; it may contain
   only the validated `site_key` and `action`.

## Required tests

`config/tests.rs`: `new` rejects `abc$`, empty; accepts `1x00000000000000000000AA`;
language and nonce validation.
`components/tests.rs` (SSR): per provider the expected element and script
URL are present; Auto theme omits `data-theme` for hCaptcha/reCAPTCHA and
emits `auto` for Turnstile; `without_script` emits no `<script>`;
`Reject` emits `<noscript>`, `AcceptWithHoneypotOnly` does not; v3 emits
the hidden input, the render URL, and an inline script containing
`grecaptcha.execute('SITE'`; `nonce` attribute present when set.

## Required documentation updates

Reference API (types and prop).  Guide pages in handoff 03.

## Acceptance criteria

Tests pass; gates green; on the hydrated example with Turnstile test key
`1x00000000000000000000AA` the widget renders and the form submits with a
`cf-turnstile-response` field in the request body (network panel).

## Prohibited shortcuts

`inner_html` for anything but the v3 script; unvalidated site keys;
loading vendor scripts when the prop is absent.

## Compatibility and security constraints

Additive; new elements only with the prop.  CSP: document in handoff 03.

## Known risks

- **`ActionForm` and `defaultPrevented`** — confirmed by the architect:
  Leptos 0.8.20 `form.rs` returns early when `default_prevented()` is set, so
  the v3 script's cancelled first submit is honoured.  `requestSubmit` is not
  available in very old browsers; document it.
- **Vendor implicit rendering and client-side navigation** (added
  2026-09-13).  All three vendors scan the page for their widget class when
  their script first loads.  A form reached by client-side navigation adds the
  widget element after that scan, so it may never render.  Test it (evidence
  item 3).  If it does not render, switch to each vendor's explicit rendering
  (`?render=explicit` plus `turnstile.render` / `hcaptcha.render` /
  `grecaptcha.render` on the element, called from a client effect under the
  `all(feature = "hydrate", not(feature = "ssr"))` gate); do not work around
  it with a page reload.
- **The vendor script loaded twice.**  `load_script` renders a `<script>` on
  every mount, so client-side navigation back to the form inserts it again.
  Vendors warn or fail on a second load.  Render the script only if a
  `<script>` with the same `src` is not already in the document, checked in
  the browser; the server render always includes it.
- **The v3 inline script under client-side rendering.**  An element created
  with `document.createElement` executes when inserted, and
  `document.currentScript` is set while it runs, but `closest('form')` needs
  the form to be connected by then.  Verify it in evidence item 3; if the
  form is not yet an ancestor at execution time, move the listener
  installation into the client effect instead of the inline script.

## Required evidence

Gate outputs; tests; and, in headless Chromium over CDP on a freshly built
bundle with Turnstile test key `1x00000000000000000000AA`:

1. Load the form directly: the widget renders, a submit carries
   `cf-turnstile-response` in the request body, **no hydration error** in
   the console.
2. With JavaScript disabled: the `<noscript>` message is shown as a
   paragraph.
3. Load another page first, navigate to the form client-side, submit: the
   widget renders, the token is in the body, and the vendor script appears
   **once** in the document.  Then navigate away and back, and show it is
   still once.
4. reCAPTCHA v3 with Google's test key: the first submit is cancelled, a
   token is fetched, the second submit carries it — on a direct load and
   after client-side navigation.

Vendor endpoints are external; if the environment cannot reach them, say so
and record what was and was not shown, as for the `__Host-` refusal.
