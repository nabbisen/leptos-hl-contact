# Architecture

Internal structure of the crate for maintainers and contributors.  What
the crate promises at its boundaries is in [External Design](./external-design.md);
why it exists is in [Requirements](./requirements.md).

## Design philosophy

**Secure by default.**  Credentials and the recipient never reach WASM;
validation runs on every submission; the honeypot is a server check;
visitor-facing errors are generic; the delivery layer only ever sees a
validated `ContactInput`.

**Minimal but extensible.**  One component, one server function, one
trait.  SMTP, Axum, form token, and Islands code sit behind feature flags.
SendGrid, SES, persistence, CAPTCHA are left to implementors.

**Progressive enhancement first.**  `<ActionForm/>` works as a plain POST
when no WASM runs.

**Accessibility is not optional.**  ARIA, label associations, live regions,
and keyboard behaviour ship in the default component.

**Documentation-driven.**  Public items carry rustdoc with feature
conditions and security notes; behaviour is specified in
[Requirements](./requirements.md) before it is implemented.

## Layers

```text
┌───────────────────────────────────────────────────────────┐
│  ContactForm                                              │
│    ContactFormClasses / Labels / Options   (props)        │  client + server
│    FormToken                         (context, form-token) │
├───────────────────────────────────────────────────────────┤
│  submit_contact                                           │
│    verify_form_token   (form-token, FormTokenContext)     │
│    → from_raw → check_honeypot → validate_fields          │  server only
│    → ContactServerPolicy → ContactDeliveryContext         │
├───────────────────────────────────────────────────────────┤
│  ContactDelivery (trait)                                  │
│    NoopDelivery                                           │  server only
│    LettreSmtpDelivery                      (smtp-lettre)  │
└───────────────────────────────────────────────────────────┘
```

## Module map

| Module | Role | Compiled for |
|--------|------|--------------|
| `model` | `ContactInput`: normalisation, validation | client + server |
| `config` | `ContactFormClasses`, `ContactFormLabels`, `ContactFormOptions`, `ContactServerPolicy` | client + server; no secrets |
| `error` | `ContactFieldErrors` (crosses the wire), `ContactDeliveryError`, `ContactValidationError` (server) | client + server |
| `security` | `sanitize_header_value` | client + server |
| `components` | `ContactForm`, `FieldError` | client + server |
| `server` | `submit_contact` | body under `ssr`; stub otherwise |
| `delivery` | trait + `ContactDeliveryContext` | server |
| `delivery::noop` | `NoopDelivery` | server |
| `delivery::smtp` | `LettreSmtpDelivery`, `SmtpConfig`, `SmtpTlsMode` | `smtp-lettre` |
| `form_token` | `FormTokenConfig`, `FormToken`, `issue_form_token`, `verify_form_token` | `form-token` |
| `axum_helpers` | `provide_contact_delivery`, `delivery_context_fn` | `axum-helpers` |

## Request flow

```text
browser ── POST /api/submit_contact (form-encoded) ──▶ submit_contact
   verify_form_token            form-token feature; fail-closed if config missing
   ContactInput::from_raw       trim; blank subject → None
   check_honeypot               non-empty website → Ok(()) without delivery
   validate_fields              ContactFieldErrors → ServerFnError::Args("field_errors:…")
   ContactServerPolicy          require_subject / max_message_len
   use_context::<ContactDeliveryContext>
   ContactDelivery::deliver     build_message → SMTP relay
```

## Error flow to the client

```text
ServerFnError::Args("field_errors:{\"email\":{\"kind\":\"format\"}}")
   → component matches the Args variant → ContactFieldErrors::from_server_fn_error
   → FieldErrorCode → labels.errors.field_text(field, err) beside each input

ServerFnError::Args("contact_error:token_invalid")
ServerFnError::ServerError("contact_error:delivery_failed")
   → ContactErrorCode::from_server_fn_error → labels.errors.code_text(code)
   → generic banner

anything else
   → generic banner with labels.error
```

The component matches the `Args` variant rather than testing the error's
displayed text, which the framework prefixes with
`"error deserializing server function arguments: "`.  An `Args` error whose
message carries no sentinel — the token failure, for instance — yields
`None` and takes the banner path, so the check never depends on the
framework's English wording.  `from_error_str` remains available for callers
that hold only a string and locates the sentinel anywhere within it.

The sentinel keeps a single error type on the wire, and the JSON carries
codes rather than sentences, so no visitor-facing English is composed on the
server (RFC 003).  `FieldError` is `serde(untagged)`, so a 0.3 server's
pre-rendered strings still parse and are shown unchanged.

## Feature flags

```text
default = []      types compile with no server or client code
hydrate           client
ssr               server function body, SSR
islands           Leptos Islands
smtp-lettre       + lettre, tokio            (server; enables delivery-timeout)
delivery-timeout  + tokio (time)             (server)
axum-helpers      + axum, leptos_axum        (server)
form-token        + hmac, sha2, rand, hex    (server)
```

Axum is optional so the crate stays usable with other HTTP frameworks.

## Source layout

```text
crates/leptos-hl-contact/src/
  lib.rs                 re-exports, feature gates
  model.rs               model/tests.rs
  config.rs              config/tests.rs
  error.rs               error/tests.rs
  security.rs            security/tests.rs
  components.rs
  server.rs              server/tests.rs
  form_token.rs          form_token/tests.rs
  delivery.rs            delivery/noop.rs  delivery/noop/tests.rs
                         delivery/smtp.rs  delivery/smtp/tests.rs
                         delivery/timeout.rs  delivery/timeout/tests.rs
  axum_helpers.rs        axum_helpers/tests.rs
examples/
  axum-basic/            local development only
  axum-with-security/    production wiring
docs/                    this book (mdBook)
rfcs/                    design records, see rfcs/README.md
```

Each `foo.rs` declares `#[cfg(test)] mod tests;` and the tests live in
`foo/tests.rs` (Rust 2018+ module style, no `mod.rs`).  Examples are not
workspace members; run them with `cd examples/<name> && cargo run`.

## Related pages

- [Testing](./testing.md)
- [Release Process](./release-process.md)
