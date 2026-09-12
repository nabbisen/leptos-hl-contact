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
4. **No-JS.**  When `no_js == Reject`, render
   `<noscript><p class=classes.error role="alert">{labels.errors.challenge_requires_js}</p></noscript>`
   right after the widget element.
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

The v3 flow depends on `requestSubmit` (not in very old browsers) and on
`ActionForm` honouring `defaultPrevented`; if the latter is false, stop
and report before inventing a workaround.

## Required evidence

Gate outputs; tests; network panel screenshot.
