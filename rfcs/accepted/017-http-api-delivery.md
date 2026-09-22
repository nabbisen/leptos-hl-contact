# RFC 017 — Delivery through an email HTTP API

**Status.** Accepted — 2026-09-22, as revised (all five recommendations
accepted).  Milestone M7, the owner's next-milestone decision of 2026-09-22
(P-22, one adapter first).
**Tracks.** Roadmap P-22.  Requirements FR-DEL-01 to FR-DEL-08 (a second
built-in backend), FR-OBS-02/03, NFR-PORT-02, §1 scope.
**Handoffs.** [`../handoffs/017-http-api-delivery/README.md`](../handoffs/017-http-api-delivery/README.md)
**Touches.** `delivery/` (a new adapter and a shared body builder), a shared
private HTTP module used by this and `challenge-http`, `Cargo.toml`, tests at
every layer, docs, `CHANGELOG.md`.

## Summary

Today a site that does not run SMTP must write its own `ContactDelivery`.  On
Cloudflare Workers there is **no** built-in option at all: SMTP needs tokio,
and the reflerd.com team had to write a backend over raw sockets.

This RFC adds **one** built-in backend that posts the enquiry to an email
provider's HTTP API, and works both natively and on a Worker.

**Not in this RFC:** a second provider, attachments, templates, scheduling,
bulk sending, or inbound mail.

## Why now, and why one provider

- **The gap is real.**  Workers is a supported target since 0.7.0, and the
  only built-in backend cannot run there.
- **The machinery exists.**  RFC 011 built the wasm32 `fetch` path with a
  time limit, no redirects and abort-on-drop, beside the native `reqwest`
  path, for challenge verification.  A delivery adapter needs exactly that.
- **One provider first** keeps the surface small and lets the second RFC copy
  a proven shape.

### The choice of provider (owner question 1)

| Provider | Request | Fits a Worker? | Notes |
|----------|---------|----------------|-------|
| **Resend** (recommended) | `POST https://api.resend.com/emails`, `Authorization: Bearer …`, JSON `{from, to, subject, text, reply_to}` | yes: one JSON POST | `reply_to` is a first-class field, which the contact form needs; the answer is `{"id": …}`; an `Idempotency-Key` header exists |
| SendGrid | `POST https://api.sendgrid.com/v3/mail/send`, `Authorization: Bearer …`, JSON with `personalizations`, `from`, `subject`, `content`; answers `202` | yes | a more nested body for no gain here |
| AWS SES | SigV4-signed request | poorly | request signing (HMAC chain, canonical requests) is a security-sensitive dependency of its own |

**Checked 2026-09-22** against each vendor's API reference.

**Recommendation: Resend first**, SendGrid as a later RFC that reuses the
shape, SES not planned.

## Design

### D1 — A shared, private HTTP layer

- **Today** `challenge/http.rs` holds a native `reqwest` path and
  `challenge/http/fetch.rs` a wasm32 `fetch` path, both specific to a
  form-encoded POST that returns JSON.
- **Change:** extract a private `src/http/` module with one small function:
  a POST with a URL, headers, a body, and a time limit, returning the status
  and the body text.
  - **Keeps:** no redirects (`redirect: "manual"` on wasm32, `Policy::none()`
    natively), the abort-on-drop guard, the timeout, and the rule that no
    secret appears in an error.
  - **Used by:** `challenge-http` (form-encoded) and this adapter (JSON with
    a bearer header).
  - **Not public.**  It is an implementation detail; no type of it appears in
    the public API.
- **The challenge path must not change behaviour.**  Its existing unit,
  worker and live tests are the proof, and they stay as they are.
- **If the extraction turns out to be more than a move** (for example because
  the two paths differ more than they appear), stop and report: duplicating a
  small amount of code is better than a shared abstraction that bends.

### D1a — Where the vendor's specifics live

- **One private module** holds everything Resend-specific: the URL, the
  bearer header, the JSON field names and the status mapping.  Everything
  else — the transport, the body text, the trait plumbing — is vendor-neutral
  already.
- **No public provider abstraction yet.**  A second adapter should be able to
  reuse the seam, but inventing an enum or a trait for one implementation
  would be a framework built for a single user.  RFC for SendGrid decides
  whether the seam becomes public.

### D2 — The adapter, and a config that will not break literals again

```rust,ignore
#[non_exhaustive]
pub struct ResendConfig { /* private fields */ }

impl ResendConfig {
    /// The three values a site must give.  Everything else has a default.
    pub fn new(
        api_key: impl Into<String>,
        from_address: impl Into<String>,
        to_address: impl Into<String>,
    ) -> Self;

    pub fn with_subject_prefix(self, prefix: impl Into<String>) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;

    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
}

pub struct ResendDelivery { /* the config, and natively one reused client */ }

impl ResendDelivery {
    pub fn new(config: ResendConfig) -> Self;
}

impl ContactDelivery for ResendDelivery { … }   // DeliveryFuture
```

**Why not `SmtpConfig`'s shape, which this would otherwise copy.**
- **We have paid for it twice.**  `SmtpConfig` is a plain struct of public
  fields with no `Default`, so adding `timeout` in 0.6.0 forced a migration
  line and an edit in every integrator's literal.  0.8.0 did the same to
  three more structs.
- **A constructor with builders and `#[non_exhaustive]`** makes a future
  field additive: no migration line, no edits, and the compiler still
  refuses a half-built config because the required three are arguments.
- **It reads the way the crate's other configurable type already does:**
  `ChallengeWidget::new(provider, site_key)?.with_theme(…).with_size(…)`.
  An integrator meets one pattern, not two.
- **`SmtpConfig` is not changed** by this RFC.  Aligning it is a separate
  question with its own migration; this RFC only stops the debt growing.

**Fail closed on a missing key**, the lesson RFC 013 D4 wrote down for the
challenge secret:
- **An empty `api_key` is accepted at construction** and reported as
  `Configuration` on the first delivery, so a missing environment variable
  fails closed rather than at start-up, which a Worker does not have.
- **The same for an empty `from_address` or `to_address`.**
- **Never a silent success.**

**Other rules:**
- **Feature:** `delivery-resend = ["ssr", "dep:reqwest", "dep:js-sys",
  "dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys"]`, the set
  `challenge-http` already names.
- **`Debug` redacts `api_key`,** as the challenge secret's does, and the
  config derives no `serde` trait, so it cannot be serialised into a log by
  accident.
- **No retries.**  A failed delivery is the visitor's to retry, as with
  challenge verification.

### D3 — What it sends

- **Body text:** the same plain-text body the SMTP backend builds, including
  the site-defined fields of RFC 015.  `build_plain_text_body` moves from
  `delivery/smtp.rs` to a shared `delivery/body.rs`, unchanged, and both
  backends call it.  A test pins that the two bodies are identical for the
  same input.
- **The request:**
  - `from`: the configured sender, never the visitor;
  - `to`: the configured recipient;
  - `reply_to`: the visitor's address, as the SMTP backend sets `Reply-To`;
  - `subject`: `subject_prefix` plus the enquiry's subject, exactly as SMTP
    composes it;
  - `text`: the body.
- **No HTML part.**  The crate has never sent one.
- **No `Idempotency-Key`** in this RFC (owner question 4).

### D4 — Errors, and what may appear in them

| Case | `ContactDeliveryError` | What the error text may contain |
|------|------------------------|---------------------------------|
| 2xx | — (success) | — |
| 401, 403 | `Configuration` | the status code |
| 422 | `MessageBuild` | the status code |
| 429, 5xx | `Transport` | the status code |
| a network failure | `Transport` | the class of failure, never a URL with a key |
| the time limit | the crate's existing timeout error | — |

- **Never in an error or a log:** the API key, the visitor's address, the
  subject, the message, or any site-field value.  FR-OBS-03 already says a
  delivery error's text is logged, so this is a hard rule, and a test greps
  the captured logs for a probe value.
- **The provider's own error text is not passed through.**  It can echo
  submitted content.
- **On success, the provider's message id is logged at `info`.**  It is an
  opaque identifier, not content, and it is what an operator needs to trace
  an enquiry the recipient says never arrived.  Nothing else from the
  answer is read.

### D5 — Workers

- **It runs on a Worker,** which is the point: no tokio, one `fetch`.
- **The Workers guide** gains this as the recommended delivery for a site
  without its own backend, replacing "bring your own".
- **`DeliveryTimeout`** still composes, and the adapter's own limit applies
  first.

### D6 — Tests

- **Unit, native, against a local responder** (the pattern
  `challenge/http/tests.rs` already uses): the URL, the bearer header, the
  JSON field names and values, the body text, `reply_to`, and each error
  row of D4.
- **Worker tests** against a stubbed `fetch`: the same request shape, a
  success, a 5xx, and abort-on-drop.
- **A log test:** a failing delivery logs the status and not the probe
  values.
- **A live test, `#[ignore]`d,** beside the live vendor challenge tests:
  with `RESEND_API_KEY` and `RESEND_FROM` set, send one real request to
  Resend's **documented test recipient**, `delivered@resend.dev`, which
  simulates a delivered message (Resend, "Send test emails", checked
  2026-09-22).  No real mailbox is involved, and `RESEND_TO` can override it
  for a site that wants a real one (owner question 5).
- **Break checks:** remove `reply_to` → its test fails; pass the provider's
  error text through → the log test fails.
- **Feature combinations, in the gates:** `delivery-resend` alone;
  `delivery-resend` with `challenge-http`; and the Workers set with it on
  wasm32.  The two features share optional dependencies, so a wrong `dep:`
  list only shows up in the combination that omits the other.

### D7 — Documentation

**The one thing an integrator must not have to guess is which backend to
use.**  Delivery Backends opens with a short table — SMTP (native only),
Resend (native and Workers), your own (any target) — and the Workers guide
stops saying "bring your own backend", which has been its advice since
0.7.0.  Also:
Cloudflare Workers, Feature Flags, the API reference, Production Checklist,
README, and `development/` (external design's delivery table, architecture's
layout, traceability).

## Compatibility

Additive.
- **No existing API changes.**
- **New optional dependencies** only under the new feature, all already in
  the tree behind `challenge-http`.
- **The shared HTTP module and body builder are private moves,** with no
  behaviour change; the challenge suites prove it.

## Handoffs (planned)

| # | Scope |
|---|-------|
| 01 | D1: extract the private HTTP module and `delivery/body.rs`; no new behaviour; the challenge suites unchanged |
| 02 | D2–D4: the adapter, its config and errors, with unit and worker tests |
| 03 | D5–D7: Workers guidance, documentation, traceability, the live test |

## Owner decisions (2026-09-22)

Accepted as revised: Resend first; `delivery-resend`, `ResendDelivery`,
`ResendConfig`; the shared HTTP module extracted in its own handoff; no
`Idempotency-Key`; an `#[ignore]`d live test to the provider's test
recipient.  The revisions of the second review — a builder-and-
`#[non_exhaustive]` config, fail-closed on an empty key, the private vendor
seam, the message id logged, feature-combination gates, and the "which
backend" table — are part of the acceptance.

### The questions as put

1. **Resend as the first provider.**  Alternative: SendGrid, which is more
   widely deployed but has a more nested request and no first-class
   `reply_to`.  A second adapter can follow either way.
2. **Names:** feature `delivery-resend`, types `ResendDelivery` and
   `ResendConfig`.  Alternative: a neutral `HttpEmailDelivery` with a
   provider enum, which would invite provider-specific options into one
   type.
3. **Extract the shared HTTP module (D1)** before the adapter, rather than
   writing a second copy.  It touches working challenge code, so the first
   handoff is a pure refactor with the existing suites as the gate.
4. **No `Idempotency-Key` in this release.**  I checked what it would buy:
   the header dedupes *repeats of the same key*, and the crate never retries
   by itself, so the only repeat is the visitor pressing send again — a new
   submission, with no key we could legitimately reuse.  It would therefore
   change nothing about the one case that worries us, a timeout after the
   provider accepted the message.  The visitor's message already says the
   enquiry may have been sent.
5. **A live test, `#[ignore]`d,** run by hand before a release, costing one
   real email to a mailbox you control.  Alternative: no live test, and rely
   on the stubbed responder.
