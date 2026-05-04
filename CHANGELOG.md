# Changelog

## Unreleased

### Added

- Initial project structure (Cargo workspace).
- `ContactForm` component design with `<ActionForm/>` progressive enhancement.
- `submit_contact` server function with honeypot and server-side validation.
- `ContactInput` model with `validator`-based field validation.
- `ContactDelivery` trait for pluggable delivery backends.
- `NoopDelivery` for local development and tests.
- `LettreSmtpDelivery` SMTP backend via `lettre`.
- `ContactFormClasses`, `ContactFormLabels`, `ContactFormOptions` configuration types.
- `security` module with header-injection sanitisation utility.
- Axum example (`examples/axum-basic`).
- Initial `docs/` content (Quick Start, Security, Styling, Accessibility, Delivery Backends).
- Apache-2.0 licence (`LICENSE`, `NOTICE`).
- `ROADMAP.md` and `CHANGELOG.md`.
