# Architecture

## Layer overview

```
┌──────────────────────────────────────────┐
│  ContactForm component                   │  client + server
│  ├─ ContactFormClasses (CSS injection)   │
│  ├─ ContactFormLabels  (text injection)  │
│  └─ ContactFormOptions (behaviour)       │
├──────────────────────────────────────────┤
│  submit_contact server function          │  server only
│  ├─ ContactInput::from_raw (normalise)   │
│  ├─ check_honeypot                       │
│  ├─ validate_fields → ContactFieldErrors │
│  └─ ContactDelivery::deliver             │
├──────────────────────────────────────────┤
│  ContactDelivery trait                   │  server only
│  ├─ NoopDelivery                         │
│  └─ LettreSmtpDelivery (smtp-lettre)     │
└──────────────────────────────────────────┘
```

## Module map

| Module | Purpose |
|--------|---------|
| `model` | `ContactInput` — internal data model, validation, normalisation |
| `config` | `ContactFormClasses` / `ContactFormLabels` / `ContactFormOptions` — client-safe config |
| `error` | `ContactFieldErrors`, `ContactDeliveryError`, `ContactValidationError` |
| `security` | `sanitize_header_value` — defence-in-depth header sanitisation |
| `components` | `ContactForm` — the public Leptos component |
| `server` | `submit_contact` — the server function |
| `delivery` | `ContactDelivery` trait + `ContactDeliveryContext` type alias |
| `delivery::noop` | `NoopDelivery` — discards all submissions |
| `delivery::smtp` | `LettreSmtpDelivery` — SMTP via `lettre` |
| `axum_helpers` | `provide_contact_delivery`, `delivery_context_fn` (axum-helpers feature) |

## Data flow

```
Browser
  │  HTML form POST / WASM fetch
  ▼
submit_contact (server function)
  │  ContactInput::from_raw()  — trim, normalise
  │  check_honeypot()          — silent success on trigger
  │  validate_fields()         — per-field validation errors
  │  use_context::<ContactDeliveryContext>()
  ▼
ContactDelivery::deliver()
  │  build_message()           — From / Reply-To / Subject / Body
  ▼
SMTP relay → admin inbox
```

## Error flow to the client

```
ServerFnError::Args("field_errors:{...json...}")
  → ContactFieldErrors::from_error_str()
  → FieldError component (per input)

ServerFnError::ServerError("Failed to send...")
  → generic error div (role="alert")
```

The `field_errors:` sentinel prefix lets the component reliably distinguish
per-field payloads from generic delivery-failure strings.

## Feature flag design rationale

- `default = []` — minimum footprint; UI types compile without any server code.
- `hydrate` / `ssr` — mirror Leptos's own feature split.
- `smtp-lettre` — pulls in `lettre` + `tokio`; server-only.
- `axum-helpers` — pulls in `axum` + `leptos_axum`; server-only.

No Axum code leaks into the crate's surface unless `axum-helpers` is enabled.
This keeps the crate usable with Actix-Web or other backends.
