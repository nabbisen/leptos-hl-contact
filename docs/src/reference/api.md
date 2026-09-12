# API

Every public item, grouped by module.  Rustdoc on
[docs.rs](https://docs.rs/leptos-hl-contact) has the same information with
examples.

## Re-exports at the crate root

`ContactForm`, `ContactFormClasses`, `ContactFormLabels`,
`ContactFormOptions`, `ContactServerPolicy`, `ContactDelivery`,
`ContactDeliveryContext`, `ContactDeliveryError`, `ContactFieldErrors`,
`ContactValidationError`, `ContactInput`, `MESSAGE_MAX_LEN`,
`ContactSuccessRedirect`, `InvalidRedirectPath`, `submit_contact`, and with the
`csrf` feature `CsrfConfig`, `CsrfConfigContext`, `CsrfToken`,
`generate_csrf_token`, `verify_csrf_token`.

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
`CsrfToken` from context when the `csrf` feature is on.  Element ids and
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
    csrf_token: Option<String>,  // verified when the `csrf` feature is on
) -> Result<(), ServerFnError>
```

`POST /api/submit_contact`, form-encoded.  Order of work: token check
(`csrf`) → `ContactInput::from_raw` → `check_honeypot` → `validate_fields`
→ `ContactServerPolicy` → `ContactDelivery::deliver` →
`ContactSuccessRedirect` when one is in context.

| Outcome | Returned as |
|---------|-------------|
| Validation or policy failure | `ServerFnError::Args("field_errors:{…json…}")` |
| Token invalid or expired | `ServerFnError::Args(generic text)` |
| Token config missing, delivery context missing, delivery failed | `ServerFnError::ServerError(generic text)` |
| Honeypot filled | `Ok(())` without delivery |

**Feature:** `ssr` for the body; the client stub exists under any feature.

## `config`

```rust,ignore
pub struct ContactFormClasses { pub root, field, label, input, textarea, button, error, success: String }
pub struct ContactFormLabels  { pub name, email, subject, message, submit, sending, success, error, honeypot_label: String }
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
pub const FIELD_ERROR_PREFIX: &str = "field_errors:";

pub struct ContactFieldErrors { pub name, email, subject, message: Option<String> }
impl ContactFieldErrors {
    pub fn is_empty(&self) -> bool;
    pub fn to_json(&self) -> String;
    pub fn from_error_str(s: &str) -> Option<Self>;    // finds the sentinel anywhere
    pub fn from_server_fn_error<E>(err: &ServerFnError<E>) -> Option<Self>;  // matches the Args variant
    pub fn into_server_fn_message(self) -> String;
}

pub enum ContactDeliveryError { Configuration(String), Transport(String), MessageBuild(String), Internal(String) }
pub enum ContactValidationError { InvalidInput(String), HoneypotTriggered }
```

`ContactFieldErrors` is safe to show; the two enums are server-side only.
A client should use `from_server_fn_error`, which matches the error variant
rather than its displayed text; `from_error_str` is the fallback for callers
holding only a string.

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
```

`success_redirect` builds a `ContactSuccessRedirect` backed by
`leptos_axum::redirect`.  It panics on a path that is not site-relative,
because it is startup configuration.

## `csrf` module

Feature `csrf`.

```rust,ignore
pub struct CsrfConfig { pub secret_key: Vec<u8>, pub token_ttl_secs: u64 }
impl CsrfConfig { pub fn new(secret_key: Vec<u8>) -> Self }   // ttl 3600
pub struct CsrfToken(pub String);
pub type CsrfConfigContext = Arc<CsrfConfig>;

pub fn generate_csrf_token(config: &CsrfConfig) -> CsrfToken;
pub fn verify_csrf_token(token: &str, config: &CsrfConfig) -> bool;
```

`CsrfConfig` implements `Debug` with the key redacted.  Behaviour and
guarantees: [Anti-automation Token](../security/csrf.md).
