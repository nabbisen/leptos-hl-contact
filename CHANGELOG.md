## [0.3.0] — Unreleased

### Added

- `csrf` feature: stateless HMAC-SHA256 CSRF token helper.
  - `CsrfConfig` — secret key + TTL configuration (server-side only).
  - `CsrfToken` — per-request token provided via Leptos context.
  - `generate_csrf_token(&config) -> CsrfToken` — generates a signed
    `{timestamp}|{nonce}|{hmac}` token on each SSR render.
  - `verify_csrf_token(token, &config) -> bool` — constant-time HMAC
    verification with TTL check.
  - `CsrfConfigContext` type alias (`Arc<CsrfConfig>`) for Leptos context.
  - 8 unit tests covering token round-trip, tamper detection, expiry,
    key mismatch, malformed tokens, and uniqueness.
- `ContactForm` now embeds a hidden `<input name="csrf_token">` field
  automatically when `CsrfToken` is present in the Leptos context.
- `submit_contact` gains a `csrf_token: String` parameter; when
  `CsrfConfigContext` is in context the token is verified before
  processing the form. Backward-compatible: verification is skipped
  when `CsrfConfigContext` is absent.
- `examples/axum-with-security` — complete example demonstrating:
  - `tower_governor` rate limiting (IP-based, 2 req/s, burst 5).
  - HMAC CSRF token injection and verification.
  - Origin/Referer header validation middleware.
  - `CSRF_SECRET` and `ALLOWED_ORIGIN` environment variable setup.

### Changed

- Near-term roadmap items fully completed (v0.2.x + v0.3.0).

# Changelog

## [0.2.2] — Unreleased

### Changed

- Renamed crate directory from `crates/core` to `crates/leptos-hl-contact`
  for clarity. The published crate name (`leptos-hl-contact`) is unchanged.
- `cargo outdated` check: all 14 dependencies confirmed up to date
  (`leptos` 0.8.19, `axum` 0.8.9, `lettre` 0.11.21, `tokio` 1.52.3, etc.).
- README.md rewritten to match the project specification: six-section structure,
  updated Quick Start to `v0.2` and `delivery_context_fn`, no license text body.
- `docs/src/SUMMARY.md` reorganised around three reader personas
  (New Users / Experienced Users / Maintainers).
- `docs/src/introduction.md` expanded with full feature list and scope.
- `docs/src/quick-start.md` rewritten as a step-by-step tutorial.
- `docs/src/api-reference.md` written (was placeholder).
- `docs/src/troubleshooting.md` written (was placeholder).
- `docs/src/faq.md` added (new page).
- `docs/src/architecture.md` expanded with design philosophy, principles,
  and release process.
- `docs/book.toml` updated with full mdBook HTML output configuration.

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
