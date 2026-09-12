# Changelog

## [Unreleased]

No version assigned; the owner decides the release number.

### Fixed

- CI gates green: rustfmt, clippy, doctest.
- CI installs one matched 1.91 toolchain via `dtolnay/rust-toolchain`; the
  previous `apt-get` step shipped no `clippy-1.91`, so the clippy step ran a
  different compiler's clippy against 1.91-compiled dependencies and every
  run failed.
- Both examples start: `"/api/*fn_name"` → `"/api/{*fn_name}"`, the Axum 0.8
  wildcard form.  The old form panicked at router construction, so neither
  example ever bound a port.
- Example manifests were at `0.3.2` while the workspace was `0.3.3`; both
  now match.

### Added

- CI job `examples`, a matrix over both example directories running
  `cargo check`.  The examples are excluded from the workspace, so the crate
  gates never compiled them.

### Documentation

- Documentation book restructured into sections (Getting Started, Guides,
  Security, Reference, Help, Development) with a persona-oriented
  `SUMMARY.md`; `README.md` synced to the six-section structure.
- Planning baseline added: `ROADMAP.md` milestones, `rfcs/` index and state
  folders, `docs/src/development/requirements.md` and
  `docs/src/development/external-design.md`.
- `docs/book.toml`: removed the `git-repository-icon` key rejected by mdBook
  0.5; the book builds with the default icon.
- Crate rustdoc corrected (RFC 001, handoff 05): the crate-level feature table
  now lists `csrf`; the security link points at `docs/src/security/README.md`;
  the quick-start paragraph distinguishes the production and local-development
  examples; the `security.rs` and `delivery.rs` header comments are current;
  the `csrf` module no longer describes its token as "single-use" or as a CSRF
  control on its own; the `axum_helpers` rustdoc examples use the Axum 0.8
  route form `"/api/{*fn_name}"`.
- Release records: every tagged version `0.2.0`–`0.3.3` now carries its tag
  date instead of an "Unreleased" label, and the previously undocumented
  `0.2.1` and `0.2.3` releases have entries.

> Entries for RFC 001 handoffs 01–04 (CI gates, example route and CI job,
> field-error rendering, length units and ceiling) are not present yet: that
> work has not been implemented at the time of writing.

## [0.3.3] — 2026-05-09

### Fixed

- **PII removed from delivery logs** (was listed as done in v0.3.1 but not
  applied): `NoopDelivery` and `LettreSmtpDelivery` no longer log `name` or
  `email` fields.  Log messages are now plain strings without personal data.
- **`Reply-To` built with `Mailbox::new`** (was listed as done in v0.3.1 but
  not applied): replaced `format!("{name} <{email}>").parse()` with
  `Mailbox::new(Some(input.name.clone()), email_address)`.  Handles special
  characters in display names (quotes, commas, angle brackets) correctly.
- **`CSRF_SECRET` / `ALLOWED_ORIGIN` now required in `axum-with-security`**:
  removed the insecure fallback to a hardcoded default secret.  Both variables
  now use `.expect(...)` so the application refuses to start without them.
  Production-ready examples must not silently use known secrets.

### Added

- Tests for `Reply-To` with special characters in the display name.
- Test confirming CSRF missing-context error is a `ServerError` (not a
  field-level error), verifying the fail-closed path returns the right shape.
- Test confirming PII log message strings contain no interpolation specifiers.

### Documentation

- `docs/src/quick-start.md` restructured: base steps use only `ssr +
  smtp-lettre + axum-helpers` (no `csrf` feature); CSRF setup moved to a
  dedicated **Adding CSRF protection** section with full context injection
  example.  This prevents the fail-closed `csrf` feature from silently
  breaking a minimal quick-start setup.
- `docs/src/configuration.md`: added `ContactServerPolicy` section with
  clear UI-vs-security-boundary distinction; updated `SmtpTlsMode` table to
  show `DangerousPlaintext` (was still showing `None`).
- `docs/src/faq.md`: added Q&A on `ContactFormOptions` vs `ContactServerPolicy`
  and on enforcing `require_subject` server-side.

## [0.3.2] — 2026-05-09

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

## [0.3.1] — 2026-05-09

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
- `verify_csrf_token` now rejects tokens with timestamps more than 60 s in
  the future (clock-skew tolerance), preventing clock-drift abuse.
- `SmtpTlsMode::None` renamed to `SmtpTlsMode::DangerousPlaintext` to make
  insecure configuration conspicuous in code reviews.

> **Note:** `Reply-To` via `Mailbox::new` and PII log removal were listed here
> but were not applied correctly; fixed in v0.3.3.

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

## [0.3.0] — 2026-05-09

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

## [0.2.3] — 2026-05-09

### Changed

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

## [0.2.2] — 2026-05-05

### Changed

- Renamed crate directory from `crates/core` to `crates/leptos-hl-contact`
  for clarity. The published crate name (`leptos-hl-contact`) is unchanged.

*Corrected 2026-09-12: earlier revisions of this file listed the 0.2.3
documentation work here.*

## [0.2.1] — 2026-05-05

### Changed

- Test modules moved out of the implementation files into
  `src/<module>/tests.rs` per the project's Rust testing rule.
- `src/delivery/mod.rs` renamed to `src/delivery.rs` (Rust 2018+ module style).
- Crate directory moved from `leptos-hl-contact-crate/` to `crates/core/`.

### Fixed

- Build error and compiler warnings introduced in `0.2.0`.

## [0.2.0] — 2026-05-05

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

## [0.1.0] — 2026-05-05

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
