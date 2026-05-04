# Changelog

## [0.2.0] — Unreleased

### Added

- Per-field validation errors (`ContactFieldErrors`) surfaced to the client.
  Each input now shows an inline error message (`aria-invalid`, `aria-describedby`)
  when server-side validation fails.
- `FieldError` internal component for accessible per-field error rendering.
- `axum_helpers` module (`axum-helpers` feature):
  - `provide_contact_delivery` — register the delivery context in one call.
  - `delivery_context_fn` — build a reusable `Arc`-cloning closure for both
    Axum injection sites.
- `ContactInput::validate_fields()` — returns `ContactFieldErrors` safe for
  client display, distinct from the opaque `validate_input()` log message.
- `serde_json` dependency for `ContactFieldErrors` serialisation.
- `axum` added as optional dependency (behind `axum-helpers`).
- Comprehensive documentation additions:
  - `docs/src/security.md` — CSRF middleware snippet, rate limiting guide,
    deployment checklist.
  - `docs/src/axum-integration.md` — dual context injection patterns.
  - `docs/src/turnstile.md` — Cloudflare Turnstile integration guide.
  - `docs/src/configuration.md`, `accessibility.md`, `testing.md`,
    `architecture.md`, `installation.md`, `feature-flags.md`.
- Updated `SUMMARY.md` with all new doc pages.
- `examples/axum-basic` updated to use `delivery_context_fn`.
- 8 additional unit tests (27 total).

### Changed

- `server.rs`: validation now returns `ContactFieldErrors` JSON payload
  (prefixed `field_errors:`) instead of a plain string, enabling the component
  to show per-field messages.
- `components.rs`: error display split into generic delivery-failure banner and
  per-field inline errors.

## [0.1.0] — 2026-05-04

### Added

- Initial project structure (Cargo workspace).
- `ContactForm` component with `<ActionForm/>` progressive enhancement.
- `submit_contact` server function with honeypot and server-side validation.
- `ContactInput` model with `validator`-based field validation.
- `ContactDelivery` trait for pluggable delivery backends.
- `NoopDelivery` for local development and tests.
- `LettreSmtpDelivery` SMTP backend via `lettre`.
- `ContactFormClasses`, `ContactFormLabels`, `ContactFormOptions`.
- `security` module with `sanitize_header_value`.
- Axum example (`examples/axum-basic`).
- Apache-2.0 licence (`LICENSE`, `NOTICE`).
