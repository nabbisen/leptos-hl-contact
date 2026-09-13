# Handoff 009-01 — Delivery deadline, SMTP default, timeout code

**RFC.** [RFC 009](../../done/009-delivery-time-bound.md) D1–D5
**Roadmap.** P-32
**Requirements.** FR-DEL-08, NFR-PERF-03 (SHOULD); FR-CFG-01, FR-SUB-09,
FR-UI-09, FR-I18N-02, FR-OBS-01..03 (MUST rows this touches).  Threat T15.
**Owner decisions (2026-09-13).**
- **SMTP default:** 30 s, on by default.
- **Timeout code:** a distinct `delivery_timeout` code and label.
- **Wrapper:** `DeliveryTimeout` is public.

## Goal

No delivery can hold a request longer than its deadline:
- **The SMTP backend.**  It stops at 30 s unless configured otherwise.
- **Custom backends.**  Any backend gets the same bound by wrapping it in
  `DeliveryTimeout`.
- **The visitor.**  They are told the message *may* have been sent, not
  that it failed.

## Change scope

### 1. Features and dependencies — `crates/leptos-hl-contact/Cargo.toml`

- **New feature.**
  `delivery-timeout = ["ssr", "dep:tokio", "tokio/time"]`.
- **`smtp-lettre`.**  It gains `"delivery-timeout"`.
- **Dev-dependency `tokio`.**  It gains `test-util`, for a paused clock.
  No test may wait in real time for a deadline.

### 2. The wrapper — new `src/delivery/timeout.rs` (tests in `src/delivery/timeout/tests.rs`)

```rust,ignore
/// Bounds any backend's `deliver` by a deadline.
pub struct DeliveryTimeout<D> { inner: D, limit: Duration }

impl<D: ContactDelivery> DeliveryTimeout<D> {
    pub fn new(inner: D, limit: Duration) -> Self;
    pub fn limit(&self) -> Duration;
}
impl<D: ContactDelivery> ContactDelivery for DeliveryTimeout<D> { … }

/// The one implementation, shared with the SMTP backend.
pub(crate) async fn with_deadline<F>(limit: Duration, delivery: F)
    -> Result<(), ContactDeliveryError>
where F: Future<Output = Result<(), ContactDeliveryError>>;
```

- **Behaviour.**  On expiry the inner future is dropped, and the result is
  `Err(ContactDeliveryError::Timeout(limit))`.
- **Module.**  Declared `#[cfg(feature = "delivery-timeout")] pub mod timeout;`
  in `delivery.rs`.
- **Re-export.**  At the crate root, behind the same `cfg`.
- **Debug.**  Implement `Debug`, printing the inner backend's `Debug` and the
  limit.  The inner SMTP backend already redacts its password.

### 3. Errors and codes — `src/error.rs`

- **New variant.**  `ContactDeliveryError::Timeout(Duration)`, with
  `#[error("delivery timed out after {0:?}")]`.
- **New code.**  `ContactErrorCode::DeliveryTimeout`: `as_str` gives
  `"delivery_timeout"`, and `from_str_code` parses it.
- **Doc comment.**  Say the message may have been delivered.

### 4. The label — `src/config.rs`

- **New field.**  `ContactErrorLabels::delivery_timeout`.  Default, exactly:
  "Sending took too long. Your message may have been sent — please wait a few minutes before trying again."
- **Mapping.**  `code_text`: `DeliveryTimeout` renders it.
  `DeliveryFailed | Unexpected` are unchanged.

### 5. The server step — `src/server.rs`, step 9

Match the delivery error.
- **`ContactDeliveryError::Timeout(limit)`:**
  - log `tracing::error!(limit_secs = limit.as_secs(), "contact form delivery timed out")`;
  - return `ServerFnError::ServerError(ContactErrorCode::DeliveryTimeout…)`.
    This is the same `ServerFnError` variant as `delivery_failed`, so both
    forms route it to the banner.
- **Any other error:** unchanged.

RFC 009 originally said `server.rs` would not change.  It must, for this
mapping; the RFC is amended.

### 6. The SMTP backend — `src/delivery/smtp.rs`

- **Config.**  `SmtpConfig` gains `pub timeout: Duration`, with
  `impl SmtpConfig { pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30); }`.
  Document it:
  - it covers connect through the relay's final reply;
  - use `DEFAULT_TIMEOUT` unless the relay is known to be slow;
  - it must be greater than zero.
- **Debug.**  `SmtpConfig`'s `Debug` prints `timeout`.
- **Connect timeout.**  `build_transport` sets `.timeout(Some(self.config.timeout))`
  on each builder, so lettre's connect timeout matches the deadline.
- **Deadline.**  `deliver` runs its whole body (build transport, build
  message, send) inside `with_deadline(self.config.timeout, …)`.
- **Leave the rest.**  Keep the backend's own `SMTP delivery failed` log for
  transport errors.  A timeout is logged once, by the server step.

**`SmtpConfig` has no `Default`, so every struct literal must add the field.**
Add `timeout: SmtpConfig::DEFAULT_TIMEOUT,` in:
- `README.md`;
- `docs/src/getting-started/quick-start.md`;
- `docs/src/guides/delivery-backends.md` (both examples);
- the rustdoc examples in `smtp.rs`;
- `src/delivery/smtp/tests.rs`;
- `docs/src/reference/api.md`.

Also check `examples/`.  None constructs `SmtpConfig` today; confirm it.

### 7. The implementer contract (RFC 009 D4)

- **`ContactDelivery` rustdoc.**  Add a `# Cancellation` section after
  `# Errors`, using D4's rule.
- **`docs/src/guides/delivery-backends.md`**, "Writing your own backend":
  - replace "The call is not time-limited by the crate.  Wrap slow APIs in
    a timeout." with a `DeliveryTimeout::new(MyCustomDelivery, Duration::from_secs(30))`
    example;
  - add the cancellation rule;
  - add one sentence: the SMTP backend already has a deadline.

### 8. Documentation and records

| File | Change |
|------|--------|
| `src/lib.rs` feature table, `docs/src/reference/feature-flags.md` | `delivery-timeout` row (implies `ssr`; enabled by `smtp-lettre`) |
| `docs/src/reference/api.md` | `DeliveryTimeout`, the new variant and code, the label, `SmtpConfig::timeout` and `DEFAULT_TIMEOUT` |
| `docs/src/guides/localization.md` | `delivery_timeout` in the example, in Japanese ("送信に時間がかかりすぎました。メッセージは送信された可能性があります。数分待ってから再度お試しください。"), and in the table |
| `docs/src/development/external-design.md` | §4.2.2 error classes: `delivery_timeout`.  §4.4.1: the Time row, now the SMTP default plus `DeliveryTimeout`.  §4.5.1: the feature.  §4.6: the `delivery timed out` event (`error`, field `limit_secs`).  §5.2 T15: control stated, status **Met**, residual "a custom backend not wrapped in `DeliveryTimeout`" |
| `docs/src/development/requirements.md` | FR-DEL-08 and NFR-PERF-03: "Met (0.6.0): the SMTP backend's 30 s deadline; any backend wrapped in `DeliveryTimeout`".  FR-CFG-01: add `delivery-timeout` to the feature list.  Gap summary: remove the FR-DEL-08 / NFR-PERF-03 row.  Change-history row and status line |
| `docs/src/development/architecture.md` | feature block and source layout: `delivery/timeout.rs` |

### 9. Tests

**L1.**
- **`delivery::timeout::`**, with `#[tokio::test(start_paused = true)]`:
  - `a_delivery_that_never_finishes_times_out`: the result is
    `Timeout(limit)`;
  - `a_delivery_that_finishes_in_time_returns_its_result`: both `Ok` and a
    `Transport` error pass through unchanged.
- **`delivery::smtp::`:**
  - `the_default_timeout_is_thirty_seconds`;
  - `debug_shows_the_timeout`;
  - `a_relay_that_never_answers_times_out`: a `TcpListener` on
    `127.0.0.1:0` accepts and never writes; `DangerousPlaintext`; a short
    limit; the result is `Timeout`.  Prefer a paused clock.  If
    auto-advance does not fire while the socket is pending, use a limit of
    at most 200 ms and say so.
- **`error::`:** `delivery_timeout` round-trips through the wire string.
- **`config::`:** `code_text` renders `delivery_timeout`, and a translated
  label substitutes.

**L2, `tests/server/`.**
- **The double.**  A `NeverDelivery` in `support/doubles.rs`: its future
  never completes.
- **`delivery::a_delivery_timeout_reaches_the_client_as_delivery_timeout`**
  (paused clock).  Use `DeliveryTimeout::new(NeverDelivery, 50 ms)` as the
  delivery context.  Assert:
  - fetch → `delivery_timeout`;
  - no-JS → error redirect, and the banner is the default label;
  - zero deliveries.
- **`logging::no_personal_data_or_secret_is_logged`.**  Add the timeout
  outcome, and assert `contact form delivery timed out` is captured.  The
  PII sweep covers it.

**Traceability.**
- **FR-SUB-09, FR-UI-09, FR-I18N-02, FR-OBS-01 rows:** gain the new tests.
- **FR-DEL-08, NFR-PERF-03:** these are SHOULD rows.  Cite them in the test
  doc comments.

### 10. CHANGELOG `[Unreleased]`

- **Added.**  `DeliveryTimeout` and the `delivery-timeout` feature, the
  `delivery_timeout` code and label, and `SmtpConfig::timeout` with
  `DEFAULT_TIMEOUT`.
- **Changed.**  The SMTP backend stops at 30 s by default, and reports
  `delivery_timeout`.
- **Migration.**
  - Add `timeout: SmtpConfig::DEFAULT_TIMEOUT` to `SmtpConfig { … }`.
  - A `match` on `ContactDeliveryError` or `ContactErrorCode` without a
    wildcard needs the new variant.
  - A `ContactErrorLabels { … }` literal needs `delivery_timeout`, or
    `..Default::default()`.

## Acceptance

1. All tests above pass.  No test waits for a deadline in real time, except
   the relay test if it has to, as allowed above.
2. **Deliberate break A.**  In `LettreSmtpDelivery::deliver`, call the body
   without `with_deadline`.  `a_relay_that_never_answers_times_out` fails,
   hanging until the test harness's own limit is acceptable: say how you
   bounded it.  Then restore.
3. **Deliberate break B.**  In `server.rs`, map `Timeout` to
   `DeliveryFailed`.  The server test fails.  Then restore.
4. **Builds.**  `ssr` alone still builds without tokio: run
   `cargo tree -p leptos-hl-contact --no-default-features --features ssr -e normal -i tokio`
   and show it prints nothing.  `delivery-timeout` alone builds and passes
   clippy.
5. **Gates.**  All pass, plus both suites.  The `examples` job is green.
6. **Grep.**  `git grep -n 'not time-limited'` finds nothing outside history.

## Review request

`.git-exclude/review-request/009-delivery-time-bound/01-delivery-deadline.md`
