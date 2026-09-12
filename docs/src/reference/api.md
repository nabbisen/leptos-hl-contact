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
`submit_contact`, and with the
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
) -> impl IntoView
```

Renders the form as an `<ActionForm/>` bound to `submit_contact`.  Reads
`FormToken` from context when the `form-token` feature is on.  Element ids and
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
    csrf_token: Option<String>,  // deprecated 0.4 name; used only if form_token is absent
) -> Result<(), ServerFnError>
```

`POST /api/submit_contact`, form-encoded.  Order of work: token check
(`form-token`) → `ContactInput::from_raw` → `check_honeypot` → `validate_fields`
→ `ContactServerPolicy` → `ContactDelivery::deliver` →
`ContactSuccessRedirect` when one is in context.

| Outcome | Returned as |
|---------|-------------|
| Validation or policy failure | `ServerFnError::Args("field_errors:{…json…}")` |
| Token invalid, expired or missing | `ServerFnError::Args("contact_error:token_invalid")` |
| Token younger than `min_age_secs` | `ServerFnError::Args("contact_error:too_fast")` — retryable |
| Token config or delivery context missing | `ServerFnError::ServerError("contact_error:not_configured")` |
| Delivery failed | `ServerFnError::ServerError("contact_error:delivery_failed")` |
| Unexpected | `ServerFnError::ServerError("contact_error:unexpected")` |
| Honeypot filled | `Ok(())` without delivery |

**Feature:** `ssr` for the body; the client stub exists under any feature.

## `config`

```rust,ignore
pub struct ContactFormClasses { pub root, field, label, input, textarea, button, error, success: String }
pub struct ContactFormLabels  { pub name, email, subject, message, submit, sending, success, error, honeypot_label: String, pub errors: ContactErrorLabels }
pub struct ContactErrorLabels { pub required, length, format_email, format, line_breaks, token_invalid, not_configured, delivery_failed: String }

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
| `email` | valid address |
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
pub enum ContactErrorCode { TokenInvalid, NotConfigured, DeliveryFailed, Unexpected }
impl ContactErrorCode {
    pub fn as_str(self) -> &'static str;
    pub fn from_str_code(s: &str) -> Option<Self>;
    pub fn into_server_fn_message(self) -> String;
    pub fn from_server_fn_error<E>(err: &ServerFnError<E>) -> Option<Self>;  // Args or ServerError
}

pub enum ContactDeliveryError { Configuration(String), Transport(String), MessageBuild(String), Internal(String) }
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
}
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

## `csrf` module — deprecated

Feature `csrf`, which enables `form-token`.  Every item warns and is removed
in the next minor: `CsrfConfig`, `CsrfToken`, `CsrfConfigContext`,
`generate_csrf_token`, `verify_csrf_token`.  The migration table is in
[Form Token](../security/form-token.md#migration-from-the-03-and-04-names).
