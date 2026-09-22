# RFC 017 — Delivery through an email HTTP API

**Status.** Proposed — 2026-09-22.  Milestone M7, the owner's next-milestone
decision of 2026-09-22 (P-22, one adapter first).
**Tracks.** Roadmap P-22.  Requirements FR-DEL-01 to FR-DEL-08 (a second
built-in backend), FR-OBS-02/03, NFR-PORT-02, §1 scope.
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

### D2 — The adapter

```rust,ignore
pub struct ResendConfig {
    pub api_key: String,          // Debug redacts it
    pub from_address: String,     // a verified sender at the provider
    pub to_address: String,
    pub subject_prefix: String,
    pub timeout: Duration,        // DEFAULT_TIMEOUT = 10 s
}

pub struct ResendDelivery { /* config, and natively one reused client */ }

impl ResendDelivery {
    pub fn new(config: ResendConfig) -> Self;
}

impl ContactDelivery for ResendDelivery { … }   // DeliveryFuture
```

- **Feature:** `delivery-resend = ["ssr", "dep:reqwest", "dep:js-sys",
  "dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys"]`, the same
  set `challenge-http` names.
- **Shape mirrors `SmtpConfig`** so the two read alike: the same field names
  where they mean the same thing, a `DEFAULT_TIMEOUT` constant, and the same
  rustdoc rules about loading secrets from the environment.
- **`Debug`** redacts `api_key`, as the challenge secret's does.

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
  with `RESEND_API_KEY`, `RESEND_FROM`, `RESEND_TO` set, send one real
  message (owner question 5).
- **Break checks:** remove `reply_to` → its test fails; pass the provider's
  error text through → the log test fails.

### D7 — Documentation

Delivery Backends (a section, and the choice between SMTP and the API),
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

## Owner questions (recommendations first)

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
4. **No `Idempotency-Key` in this release.**  A retry is the visitor's, and
   the crate never retries by itself; adding a key needs a stable value to
   derive it from, which is its own design.
5. **A live test, `#[ignore]`d,** run by hand before a release, costing one
   real email to a mailbox you control.  Alternative: no live test, and rely
   on the stubbed responder.
