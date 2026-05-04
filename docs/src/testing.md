# Testing

## Running the test suite

```bash
# All unit tests (includes SMTP message builder and model validation)
cargo test -p leptos-hl-contact --features smtp-lettre,axum-helpers --lib

# Just model and security tests (no SMTP feature needed)
cargo test -p leptos-hl-contact --lib
```

## Test categories

### Model tests (`src/model.rs`)

Validate `ContactInput` construction and the `validator` derive rules:

- `valid_input_passes_validation`
- `empty_name_fails`
- `invalid_email_fails`
- `too_long_message_fails`
- `newline_in_name_fails`
- `newline_in_subject_fails`
- `honeypot_input_is_detected`
- `subject_fallback_works`
- `empty_subject_uses_fallback`

### Error tests (`src/error.rs`)

Validate the `ContactFieldErrors` serialisation and sentinel prefix:

- `field_errors_default_is_empty`
- `field_errors_roundtrip_json`
- `from_error_str_parses_prefixed_payload`
- `from_error_str_returns_none_for_plain_string`

### Delivery tests (`src/delivery/noop.rs`, `src/delivery/smtp.rs`)

- `noop_delivery_succeeds` — async test; uses `tokio::test`
- `message_builder_creates_expected_headers`
- `from_uses_configured_address`
- `reply_to_uses_user_email`
- `subject_contains_prefix_and_value`
- `body_includes_expected_fields`

### Security tests (`src/security.rs`, `src/server.rs`)

- `header_injection_attempt_is_sanitised`
- `field_error_message_has_prefix`
- `generic_error_has_no_prefix`
- `field_error_prefix_is_not_empty`

### Axum helper tests (`src/axum_helpers.rs`)

- `delivery_context_fn_is_clone`

## Testing SMTP delivery

Real SMTP sends are not part of the standard CI suite.  To test against a
real relay:

1. Copy `examples/axum-basic` and configure `LettreSmtpDelivery` with a test
   inbox (e.g. Mailtrap, MailHog, or Ethereal Email).
2. Set the required environment variables and run the example.
3. Submit the form and confirm delivery.

### Local SMTP server (MailHog)

```bash
# Start MailHog
docker run -p 1025:1025 -p 8025:8025 mailhog/mailhog

# Configure SmtpConfig
SmtpConfig {
    host:     "localhost".into(),
    port:     1025,
    username: "".into(),
    password: "".into(),
    tls_mode: SmtpTlsMode::None,  // no TLS for local dev
    // …
}
```

Open http://localhost:8025 to view received messages.
