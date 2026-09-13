# RFC 009 — A bound on delivery time

**Status.** Proposed — 2026-09-13.  Scheduled for 0.6.0 by the owner (P-32
approved); the design decisions below await the owner.
**Tracks.** Roadmap P-32.  Requirements FR-DEL-08 (SHOULD, Gap),
NFR-PERF-03 (Gap).  Threat T15 (Gap).
**Touches.** `delivery/` (a timeout wrapper, the SMTP backend),
`error.rs`, `config.rs` (a label), `Cargo.toml` (tokio `time`), docs.
`server.rs` is not expected to change.

## Summary

A delivery that never finishes holds the visitor's request open until a
proxy gives up, and the visitor sees a proxy error page instead of the
form's message.  This RFC gives every delivery a bound.
- **Built-in SMTP backend.**  It gets a total deadline by default.
- **Custom backends.**  A public wrapper gives any backend the same
  deadline in one line.
- **Visitor message.**  A timeout gets its own code and label, because the
  message may already have been sent.

## Motivation — what bounds delivery today (checked 2026-09-13)

| Backend | Bound today |
|---------|-------------|
| `LettreSmtpDelivery` | lettre's async transport applies its timeout (default 60 s) to the **TCP connect only** (`lettre` 0.11.23, `async_net.rs:171`).  After connecting, TLS, `AUTH`, `MAIL`, `RCPT` and `DATA` have no timeout: a relay that stops answering holds the request indefinitely |
| A custom `ContactDelivery` | none; the delivery guide tells implementers to "wrap slow APIs in a timeout" |
| `submit_contact` | none |

Timers available by feature (normal dependency tree):
- `ssr` alone has **no** tokio.
- `smtp-lettre`, `axum-helpers` and `challenge-http` each bring tokio with
  `time`.  Our own manifest declares only `rt`, so `time` arrives
  transitively.

Proxies in front of a server commonly end a request at 60 s (nginx's
`proxy_read_timeout` default) or 100 s (Cloudflare).  A bound below that
turns a proxy error page into the form's own message.

## Design

### D1 — `DeliveryTimeout`, a wrapper for any backend

```rust,ignore
pub struct DeliveryTimeout<D> { inner: D, limit: Duration }

impl<D: ContactDelivery> DeliveryTimeout<D> {
    pub fn new(inner: D, limit: Duration) -> Self;
}
impl<D: ContactDelivery> ContactDelivery for DeliveryTimeout<D> { … }
```

- **Behaviour.**  `deliver` runs the inner future under
  `tokio::time::timeout`.  On expiry the inner future is dropped, and the
  result is `Err(ContactDeliveryError::Timeout(limit))`.
- **Feature.**  It lives behind a new feature,
  `delivery-timeout = ["ssr", "dep:tokio", "tokio/time"]`.
  `smtp-lettre` enables it.
- **The timer is explicit.**  Our manifest adds `time` to the tokio features
  it uses, instead of relying on lettre to turn it on.
- **Why a wrapper.**  It composes with any backend, including the planned
  HTTP adapters (P-22).  It keeps `submit_contact` runtime-neutral: `ssr`
  alone still needs no tokio.  And the SMTP backend uses the same code, so
  there is one implementation.

### D2 — The SMTP backend has a total deadline by default

- **Config.**  `SmtpConfig` gains `timeout: Duration`, default **30 s**,
  covering connect through the final reply to `DATA`.
- **Implementation.**  The backend applies it with the same code as D1.
  lettre's own connect timeout is set to the same value, so a slow connect
  fails as quickly as a slow relay.
- **Why 30 s.**  It sits under the common proxy limits with room to spare.
  A healthy relay answers in well under a second.
- **Cost.**  `SmtpConfig` is a struct with public fields, so a struct
  literal without `..Default::default()` stops compiling.  That is
  acceptable in a minor release, with a migration line.

### D3 — What the visitor sees

A timeout does not mean "not sent".
- **Before the final `.`.**  If the deadline passes before the relay has
  received the message's final `.`, nothing is delivered.
- **After it.**  If it passes after the relay accepted the message but before
  its reply arrived, the message *was* delivered.
- **The risk.**  The server cannot tell which happened.  Telling the visitor
  "Failed to send" invites a duplicate.

Proposed:
- **Error.**  `ContactDeliveryError::Timeout(Duration)`, a new variant.
- **Wire code.**  `ContactErrorCode::DeliveryTimeout`, spelled
  `delivery_timeout`.
- **Label.**  `ContactErrorLabels::delivery_timeout`, default: "Sending took
  too long. Your message may have been sent — please wait a few minutes
  before trying again."
- **Log.**  `error` level: `contact form delivery timed out` with the limit.
  Never the content.
- **Client compatibility.**  An older client that does not know the code
  falls back to `labels.error`, as for any unknown code (RFC 003).

### D4 — Cancellation contract for implementers

Dropping a future cancels it at its current `.await`.  The `ContactDelivery`
rustdoc and the delivery guide gain a rule:

> **Timeouts.**  A delivery may be cancelled at any `.await` when it is
> wrapped in a timeout.  Do not leave shared state half-updated across an
> `.await`.  An HTTP API call cancelled mid-flight may still complete on the
> vendor's side.

This lands with RFC 010's P-33 section.  Whichever handoff lands second
merges the two.

### D5 — Records

- **FR-DEL-08.**  Met for the built-in backend and for any backend wrapped
  in `DeliveryTimeout`.  The requirement text stays a SHOULD.
- **NFR-PERF-03.**  Met on the same terms.
- **T15.**  Control: the default SMTP deadline, and `DeliveryTimeout` for
  others.  Residual: an unwrapped custom backend.
- **Delivery guide.**  "The call is not time-limited by the crate" is
  replaced with the wrapper and the SMTP default.

## Alternatives considered

| Alternative | Why not |
|-------------|---------|
| A deadline inside `submit_contact` for every backend | Needs a timer in the core, so `ssr` would require tokio; the crate would also impose one policy on backends that already bound themselves |
| Only expose lettre's timeout in `SmtpConfig` | lettre's async transport applies it to the connect only; a stalled relay stays unbounded |
| Document a `tower` timeout layer as the application's job | Bounds the whole request, but the visitor gets a bare `408`/`503` page, not the form's message, and a no-JS visitor loses the round trip |
| Reuse `delivery_failed` on timeout | Says "failed" when the message may have gone; invites duplicates |
| No default deadline, opt-in only | Leaves every existing deployment with the gap the milestone is meant to close |

## Compatibility

Breaking, in 0.6.0 (a minor release), with CHANGELOG migration lines:
- **`SmtpConfig` gains `timeout`.**  Struct literals need the field or
  `..Default::default()`.
- **New enum variants.**  `ContactDeliveryError::Timeout` and
  `ContactErrorCode::DeliveryTimeout` are added.  A `match` without a
  wildcard stops compiling.
- **New label.**  `ContactErrorLabels` gains `delivery_timeout`.
- **Behaviour.**  An SMTP delivery longer than 30 s now fails with
  `delivery_timeout`.

Wire and DOM contract: one new error code; nothing removed.

## Security considerations

- **Availability (T15).**  A slow or malicious relay can no longer pin
  server tasks.
- **Privacy.**  The timeout log names the limit only.
- **Duplicates.**  A retry after a timeout may produce a second email.  That
  is a visible nuisance, not a leak.  D3's message reduces it.

## Testing

- **L1.**
  - `DeliveryTimeout` over a double that never finishes returns `Timeout`
    after a paused-clock advance (`tokio::time::pause`, no real waiting).
  - Over a double that finishes in time, it returns its result.
  - `SmtpConfig::default().timeout` is 30 s.
- **L2.**  A server test with a never-finishing double inside
  `DeliveryTimeout` and a short limit:
  - fetch → `delivery_timeout`;
  - no-JS → the banner shows the new label;
  - logs → the timeout event, with no PII.
- **Traceability.**  FR-DEL-08 and NFR-PERF-03 rows; a SHOULD, so listed
  beside the MUST table as the RFC 008 D5 table allows.
- **Deliberate break.**  Remove the wrapper from the SMTP backend and show
  the default-deadline unit test fail.

## Owner decisions requested

1. **Default for the SMTP backend.**  30 s, on by default (recommended); or a
   different value; or opt-in only.
2. **A distinct `delivery_timeout` code and label** (recommended), or reuse
   `delivery_failed`.
3. **`DeliveryTimeout` public for custom backends** (recommended), or
   internal to the SMTP backend only.

## Release implications

Part of 0.6.0 (RFC 010).  One handoff after acceptance; it touches
`delivery/`, `error.rs` and `config.rs`, so it follows RFC 010 handoff 03,
which edits the same rustdoc.
