# API Reference

Complete reference for every public type and function in `leptos-hl-contact`.

---

## Components

### `ContactForm`

```rust
#[component]
pub fn ContactForm(
    #[prop(optional, into)] classes: ContactFormClasses,
    #[prop(optional, into)] labels:  ContactFormLabels,
    #[prop(optional, into)] options: ContactFormOptions,
) -> impl IntoView
```

The primary Leptos contact form component.  Uses `<ActionForm/>` for
progressive enhancement.  All props are optional and default to sensible
English-language values.

**Feature:** available under both `hydrate` and `ssr`.

---

## Server Functions

### `submit_contact`

```rust
#[server]
pub async fn submit_contact(
    name:    String,
    email:   String,
    subject: Option<String>,
    message: String,
    website: String,            // honeypot — must be empty
) -> Result<(), ServerFnError>
```

Leptos server function.  Compiled to `POST /api/submit_contact`.

**Processing order:**
1. `ContactInput::from_raw` — trim and normalise
2. `check_honeypot` — silent `Ok(())` if `website` is non-empty
3. `validate_fields` — returns `ContactFieldErrors` payload on failure
4. `use_context::<ContactDeliveryContext>` — panics with a logged error if absent
5. `ContactDelivery::deliver` — generic error string on failure

**Feature:** `ssr`

---

## Configuration Types

### `ContactFormClasses`

```rust
pub struct ContactFormClasses {
    pub root:     String,   // outermost wrapper
    pub field:    String,   // label + input wrapper
    pub label:    String,   // <label> elements
    pub input:    String,   // <input> elements
    pub textarea: String,   // <textarea> element
    pub button:   String,   // submit <button>
    pub error:    String,   // per-field and generic error messages
    pub success:  String,   // success message
}
```

All fields default to `""`.  Pass CSS class strings from any framework.

### `ContactFormLabels`

```rust
pub struct ContactFormLabels {
    pub name:          String,  // default: "Name"
    pub email:         String,  // default: "Email"
    pub subject:       String,  // default: "Subject"
    pub message:       String,  // default: "Message"
    pub submit:        String,  // default: "Send"
    pub sending:       String,  // default: "Sending…"
    pub success:       String,  // default: "Your message has been sent…"
    pub error:         String,  // default: "Failed to send message…"
    pub honeypot_label:String,  // screen-reader text for honeypot
}
```

Override any subset to localise or customise copy.

### `ContactFormOptions`

```rust
pub struct ContactFormOptions {
    pub show_subject:    bool,    // default: true
    pub require_subject: bool,    // default: false
    pub max_message_len: usize,   // default: 4000 (hard server limit)
}
```

---

## Data Model

### `ContactInput`

```rust
pub struct ContactInput {
    pub name:    String,
    pub email:   String,
    pub subject: Option<String>,
    pub message: String,
    pub website: String,         // honeypot
}
```

Internal model produced on the server after normalisation.  Never serialised
to the client.

**Validation rules:**

| Field | Rule |
|-------|------|
| `name` | 1–80 chars, no `\r`/`\n` |
| `email` | RFC-valid email address |
| `subject` | 0–120 chars, no `\r`/`\n` |
| `message` | 1–4 000 chars |
| `website` | must be empty (honeypot) |

**Key methods:**

```rust
// Construct and normalise from raw server-fn arguments
pub fn from_raw(name, email, subject, message, website) -> Self

// Check honeypot; returns HoneypotTriggered if website is non-empty
pub fn check_honeypot(&self) -> Result<(), ContactValidationError>

// Server-internal validation (opaque error string, for logging)
pub fn validate_input(&self) -> Result<(), ContactValidationError>

// Client-safe per-field validation errors
pub fn validate_fields(&self) -> ContactFieldErrors

// Resolve effective subject with fallback
pub fn effective_subject(&self, fallback: &str) -> String
```

---

## Delivery

### `ContactDelivery` trait

```rust
pub trait ContactDelivery: Send + Sync + 'static {
    async fn deliver(&self, input: ContactInput) -> Result<(), ContactDeliveryError>;
}
```

Implement this to add any delivery backend.

### `ContactDeliveryContext`

```rust
pub type ContactDeliveryContext = Arc<dyn ContactDelivery>;
```

Type alias for the Leptos context value.  Register with `provide_context` in
both the server-function handler and the SSR renderer.

### `NoopDelivery`

```rust
pub struct NoopDelivery;
```

Discards every submission and logs at `DEBUG`.  Use for local development and
tests.  **Feature:** no feature flag required.

### `LettreSmtpDelivery`

```rust
pub struct LettreSmtpDelivery {
    pub config: SmtpConfig,
}
```

SMTP delivery via [`lettre`](https://docs.rs/lettre).  **Feature:** `smtp-lettre`.

### `SmtpConfig`

```rust
pub struct SmtpConfig {
    pub host:           String,
    pub port:           u16,
    pub username:       String,
    pub password:       String,       // load from env var
    pub from_address:   String,
    pub to_address:     String,
    pub subject_prefix: String,
    pub tls_mode:       SmtpTlsMode,
}
```

### `SmtpTlsMode`

```rust
pub enum SmtpTlsMode {
    StartTls,   // port 587, recommended
    Tls,        // port 465, implicit TLS
    None,       // plaintext — local dev only
}
```

---

## Error Types

### `ContactFieldErrors`

```rust
pub struct ContactFieldErrors {
    pub name:    Option<String>,
    pub email:   Option<String>,
    pub subject: Option<String>,
    pub message: Option<String>,
}
```

Per-field validation errors safe for client display.  Serialised as JSON with
the `field_errors:` sentinel prefix inside `ServerFnError::Args`.

### `ContactDeliveryError`

```rust
pub enum ContactDeliveryError {
    Configuration(String),
    Transport(String),
    MessageBuild(String),
    Internal(String),
}
```

Server-side only.  Never forward to the client.

### `ContactValidationError`

```rust
pub enum ContactValidationError {
    InvalidInput(String),
    HoneypotTriggered,
}
```

Server-internal.  `HoneypotTriggered` results in a silent `Ok(())` to the
client.

---

## Axum Helpers

**Feature:** `axum-helpers`

### `provide_contact_delivery`

```rust
pub fn provide_contact_delivery(delivery: ContactDeliveryContext)
```

Register `delivery` as a Leptos context value.  Call inside the context
closures of both `handle_server_fns_with_context` and
`leptos_routes_with_context`.

### `delivery_context_fn`

```rust
pub fn delivery_context_fn(
    delivery: ContactDeliveryContext,
) -> impl Fn() + Clone + Send + Sync + 'static
```

Build a reusable `Arc`-cloning closure that calls `provide_contact_delivery`.
Pass the same closure (or its clones) to both Axum injection sites.

---

## Security Utilities

### `sanitize_header_value`

```rust
pub fn sanitize_header_value(value: &str) -> String
```

Replace `\r` and `\n` with spaces.  Defence-in-depth for email header
injection.  The input validation layer already rejects newlines; this function
is an additional safety net for custom code paths.
