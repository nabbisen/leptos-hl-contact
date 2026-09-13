# API

Every public item, grouped by module.  Rustdoc on
[docs.rs](https://docs.rs/leptos-hl-contact) has the same information with
examples.

## Re-exports at the crate root

`ContactForm`, `ContactFormClasses`, `ContactFormLabels`,
`ContactFormOptions`, `ContactServerPolicy`, `ContactDelivery`,
`ContactDeliveryContext`, `ContactDeliveryError`, `ContactFieldErrors`,
`ContactValidationError`, `ContactInput`, `MESSAGE_MAX_LEN`,
`ContactSuccessRedirect`, `InvalidRedirectPath`, `ContactErrorCode`,
`ContactErrorLabels`, `ContactField`, `FieldError`, `FieldErrorCode`,
`submit_contact`, `ChallengeProvider`, `ChallengeTheme`, `ChallengeWidget`,
`InvalidChallengeConfig`, `NoJsPolicy`; with `ssr` `ChallengeContext`,
`ChallengeError`, `ChallengeOutcome`, `ChallengePolicy`, `ChallengeVerifier`,
`ContactFilter`, `ContactFilterContext`, `FilterChain`, `FilterDecision`; with
`challenge-http` `HttpChallengeVerifier`; and with the
`form-token` feature `Binding`, `FormToken`, `FormTokenConfig`,
`FormTokenBinding`, `FormTokenContext`, `FormTokenError`,
`FormTokenIssuer`, `issue_form_token`, `issue_form_token_fn`,
`issue_form_token_with_nonce`, `verify_form_token`.

## `components`

### `ContactForm`

```rust,ignore
#[component]
pub fn ContactForm(
    #[prop(optional, into)] classes: ContactFormClasses,
    #[prop(optional, into)] labels:  ContactFormLabels,
    #[prop(optional, into)] options: ContactFormOptions,
    #[prop(optional_no_strip)] challenge: Option<ChallengeWidget>,
) -> impl IntoView
```

Renders the form as an `<ActionForm/>` bound to `submit_contact`.  Reads
`FormToken` from context when the `form-token` feature is on.  With
`challenge`, renders the vendor widget after the message field; without it,
nothing is loaded from any vendor.  `challenge` takes the `Option` itself:
`challenge=Some(widget)`, or an `Option` built from configuration.  Element ids and
attributes are listed in the
[DOM contract](../development/external-design.md#412-dom-contract).

## `server`

### `submit_contact`

```rust,ignore
#[server(endpoint = "submit_contact")]
pub async fn submit_contact(
    name:       String,
    email:      String,
    subject:    Option<String>,
    message:    String,
    website:    String,          // honeypot — must be empty
    form_token: Option<String>,  // verified when the `form-token` feature is on
    #[server(rename = "cf-turnstile-response")] #[server(default)]
    cf_turnstile_response: Option<String>,
    #[server(rename = "h-captcha-response")] #[server(default)]
    h_captcha_response: Option<String>,
    #[server(rename = "g-recaptcha-response")] #[server(default)]
    g_recaptcha_response: Option<String>,
) -> Result<(), ServerFnError>
```

The three challenge arguments use the field names the vendor widgets inject;
the first non-blank one is the challenge token.

`POST /api/submit_contact`, form-encoded.  Order of work: token check
(`form-token`) → `ContactInput::from_raw` → `check_honeypot` → `validate_fields`
→ `ContactServerPolicy` → challenge (`ChallengeContext`) → filter
(`ContactFilterContext`) → `ContactDelivery::deliver` →
`ContactSuccessRedirect` when one is in context.

| Outcome | Returned as |
|---------|-------------|
| Validation or policy failure | `ServerFnError::Args("field_errors:{…json…}")` |
| Token invalid, expired or missing | `ServerFnError::Args("contact_error:token_invalid")` |
| Token younger than `min_age_secs` | `ServerFnError::Args("contact_error:too_fast")` — retryable |
| Token config or delivery context missing | `ServerFnError::ServerError("contact_error:not_configured")` |
| Challenge token with no `ChallengeContext` | `ServerFnError::ServerError("contact_error:not_configured")` |
| No challenge token under `NoJsPolicy::Reject` | `ServerFnError::Args("contact_error:challenge_required")` |
| Challenge failed (not passed, score, action) | `ServerFnError::Args("contact_error:challenge_failed")` |
| Challenge verifier error | `ServerFnError::ServerError("contact_error:challenge_unavailable")` |
| Filter returned `Reject` | `ServerFnError::Args("contact_error:rejected")` |
| Filter returned `SilentDrop` | `Ok(())` without delivery; the success page applied exactly as for a delivered message |
| Delivery failed | `ServerFnError::ServerError("contact_error:delivery_failed")` |
| Delivery timed out (`ContactDeliveryError::Timeout`) | `ServerFnError::ServerError("contact_error:delivery_timeout")` |
| Unexpected | `ServerFnError::ServerError("contact_error:unexpected")` |
| Honeypot filled | `Ok(())` without delivery; the success page applied exactly as for a delivered message |

**Feature:** `ssr` for the body; the client stub exists under any feature.

## `config`

```rust,ignore
pub struct ContactFormClasses { pub root, field, label, input, textarea, button, error, success: String }
pub struct ContactFormLabels  { pub name, email, subject, message, submit, sending, success, error, honeypot_label: String, pub errors: ContactErrorLabels }
pub struct ContactErrorLabels {
    pub required, length, format_email, format, line_breaks, token_invalid, too_fast,
        not_configured, delivery_failed, delivery_timeout,
        challenge_required, challenge_failed, challenge_unavailable, challenge_requires_js,
        rejected: String,
}
pub enum NoJsPolicy { Reject /* default */, AcceptWithHoneypotOnly }

pub enum ChallengeProvider { Turnstile, HCaptcha, RecaptchaV2, RecaptchaV3 { action: String } }
pub enum ChallengeTheme { Auto /* default */, Light, Dark }
pub struct InvalidChallengeConfig(pub &'static str);   // thiserror

pub struct ChallengeWidget { /* private: every value is validated */ }
impl ChallengeWidget {
    pub fn new(provider: ChallengeProvider, site_key: impl Into<String>) -> Result<Self, InvalidChallengeConfig>;
    pub fn with_theme(self, theme: ChallengeTheme) -> Self;
    pub fn with_language(self, language: impl Into<String>) -> Result<Self, InvalidChallengeConfig>;
    pub fn without_script(self) -> Self;
    pub fn with_script_nonce(self, nonce: impl Into<String>) -> Result<Self, InvalidChallengeConfig>;
    pub fn with_no_js(self, no_js: NoJsPolicy) -> Self;
}
```

`ChallengeWidget` validation: `site_key` and the v3 `action` match
`[A-Za-z0-9_-]+`; `language` matches `[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*`;
`script_nonce` is base64 (`[A-Za-z0-9+/=]+`).  Defaults from `new`: theme
`Auto`, vendor script loaded by the component, no language, no nonce,
`NoJsPolicy::Reject`.

| Provider | Element | Vendor script |
|----------|---------|---------------|
| `Turnstile` | `<div class="cf-turnstile" data-sitekey data-theme="auto\|light\|dark" data-language?>` | `https://challenges.cloudflare.com/turnstile/v0/api.js` |
| `HCaptcha` | `<div class="h-captcha" data-sitekey data-theme?>` (omitted for `Auto`) | `https://js.hcaptcha.com/1/api.js` + `?hl=` |
| `RecaptchaV2` | `<div class="g-recaptcha" data-sitekey data-theme?>` (omitted for `Auto`) | `https://www.google.com/recaptcha/api.js` + `?hl=` |
| `RecaptchaV3` | `<input type="hidden" name="g-recaptcha-response">` and an inline submit script | `https://www.google.com/recaptcha/api.js?render=<site_key>` + `&hl=` |

Scripts are `async defer`, with `nonce` when set.  `without_script` drops the
vendor script; reCAPTCHA v3 keeps its inline submit script.  Under
`NoJsPolicy::Reject` a `<noscript>` follows with
`labels.errors.challenge_requires_js` in a `role="alert"` paragraph.

In the browser, a widget reached by client-side navigation is rendered with
the vendor's explicit `render` when the vendor script has already loaded, and
the script is added to `<head>` at most once otherwise.

```rust,ignore

impl ContactErrorLabels {
    pub fn field_text(&self, field: ContactField, err: &FieldError) -> String;
    pub fn code_text(&self, code: ContactErrorCode) -> String;
}
pub struct ContactFormOptions { pub show_subject: bool, pub require_subject: bool, pub max_message_len: usize, pub focus_first_error: bool }
pub struct ContactServerPolicy { pub require_subject: bool, pub max_message_len: usize }

impl ContactFormOptions {
    pub fn effective_max_message_len(&self) -> usize;   // clamped to MESSAGE_MAX_LEN
}

impl ContactServerPolicy {
    pub fn effective_max_message_len(&self) -> usize;   // clamped to MESSAGE_MAX_LEN
    pub fn check(&self, input: &ContactInput) -> ContactFieldErrors;   // empty == passes
}

pub struct ContactSuccessRedirect { /* path + executor */ }
pub struct InvalidRedirectPath;     // Error

impl ContactSuccessRedirect {
    pub fn new(path: impl Into<String>, redirect: impl Fn(&str) + Send + Sync + 'static)
        -> Result<Self, InvalidRedirectPath>;
    pub fn path(&self) -> &str;
    pub fn apply(&self);            // called by submit_contact on success
}
```

Defaults: classes empty; labels English; options
`true / false / MESSAGE_MAX_LEN / true`; policy `false / MESSAGE_MAX_LEN`.  Meaning
of each field: [Customization](../guides/customization.md).

`ContactSuccessRedirect` is server-side configuration and is not
serialisable.  `new` accepts only site-relative paths — starting with a
single `/`, no scheme, backslash, whitespace or control character — so a
misconfiguration cannot become an open redirect; `Debug` shows the path and
not the executor.  Provide it in the context closure; see
[Success page](../guides/customization.md#success-page).

Both `max_message_len` fields are counted in characters and clamped to
`MESSAGE_MAX_LEN`: a policy can tighten the validator's limit, never raise it.
`ContactServerPolicy::check` applies `require_subject` and the effective
message limit to a normalised input and may report both errors at once.

## `model`

### `MESSAGE_MAX_LEN`

```rust,ignore
pub const MESSAGE_MAX_LEN: usize = 4000;
```

Hard ceiling for `message`, in characters (Unicode scalar values).  It drives
the validator attribute, both `Default` implementations, and both clamps, so
the limit has exactly one definition.

### `ContactInput`

```rust,ignore
pub struct ContactInput {
    pub name: String, pub email: String, pub subject: Option<String>,
    pub message: String, pub website: String,
}

impl ContactInput {
    pub fn from_raw(name, email, subject, message, website) -> Self;      // trims, blank subject → None
    pub fn check_honeypot(&self) -> Result<(), ContactValidationError>;
    pub fn validate_input(&self) -> Result<(), ContactValidationError>;  // opaque, for logs
    pub fn validate_fields(&self) -> ContactFieldErrors;                 // per field, for clients
    pub fn effective_subject(&self, fallback: &str) -> String;
}
```

| Field | Rule |
|-------|------|
| `name` | 1–80 characters, no `\r` `\n` |
| `email` | valid address, at most 254 characters; the domain needs at least two labels and no empty one; address literals are refused |
| `subject` | absent, or 1–120 characters, no `\r` `\n` |
| `message` | 1 to `MESSAGE_MAX_LEN` characters |
| `website` | empty |

## `error`

```rust,ignore
pub const FIELD_ERROR_PREFIX:   &str = "field_errors:";
pub const CONTACT_ERROR_PREFIX: &str = "contact_error:";

pub enum ContactField { Name, Email, Subject, Message }

#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldErrorCode { Required, Length { min: usize, max: usize }, Format, LineBreaks }

#[serde(untagged)]
pub enum FieldError { Code(FieldErrorCode), Text(String) }

pub struct ContactFieldErrors { pub name, email, subject, message: Option<FieldError> }
impl ContactFieldErrors {
    pub fn is_empty(&self) -> bool;
    pub fn to_json(&self) -> String;
    pub fn get(&self, field: ContactField) -> Option<&FieldError>;
    pub fn from_error_str(s: &str) -> Option<Self>;    // finds the sentinel anywhere
    pub fn from_server_fn_error<E>(err: &ServerFnError<E>) -> Option<Self>;  // matches the Args variant
    pub fn into_server_fn_message(self) -> String;
}

#[serde(rename_all = "snake_case")]
pub enum ContactErrorCode {
    TokenInvalid, TooFast, NotConfigured, DeliveryFailed, DeliveryTimeout, Unexpected,
    ChallengeRequired, ChallengeFailed, ChallengeUnavailable, Rejected,
}
impl ContactErrorCode {
    pub fn as_str(self) -> &'static str;
    pub fn from_str_code(s: &str) -> Option<Self>;
    pub fn into_server_fn_message(self) -> String;
    pub fn from_server_fn_error<E>(err: &ServerFnError<E>) -> Option<Self>;  // Args or ServerError
}

pub enum ContactDeliveryError { Configuration(String), Transport(String), MessageBuild(String), Internal(String), Timeout(Duration) }
pub enum ContactValidationError { InvalidInput(String), HoneypotTriggered }
```

The server sends codes, never visitor-facing text; the component renders
them through `ContactErrorLabels`.  `FieldError` is `serde(untagged)`, so a
`0.3` server's pre-rendered sentences still parse as `Text` and are shown
unchanged.

A client should use `from_server_fn_error`, which matches the error variant
rather than its displayed text; `from_error_str` is the fallback for callers
holding only a string.  `ContactDeliveryError` and `ContactValidationError`
are server-side only.

Wire examples:

```text
field_errors:{"email":{"kind":"format"},"name":{"kind":"length","min":1,"max":80}}
contact_error:token_invalid
contact_error:delivery_failed
contact_error:delivery_timeout
contact_error:rejected
```

## `delivery`

```rust,ignore
pub trait ContactDelivery: Send + Sync + 'static {
    fn deliver(&self, input: ContactInput)
        -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>>;
}
pub type ContactDeliveryContext = Arc<dyn ContactDelivery>;
```

### `delivery::noop::NoopDelivery`

Unit struct; discards and logs at `debug`.  No feature flag.

### `delivery::timeout::DeliveryTimeout` (feature `delivery-timeout`)

```rust,ignore
pub struct DeliveryTimeout<D> { /* private */ }
impl<D: ContactDelivery> DeliveryTimeout<D> {
    pub fn new(inner: D, limit: Duration) -> Self;
    pub fn limit(&self) -> Duration;
}
impl<D: ContactDelivery> ContactDelivery for DeliveryTimeout<D> { /* inner, bounded by limit */ }
```

Re-exported at the crate root.  On expiry the inner delivery is dropped and
the result is `ContactDeliveryError::Timeout(limit)`, which reaches the
client as `delivery_timeout`.  `smtp-lettre` enables the feature.

### `delivery::smtp` (feature `smtp-lettre`)

```rust,ignore
pub struct LettreSmtpDelivery { pub config: SmtpConfig }
impl LettreSmtpDelivery {
    pub fn build_message(&self, input: &ContactInput) -> Result<Message, ContactDeliveryError>;
}

pub struct SmtpConfig {
    pub host: String, pub port: u16, pub username: String, pub password: String,
    pub from_address: String, pub to_address: String, pub subject_prefix: String,
    pub tls_mode: SmtpTlsMode,
    pub timeout: Duration,             // connect through the relay's final reply
}
impl SmtpConfig { pub const DEFAULT_TIMEOUT: Duration; }  // 30 s
pub enum SmtpTlsMode { StartTls /* default */, Tls, DangerousPlaintext }
```

`SmtpConfig` and `LettreSmtpDelivery` implement `Debug` with the password
redacted.

## `security`

```rust,ignore
pub fn sanitize_header_value(value: &str) -> String   // replaces each \r and \n with a space
```

## `axum_helpers` (feature `axum-helpers`)

```rust,ignore
pub fn provide_contact_delivery(delivery: ContactDeliveryContext);
pub fn delivery_context_fn(delivery: ContactDeliveryContext) -> impl Fn() + Clone + Send + Sync + 'static;
pub fn success_redirect(path: impl Into<String>) -> ContactSuccessRedirect;

// With the `form-token` feature as well:
pub struct FormTokenCookie {
    pub name: String,   // default "hl_contact_ft"; sent as "__Host-hl_contact_ft"
    pub secure: bool,   // default true
    pub path: String,   // default "/"
}
pub fn provide_form_token_with_cookie(config: &FormTokenContext, cookie: &FormTokenCookie);
pub fn provide_form_token_binding(cookie: &FormTokenCookie);
pub fn provide_form_token_issuer(cookie: &FormTokenCookie);
```

`success_redirect` builds a `ContactSuccessRedirect` backed by
`leptos_axum::redirect`.  It panics on a path that is not site-relative,
because it is startup configuration.

`provide_form_token_with_cookie` acts on `GET` requests only, and reuses the
nonce from an existing cookie so the value is stable per browser.  The
`__Host-` prefix is added to `name` when `secure` is `true` and `path` is
`/`, and all three helpers read and write under that same effective name.
`provide_form_token_issuer` writes the cookie for a token the browser fetches
from `issue_form_token_fn`.  Both
helpers belong in the one context closure:
[Cookie binding](../security/form-token.md#cookie-binding).

## `form_token` module

Feature `form-token`.

```rust,ignore
pub enum Binding { None, Cookie }

pub enum FormTokenError {
    Malformed, BadSignature, Expired, FromFuture, TooYoung,
    BindingMissing, BindingMismatch,
}

pub struct FormTokenConfig {
    pub secret_key: Vec<u8>,
    pub ttl_secs: u64,        // default 3600
    pub min_age_secs: u64,    // default 2; 0 disables
    pub binding: Binding,     // default Binding::None
}
impl FormTokenConfig {
    pub fn new(secret_key: Vec<u8>) -> Self;
    pub fn with_ttl(self, secs: u64) -> Self;
    pub fn with_min_age(self, secs: u64) -> Self;
    pub fn with_binding(self, binding: Binding) -> Self;
}

pub struct FormToken(pub String);
pub type FormTokenContext = Arc<FormTokenConfig>;

pub struct FormTokenBinding(pub Option<String>);
pub struct FormTokenIssuer(pub Arc<dyn Fn(&FormToken) + Send + Sync>);

pub fn issue_form_token(config: &FormTokenConfig) -> FormToken;
pub fn issue_form_token_with_nonce(config: &FormTokenConfig, nonce: &str) -> Option<FormToken>;

// Server function, POST /api/form_token.  Declared in `server` so the browser
// build has it; re-exported here.
#[server(endpoint = "form_token")]
pub async fn issue_form_token_fn() -> Result<String, ServerFnError>;
pub fn verify_form_token(token: &str, bound_value: Option<&str>, config: &FormTokenConfig)
    -> Result<(), FormTokenError>;
```

`FormTokenConfig` implements `Debug` with the key redacted.  Checks run in
this order: format, timestamp, future skew, expiry, minimum age, signature,
binding.  Behaviour and guarantees:
[Form Token](../security/form-token.md).

## `challenge` module

Feature `ssr`; no feature flag of its own.  No HTTP: the built-in vendor
verifiers are separate.

```rust,ignore
pub struct ChallengeOutcome { pub passed: bool, pub score: Option<f32>, pub action: Option<String>, pub error_codes: Vec<String> }
pub enum ChallengeError { Timeout, Unavailable(String), Misconfigured(String) }   // thiserror

pub trait ChallengeVerifier: Send + Sync + 'static {
    fn verify(&self, token: &str)
        -> Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + '_>>;
}

pub struct ChallengePolicy {
    pub no_js: NoJsPolicy,               // default Reject
    pub min_score: f32,                  // default 0.5; ignored without a score
    pub expected_action: Option<String>, // default None: any action
}
pub struct ChallengeContext { pub verifier: Arc<dyn ChallengeVerifier>, pub policy: ChallengePolicy }
```

Provide `ChallengeContext` in the context closure.  `submit_contact` follows
this table:

| Context | Token | Result |
|---------|-------|--------|
| absent | absent | proceed |
| absent | present | `not_configured`, logged at `error` |
| present | absent, `Reject` | `challenge_required` |
| present | absent, `AcceptWithHoneypotOnly` | proceed, logged at `info` |
| present | present, passed and policy satisfied | proceed |
| present | present, not passed, score below `min_score` (or NaN), or action mismatch | `challenge_failed`, logged at `warn` with the vendor's error codes |
| present | present, verifier error | `challenge_unavailable`, logged at `error` — fail-closed |

An empty or whitespace-only token counts as absent.  The token itself is
never logged.

## `filter` module

Feature `ssr`; no feature flag of its own, and no built-in filters.

```rust,ignore
pub enum FilterDecision { Accept, Reject, SilentDrop }

pub trait ContactFilter: Send + Sync + 'static {
    fn filter(&self, input: &ContactInput)
        -> Pin<Box<dyn Future<Output = FilterDecision> + Send + '_>>;
    fn name(&self) -> &'static str { std::any::type_name::<Self>() }
}
pub type ContactFilterContext = Arc<dyn ContactFilter>;

pub struct FilterChain(/* private */);
impl FilterChain { pub fn new(filters: Vec<Arc<dyn ContactFilter>>) -> Self; }
impl ContactFilter for FilterChain { /* first non-Accept wins; later filters are not called */ }
```

Runs after the challenge and before delivery, on validated input only.
`Reject` is `contact_error:rejected`; `SilentDrop` is `Ok(())` without
delivery.  Both are logged at `warn` inside a `contact_filter` span: `filter`
names the filter in context, and for a `FilterChain`, `decided_by` names the
member that decided.  Guide and examples: [Filter](../security/filter.md).
