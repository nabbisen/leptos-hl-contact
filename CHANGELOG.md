# Changelog

## [Unreleased]

No version assigned; the owner decides the release number.

### Removed

- **The deprecated 0.4 names**, as the 0.5.0 release promised: the `csrf`
  feature, the `csrf` module with `CsrfConfig`, `CsrfToken`,
  `CsrfConfigContext`, `generate_csrf_token` and `verify_csrf_token`, and the
  `csrf_token` argument of `submit_contact`.  A page rendered by 0.4, which
  posts only `csrf_token`, is now refused as `token_invalid`; a 0.5 page is
  unaffected.

### Changed

- **Stricter email addresses.**  `email` is refused, with the existing
  `format_email` text, when its domain is an address literal
  (`user@[127.0.0.1]`), a single label (`user@localhost`), or has an empty
  label (`user@example.`); and with the `length` text when it is over 254
  characters, the limit the form's `maxlength` already applied.  No new
  error code or label.

### Migration

**Newly refused addresses.**  `user@localhost`, `user@[127.0.0.1]` and
addresses over 254 characters are now refused; they cannot be replied to
from a public mailbox.

| 0.4 name | Use |
|----------|-----|
| feature `csrf` | `form-token` |
| module `csrf` | `form_token` |
| `CsrfConfig` | `FormTokenConfig` |
| `CsrfToken` | `FormToken` |
| `CsrfConfigContext` | `FormTokenContext` |
| `generate_csrf_token` | `issue_form_token` |
| `verify_csrf_token(token, cfg) -> bool` | `verify_form_token(token, bound, cfg) -> Result<(), FormTokenError>` |
| hidden field `csrf_token` | `form_token` |
| env var `CSRF_SECRET` (docs and examples) | `FORM_TOKEN_SECRET` |

### Documentation

- Development: a server integration suite in `crates/leptos-hl-contact/tests/server/`
  now tests the crate through a router built exactly as the documentation
  instructs.  It covers delivery and delivery errors, the success page, every silent outcome,
  validation and the no-JavaScript round trip, server policy, the form token
  and its cookie binding, the challenge decision table, the filter, and log
  hygiene.  Tests and development dependencies only; the published crate is
  unchanged.  `development/testing.md` describes the harness.
- Development: browser tests in `crates/leptos-hl-contact/tests/browser/` mount
  `ContactForm` in headless Chrome.  They cover focus after a field error,
  the form token's acquisition and refresh (with a clock the test
  controls), the token surviving a failed submission, and explicit rendering
  of a challenge widget.  Server functions are answered by a stubbed `fetch`
  and no vendor script is loaded.  CI runs them in a new `browser` job on
  every push.  Tests, development dependencies and CI only; the published
  crate is unchanged.
- Development: tests for the rendered accessibility attributes (labels,
  `required` and `aria-required`, the hidden honeypot, `maxlength`), the SMTP
  password's redaction, and the submit button's pending state; CI also builds
  the crate for wasm32 with default features.  `development/testing.md`'s
  requirement table now lists 102 MUST rows.

## [0.5.0] — 2026-09-13

### Security

- **Honeypot detection could be observed (affects 0.4.0).**  With a success
  page configured (`ContactSuccessRedirect`), a submission caught by the
  honeypot returned success without the redirect, so an automated sender
  could tell it had been detected by comparing responses.  Every successful
  outcome — honeypot, filter `SilentDrop`, and delivery — now applies the
  success redirect.  Sites using 0.4.0 with a success page should upgrade.
  Deployments without a success page were not affected: every successful
  outcome already returned the same response.

### Changed

- **The `csrf` feature, module and items are renamed to `form-token` /
  `form_token`.**  The token proves the sender fetched a page from this
  server recently; it is not a CSRF control on its own, and the name said
  otherwise.  Origin validation remains the CSRF control.
- **The hidden field is `form_token`, not `csrf_token`.**  The DOM contract
  is public API, so this is the breaking change that makes the release a
  minor.  `submit_contact` accepts the old field for one minor, so a page
  rendered by 0.4 still submits successfully to 0.5.

### Added

- **A minimum age.**  `FormTokenConfig::min_age_secs`, two seconds by
  default, rejects a submission that arrives sooner than that after the page
  was rendered — a script's speed, not a person's.  The rejection is
  retryable by design: the same token is valid a moment later, so somebody
  who autofilled and clicked immediately is delayed by one attempt rather
  than turned away.  `0` disables the check.
- `ContactErrorCode::TooFast` and `ContactErrorLabels::too_fast`
  ("Please wait a moment and try again.") for that case, so it is
  distinguishable from a forged token and translatable like every other
  message.
- `FormTokenError`, which reports *why* a token failed.
- **Cookie binding.**  `Binding::Cookie` ties the token to the browser that
  was issued it: the token's nonce — never the token, never the secret — is
  also written to an `HttpOnly`, `SameSite=Lax` cookie, and verification
  requires both halves to agree.  That adds a second check against
  cross-site submissions; it is defence in depth, and Origin validation
  remains the control that rejects cross-site POSTs.  Opt-in;
  `Binding::None` remains the default.
- **The binding cookie is `__Host-hl_contact_ft` at the defaults.**
  Browsers refuse a `__Host-` cookie that names a `Domain`, so a sibling
  subdomain cannot plant a value of its choosing.  The prefix is added only
  when `secure` is `true` and `path` is `/`, as the prefix requires; the
  local-HTTP override keeps the plain name.
- `axum_helpers::FormTokenCookie`, `provide_form_token_with_cookie` and
  `provide_form_token_binding` wire it in the one context closure.  Issuing
  happens on `GET` only, so a submission never overwrites the cookie it is
  bound to, and an existing cookie is reused rather than replaced, so the
  value is stable per browser: several open forms all submit, and a form
  left open while the visitor browsed elsewhere still works.  Any other
  framework can provide `FormTokenBinding` itself.
- `issue_form_token_with_nonce`, for wiring binding into a framework other
  than Axum: it signs a nonce the browser already holds, and refuses
  anything that is not 32 hex characters.
- **Browser-side form tokens, off by default.**
  `ContactFormOptions::token_refresh_secs` (default `None`) is the switch.
  Set it to `ttl_secs - 60` when the server issues form tokens, and:
  - a form reached by client-side navigation, which arrives with an empty
    token field, fetches one from the new `issue_form_token_fn` server
    function (`POST /api/form_token`);
  - the token is refreshed before it expires — once immediately if the
    mounted token is already overdue, and after that timed from each new
    token's arrival, so a wrong browser clock cannot cause a request loop.

  With `None` the browser never calls that endpoint; a server-rendered form
  behaves as before.  With binding on, the fetched token reuses the
  browser's nonce, and `axum_helpers::provide_form_token_issuer` re-sends
  the cookie through the new `FormTokenIssuer` context.
- `FormTokenConfig` builders `with_ttl`, `with_min_age`, `with_binding`.
- **Challenge verification (server side).**  `submit_contact` accepts the
  vendor token fields `cf-turnstile-response`, `h-captcha-response` and
  `g-recaptcha-response`, and verifies the first non-blank one through a
  `ChallengeVerifier` provided in `ChallengeContext`, after every local check
  and before delivery.  Fail-closed: a token with no verifier is
  `not_configured`, a verifier error is `challenge_unavailable`.
  `NoJsPolicy` decides whether a submission without a token is rejected
  (`challenge_required`, the default) or accepted on the honeypot alone.
  New codes `challenge_required`, `challenge_failed`,
  `challenge_unavailable`, with matching labels and `challenge_requires_js`.
  No HTTP client and no new dependency yet; the built-in vendor verifiers
  come separately.
- **Challenge widget.**  `ContactForm` takes an optional `challenge` prop, an
  `Option<ChallengeWidget>` for Turnstile, hCaptcha, reCAPTCHA v2 or v3, and renders
  the vendor widget and script after the message field.  Every value that
  reaches the page is validated when the widget is built.  reCAPTCHA v3 gets
  an inline script that fetches a fresh token on every submit.  Without
  JavaScript, `NoJsPolicy::Reject` shows `challenge_requires_js`.  A widget
  reached by client-side navigation is rendered explicitly, and the vendor
  script is never loaded twice.  Without the prop, nothing is loaded from any
  vendor.
- **Built-in challenge verifiers** (feature `challenge-http`).
  `HttpChallengeVerifier` calls Turnstile's, hCaptcha's or reCAPTCHA's
  siteverify endpoint with one reused `reqwest` client (rustls only), a
  five-second cap, and no retries.  A timeout is `Timeout`; a network error,
  a non-2xx status or a response without a boolean `success` is
  `Unavailable`; an empty secret is `Misconfigured`.  All three reject the
  submission as `challenge_unavailable`.  The secret is redacted from
  `Debug`, and `with_verify_url` supports a forwarding proxy.  Redirects are
  not followed, so the secret is never resent to a `Location` header.
- **Filter hook.**  A `ContactFilter` provided as `ContactFilterContext`
  sees the validated submission after the challenge and before delivery,
  and returns `FilterDecision::Accept`, `Reject` (`contact_error:rejected`,
  label `rejected`: "Your message could not be accepted.") or `SilentDrop`
  (success without delivery, like the honeypot).  `FilterChain` runs several
  in order; the first non-`Accept` wins.  Decisions are logged at `warn`
  with the deciding filter's name, never with content.  The crate ships no
  filters; `security/filter.md` has examples.

### Migration

Every old name still works, warns, and is removed in the next minor.

| 0.4 | 0.5 |
|-----|-----|
| feature `csrf` | `form-token` (`csrf` kept as an alias) |
| module `csrf` | `form_token` |
| `CsrfConfig` | `FormTokenConfig` |
| `CsrfToken` | `FormToken` |
| `CsrfConfigContext` | `FormTokenContext` |
| `generate_csrf_token` | `issue_form_token` |
| `verify_csrf_token(token, cfg) -> bool` | `verify_form_token(token, bound, cfg) -> Result<(), FormTokenError>` |
| hidden field `csrf_token` | `form_token` |
| env var `CSRF_SECRET` (docs and examples) | `FORM_TOKEN_SECRET` |

Two things do not move by themselves:

- **A `CsrfConfig { secret_key, token_ttl_secs }` struct literal no longer
  compiles.**  `token_ttl_secs` is now `ttl_secs` and two fields were added;
  use `FormTokenConfig::new(secret)` with the `with_*` builders.
- **`min_age_secs` defaults to 2**, which is new behaviour.  A test that
  submits instantly will now see `too_fast`; call `.with_min_age(0)` for the
  old behaviour.
- **Two structs gain fields.**  `ContactFormOptions` gains
  `token_refresh_secs`; `ContactErrorLabels` gains `too_fast`,
  `challenge_required`, `challenge_failed`, `challenge_unavailable`,
  `challenge_requires_js` and `rejected`.  An exhaustive struct literal of either must add
  them; `..Default::default()` is unaffected.  `token_refresh_secs` defaults
  to `None`, which is off.

### Documentation

- The Security overview has one table, "Which layer decides what", mapping
  each of the five mechanisms to the question it answers, how it is
  configured, and what the visitor sees; each mechanism is explained on its
  own page only.  New page `security/filter.md`.
- `security/turnstile.md` is now `security/challenge.md`, covering all four
  providers, the server's decision table, the no-JavaScript policy and its
  weakness, CSP and privacy per vendor, and the vendors' test keys.  The old
  URL redirects.
- `security/csrf.md` → `security/form-token.md`, rewritten: the guarantees
  table has a column per binding mode, the minimum age is explained as a
  deliberate one-retry cost to a fast human, and a migration section lists
  every old → new name.  The book redirects the old URL.

## [0.4.0] — 2026-09-13

### Fixed

- The hidden anti-automation token survives a failed submission in the
  browser.  The form subtree was rebuilt whenever the action's value changed,
  and the rebuild rewrote the hidden field from the client, where no token
  context exists — so the next submit failed with "Invalid or expired
  security token. Please reload the page."  The `<form>` and its inputs are
  now created once: only the success region, the generic-error region, and
  each field's error paragraph and ARIA attributes react.

- A required field left empty now renders the `required` label instead of the
  length label.  `validator` reports a blank value and a too-short one
  identically, as `length` with `min: 1`, so a visitor who simply left the
  box empty was told it "must be between 1 and 4000 characters".

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

- `FieldErrorCode`, `FieldError`, `ContactErrorCode`, `ContactField` and
  `ContactErrorLabels`, all re-exported at the crate root, plus
  `ContactFieldErrors::get`, `ContactErrorCode::from_server_fn_error` and
  the `CONTACT_ERROR_PREFIX` sentinel.
- `ContactErrorLabels::field_text` and `code_text` render a code into text;
  `length` may use the `{min}` and `{max}` placeholders, replaced by plain
  substitution with no format syntax.

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

  If you translated `ContactFormLabels::error` but not the new `errors`
  block, visitors will now see English error text: `error` is reached only
  for a payload the client does not recognise, and every message the current
  server sends has its own label in `errors`.

  On the wire, `field_errors:` members become objects such as
  `{"kind":"length","min":1,"max":80}`, and whole-submission failures become
  `contact_error:<code>`.  `FieldError` is `serde(untagged)`, so a current
  client still reads a `0.3` server's sentences and shows them unchanged.
  A `0.3` client reading a current server falls back to its generic banner,
  which is what it already showed for every validation error.

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
