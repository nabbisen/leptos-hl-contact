# Handoff 017-02 — The Resend adapter: config, request, errors, tests

**RFC.** [RFC 017](../../accepted/017-http-api-delivery.md) D1a, D2, D3, D4, D6
**Roadmap.** P-22
**Requirements.** FR-DEL-01 to FR-DEL-08, FR-OBS-02, FR-OBS-03, NFR-PORT-02
**Depends on.** 01 (approved)

## Goal

A site can deliver through Resend's HTTP API, natively and on a Worker,
without writing a backend.

## Change scope

### 1. `Cargo.toml`

```toml
delivery-resend = ["ssr", "dep:reqwest", "dep:js-sys", "dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys"]
```

- **The same set `challenge-http` names.**  Check each `dep:` is really
  needed by this path, and say which are shared.
- **`serde_json`** is already a normal dependency; use it to build the
  request body rather than formatting JSON by hand.

### 2. `src/delivery/resend.rs` (new), behind the feature

**The config, per RFC D2 — a constructor and builders, not public fields:**

```rust,ignore
#[non_exhaustive]
pub struct ResendConfig { /* private */ }

impl ResendConfig {
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
    pub fn new(api_key: impl Into<String>, from_address: impl Into<String>, to_address: impl Into<String>) -> Self;
    pub fn with_subject_prefix(self, prefix: impl Into<String>) -> Self;
    pub fn with_timeout(self, timeout: Duration) -> Self;
}
```

- **Defaults:** an empty subject prefix, and `DEFAULT_TIMEOUT`.
- **Rustdoc:** where the key comes from (an environment variable, never
  source), that the sender must be a domain verified at the provider, and
  what happens when a value is missing (fail closed, below).
- **`Debug`** is written by hand: the key is `"<redacted>"`, as
  `HttpChallengeVerifier`'s is.  A test asserts a probe key is absent from
  the `Debug` output.
- **No `serde` derive.**

**The adapter:**

```rust,ignore
pub struct ResendDelivery { /* config, and an HttpClient from handoff 01 */ }
impl ResendDelivery { pub fn new(config: ResendConfig) -> Self; }
impl ContactDelivery for ResendDelivery { fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_>; }
```

**Fail closed, before any request:** an empty `api_key`, `from_address` or
`to_address` returns
`ContactDeliveryError::Configuration("…")` naming **which** value is missing
— the name only, never a value.

**The request** (RFC D3), through handoff 01's module:
- `POST https://api.resend.com/emails`, `HttpBody::Json`;
- header `Authorization: Bearer <key>`;
- body `{"from", "to", "subject", "text", "reply_to"}`:
  - `from` the configured sender, `to` the configured recipient, both never
    from the submission;
  - `reply_to` the visitor's address, as `smtp.rs` sets `Reply-To`;
  - `subject` composed exactly as `smtp.rs` composes it, prefix included;
  - `text` from `delivery/body.rs`, so both backends send the same body.
- **Keep the URL in one `const`,** in the vendor module (RFC D1a).

**The answer:**
- **2xx:** success.  Read the `id` field and log it at `info` as the
  provider's message id, with no other field and nothing from the
  submission.  A missing or unreadable `id` is **not** an error: log the
  success without it.
- **Non-2xx,** per RFC D4: 401 or 403 → `Configuration`; 422 →
  `MessageBuild`; 429 and 5xx → `Transport`; anything else → `Transport`.
  **The message carries the status code and nothing else.**
- **`HttpError::Timeout`** → the crate's timeout error, the same one
  `DeliveryTimeout` and the SMTP deadline produce, so the visitor sees
  "may have been sent".
- **`HttpError::Transport`/`Unusable`** → `Transport`, with the module's own
  text, which never holds a URL.

### 3. `src/delivery.rs`, `src/lib.rs`

- `pub mod resend;` behind the feature, beside `smtp`.
- Re-export `ResendDelivery` and `ResendConfig` where `LettreSmtpDelivery`
  and `SmtpConfig` are re-exported, under the same `#[cfg]` style.

### 4. Tests

**Unit, native, against a local responder** (`challenge/http/tests.rs` shows
the pattern), in `src/delivery/resend/tests.rs`:
- the URL path, the bearer header, and the JSON field names and values,
  including `reply_to` and a subject with the prefix;
- the body text equals `delivery/body.rs`'s output for the same input,
  including a site-defined field;
- each status row of D4, and that the message holds the status and no
  vendor text;
- a 2xx with an `id` logs it; a 2xx without one still succeeds;
- an empty key, sender or recipient is `Configuration` **and sends nothing**
  (the responder records no request);
- `Debug` redaction.

**Worker tests** (`tests/worker/`), against the stubbed `fetch` the challenge
tests already use:
- the same request shape on wasm32;
- a 5xx is `Transport`;
- a dropped delivery aborts its request, as `a_dropped_verification_aborts_its_request` does.

**A log test** in the server suite, with a probe value in every submitted
field: a failing delivery logs the status, and no captured line contains a
probe value or the key.

**Break checks, required:**
- drop `reply_to` from the body → its unit test fails;
- pass the provider's error text into the error → the log test fails.

### 5. `CHANGELOG.md` `[Unreleased]`

**Added:** the adapter, its feature, that it runs natively and on a Worker,
and that it sends the same body as the SMTP backend.

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The MSRV checks** on 1.88.
- **Feature combinations** (RFC D6): `delivery-resend` alone;
  `delivery-resend,challenge-http`; the Workers set plus `delivery-resend` on
  wasm32; and a build with neither, unchanged.
- **Both wasm suites**, with counts.
- **The examples** with `--locked`.
- **CI.**

## Review request

`.git-exclude/review-request/017-http-api-delivery/02-resend-adapter.md`:
- the commit;
- the config's public surface;
- the exact request, with the key redacted;
- the error table as implemented;
- the tests with results;
- both break checks;
- the feature-combination results;
- the gates and CI.
