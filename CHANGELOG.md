# Changelog

## [Unreleased]

No version assigned; the owner decides the release number.

### Fixed

- The hidden anti-automation token survives a failed submission in the
  browser.  The form subtree was rebuilt whenever the action's value changed,
  and the rebuild rewrote the hidden field from the client, where no token
  context exists — so the next submit failed with "Invalid or expired
  security token. Please reload the page."  The `<form>` and its inputs are
  now created once: only the success region, the generic-error region, and
  each field's error paragraph and ARIA attributes react.

### Added

- `ContactFormOptions::focus_first_error` (default `true`): after a failed
  submission, keyboard focus moves to the first invalid input.  Client-side
  only.  `ContactFormOptions` gains a field; construct it with
  `..Default::default()` to stay source-compatible.
- `ContactSuccessRedirect` and `InvalidRedirectPath`, with
  `axum_helpers::success_redirect`.  Name a success page and every successful
  submission lands there, with or without JavaScript — closing the gap where
  a no-JavaScript visitor got no confirmation at all.  Only site-relative
  paths are accepted, so a misconfiguration cannot become an open redirect.
  Without the context nothing changes: JavaScript clients show the inline
  message and no-JavaScript clients reload the form page.

### Changed

- **The server no longer composes visitor-facing text.**  Validation, policy,
  token and configuration failures now travel as codes, and the component
  renders them from the new `ContactFormLabels::errors`, so translating the
  labels translates every message a visitor can see — including on the
  no-JavaScript path.

  **Breaking for struct-literal users** of two types:

  - `ContactFieldErrors`'s four fields change from `Option<String>` to
    `Option<FieldError>`.  Construct them as
    `Some(FieldError::Code(FieldErrorCode::Required))`, or
    `Some(FieldError::Text("…".into()))` to keep a literal sentence.  Reading
    code should match on `FieldError` or call the new
    `ContactFieldErrors::get(ContactField)`.
  - `ContactFormLabels` gains `errors: ContactErrorLabels`.  Build it with
    `..Default::default()` and you are unaffected; an exhaustive struct
    literal must add the field.

  On the wire, `field_errors:` members become objects such as
  `{"kind":"length","min":1,"max":80}`, and whole-submission failures become
  `contact_error:<code>`.  `FieldError` is `serde(untagged)`, so a current
  client still reads a `0.3` server's sentences and shows them unchanged.
  A `0.3` client reading a current server falls back to its generic banner,
  which is what it already showed for every validation error.

### Added

- `FieldErrorCode`, `FieldError`, `ContactErrorCode`, `ContactField` and
  `ContactErrorLabels`, all re-exported at the crate root, plus
  `ContactFieldErrors::get`, `ContactErrorCode::from_server_fn_error` and
  the `CONTACT_ERROR_PREFIX` sentinel.
- `ContactErrorLabels::field_text` and `code_text` render a code into text;
  `length` may use the `{min}` and `{max}` placeholders, replaced by plain
  substitution with no format syntax.

### Documentation

- **One context closure for Axum.**  The documentation told integrators to
  provide every context value at "two sites", a hand-written
  `/api/{*fn_name}` route and the SSR closure.  That was wrong:
  `leptos_routes_with_context` registers each server function at its own path
  using its own closure, and Axum prefers that literal path, so the manual
  route never served a registered server function and a value provided only
  there never reached it.  Both examples now use one closure and no manual
  route; the guides, the security pages, the troubleshooting entries, the
  External Design and the rustdoc say so.  No crate logic changed.
- The Production Checklist gains a row for configuring a success page when
  visitors without JavaScript must see a confirmation, and the accessibility
  guide notes that such a page confirms by navigation rather than a live
  region.

## [0.3.4] — 2026-09-12

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
- Per-field validation errors render beside their field again, in both the
  WASM and no-JavaScript paths.  The component tested the error's displayed
  text with `starts_with`, but the framework prefixes `ServerFnError::Args`
  with `"error deserializing server function arguments: "`, so the test was
  always false and every validation failure fell through to the generic
  banner.  The component now matches the error variant instead.
- `ContactServerPolicy` counts the message limit in characters, not bytes.
  It compared `String::len` while the validator and the textarea count
  characters, so a policy of 4 000 rejected a roughly 1 300-character Japanese
  message that both the browser and the validator accepted.
- The 4 000-character ceiling is enforced rather than only documented.  Both
  `max_message_len` fields said "must not exceed 4 000" with nothing checking
  it; values above the ceiling are now clamped to it, in the direction of the
  stricter limit only.
- The crate compiles warning-free with `ssr` enabled and `csrf` disabled; the
  unread `csrf_token` parameter raised `unused_variable` in that combination,
  which `--all-features` never builds.
- `axum-with-security` serves with
  `into_make_service_with_connect_info::<SocketAddr>()`, so
  `tower_governor`'s `SmartIpKeyExtractor` can fall back to the peer address.
  Without it every request lacking a forwarded-IP header was answered
  `500 Unable To Extract Key!` before reaching a handler.

### Added

- CI job `examples`, a matrix over both example directories running
  `cargo check`.  The examples are excluded from the workspace, so the crate
  gates never compiled them.
- `MESSAGE_MAX_LEN`, the single definition of the 4 000-character message
  ceiling, re-exported at the crate root.  It drives the validator attribute,
  both `Default` implementations, and both clamps.
- `ContactFormOptions::effective_max_message_len`,
  `ContactServerPolicy::effective_max_message_len` — the limit actually in
  force after clamping — and `ContactServerPolicy::check`, which applies the
  whole policy to a normalised input and can report both errors at once.
- CI step `clippy (ssr without csrf)`, gating the feature combination that
  `--all-features` cannot reach, and `RUSTFLAGS: -D warnings` on the
  `examples` job.
- `ContactFieldErrors::from_server_fn_error`, the intended way for a client
  to detect a field-error payload: it matches the `Args` variant instead of
  inspecting the framework's displayed text.  `from_error_str` stays as the
  fallback for callers holding only a string and now locates the sentinel
  anywhere within it rather than only at the start.

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
- `guides/customization.md` no longer warns that the policy counts bytes, and
  states that limits are characters everywhere and clamped to 4 000;
  `reference/api.md` documents `MESSAGE_MAX_LEN`, both
  `effective_max_message_len` methods, and `ContactServerPolicy::check`.

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
