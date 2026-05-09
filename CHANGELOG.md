# Changelog

## [0.3.2] — Unreleased

### Fixed

- `cargo package` now succeeds without errors or warnings.
  - **Root cause**: `examples/axum-basic` and `examples/axum-with-security` were
    listed as workspace `members`.  Their `path = "../../crates/leptos-hl-contact"`
    dependencies cannot be resolved during `cargo package` verification (the
    examples directory is not part of the `.crate` tarball).
  - **Fix**: removed examples from workspace `members`; added them to
    `workspace.exclude`.  Examples are now standalone Cargo projects run via
    `cd examples/<name> && cargo run`.  Their `Cargo.toml` files no longer use
    `*.workspace = true`.
- `[workspace.package]` now includes `homepage` and `documentation`, eliminating
  the "manifest has no documentation, homepage or repository" warning during
  `cargo package`.

## [0.3.1] — Unreleased

### Security (Critical)

- `SmtpConfig` and `CsrfConfig` no longer derive `Debug`; manual `Debug` impls
  redact `password` and `secret_key` as `"<redacted>"`.
  `LettreSmtpDelivery` manual `Debug` delegates to the redacted `SmtpConfig`.
  Prevents accidental credential exposure in logs, panics, and observability
  middleware.
- `csrf` feature is now **fail-closed**: when `CsrfConfigContext` is not
  provided in Leptos context, `submit_contact` returns a `ServerError` instead
  of silently skipping verification. Prevents the silent-CSRF-bypass footgun.
- Origin / Referer validation in `examples/axum-with-security` now uses
  `url::Url::parse` + strict scheme/host/port comparison instead of
  `starts_with`, closing the prefix-spoofing bypass
  (`https://example.com.evil.test`).
- `RequestBodyLimitLayer` (32 KiB) added to both examples, preventing
  large-POST abuse before any handler or validation runs.
- `examples/axum-basic` now emits a loud `tracing::warn!` at startup and is
  clearly labelled **local development only** in comments and README.

### Security (Medium)

- `sanitize_header_value` now applied to both `subject_prefix` and the
  effective subject in `LettreSmtpDelivery::build_message` (defence-in-depth
  on top of the existing newline-rejection validator).
- `Reply-To` header now built with `Mailbox::new(Some(display_name), address)`
  instead of `"{name} <{email}>".parse()`, handling special characters in
  display names safely.
- `verify_csrf_token` now rejects tokens with timestamps more than 60 s in
  the future (clock-skew tolerance), preventing clock-drift abuse.
- `SmtpTlsMode::None` renamed to `SmtpTlsMode::DangerousPlaintext` to make
  insecure configuration conspicuous in code reviews.
- PII (name, email) removed from `NoopDelivery` debug log and
  `LettreSmtpDelivery` info log.

### Added

- `ContactServerPolicy` — server-side enforcement of `require_subject` and
  `max_message_len`, independent of the client-side `ContactFormOptions`.
  Provide via Leptos context; enforced in `submit_contact` before delivery.
- `csrf_token` parameter in `submit_contact` changed from `String` to
  `Option<String>` for graceful deserialization when the hidden field is
  absent in older form deployments.
- `server function endpoint = "submit_contact"` annotation for stable routing.

### Changed

- README Quick Start now links to `examples/axum-with-security` as the
  primary production reference; `examples/axum-basic` demoted to local-dev
  skeleton.
- `docs/src/quick-start.md` updated to v0.3 versions and `csrf` feature.
- `docs/src/security.md` Origin validation example updated to URL-parsed
  strict comparison.

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
