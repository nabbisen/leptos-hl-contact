# Handoff 011-03 — Challenge verification over `fetch`; the visitor's IP

**RFC.** [RFC 011](../../accepted/011-cloudflare-workers.md) D3, D5
**Roadmap.** P-23, P-40
**Requirements.** FR-ABUSE-10, FR-ABUSE-12, NFR-PRIV-02, FR-OBS-02, NFR-PORT-02
**Depends on.** Handoff 02 merged, for its `wasm_timer`.

## Goal

1. **`HttpChallengeVerifier` works on a wasm32 server** through `fetch`, with
   the same guarantees as natively:
   - redirects are never followed;
   - a time limit applies;
   - an empty secret sends nothing.
2. **A site can pass the visitor's IP** to verification, and the built-in
   verifier sends it as `remoteip`.

## Change scope

### 1. The request type and the IP context (D5) — `src/challenge.rs`

```rust,ignore
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ChallengeRequest<'a> {
    pub token: &'a str,
    pub remote_ip: Option<std::net::IpAddr>,
}
impl<'a> ChallengeRequest<'a> {
    pub fn new(token: &'a str) -> Self;
    pub fn with_remote_ip(self, ip: std::net::IpAddr) -> Self;
}

/// The visitor's IP for this request, provided by the site.
#[derive(Clone, Copy, Debug)]
pub struct ChallengeClientIp(pub std::net::IpAddr);
```

**`ChallengeVerifier`** gains a method with a default implementation:

```rust,ignore
fn verify_request(&self, request: &ChallengeRequest<'_>) -> VerifyFuture<'_> {
    self.verify(request.token)
}
```

**Rustdoc on `ChallengeClientIp`:**
- The crate never reads headers.
- The site takes the IP from what its proxy guarantees: `CF-Connecting-IP`
  behind Cloudflare, the peer address with no proxy.
- The IP is PII and is never logged.

**Re-export** both types at the crate root, beside the other challenge types.

### 2. The server step — `src/server.rs`, step 7

- **Read** `use_context::<ChallengeClientIp>()`.
- **Call** `ctx.verifier.verify_request(&request)`, where `request` is
  `ChallengeRequest::new(&token)`, plus `with_remote_ip` when the context
  is present.
- **Keep** the handoff 01 `SendWrapper` on that await.
- **No log line** may include the IP.

### 3. `HttpChallengeVerifier` — `src/challenge/http.rs`

**Shared by both paths:**
- **`fn form_body(&self, request) -> Vec<(&str, String)>`:** `secret`,
  `response`, and `remoteip` when present.
- **`verify`** delegates to `verify_request(&ChallengeRequest::new(token))`.
  Implement `verify_request` for real.
- **The empty-secret check,** `parse_response`, and the `Debug` redaction.

**Confirm the vendors' parameter name.**  Before sending `remoteip` for
hCaptcha and reCAPTCHA, confirm that name in each vendor's siteverify
documentation.  Cite the URLs in the review request.  If a vendor does not
accept it, omit it for that vendor and say so.

**Native path:** unchanged apart from the body.  reqwest with
`Policy::none()` and `.timeout(self.timeout)`.
- **Dependency.**  `reqwest` moves to a native-only optional dependency
  table.  `challenge-http` keeps naming it.
- **Fields.**  The `client` field and `transport_error` become native-only.

**wasm32 server path:**
- **Request.**  Build a `web_sys::Request` with `RequestInit`:
  - `method` `POST`;
  - header `Content-Type: application/x-www-form-urlencoded`;
  - body the URL-encoded form;
  - **`redirect` set to `RequestRedirect::Manual`**;
  - `signal` from an `AbortController`.
- **Time limit.**  Abort that controller when `wasm_timer::sleep(self.timeout)`
  finishes first.  Record that the abort was ours, so it maps to `Timeout`.
- **Dispatch.**  Call `fetch` from `js_sys::global()`, not `window()`.
- **Status.**  Any status outside `200..=299` is
  `Unavailable(format!("HTTP {status}"))`.  That covers a 3xx in workerd
  and status 0 for a browser's opaque redirect.
- **Body.**  `array_buffer()` into `parse_response`.
- **Errors.**
  - A rejected `fetch` that was not our abort is `Unavailable`, with a
    message that contains no URL.
  - Our abort is `Timeout`.
- **Dependencies.**  `challenge-http` enables `web-sys` with the needed
  features (`Request`, `RequestInit`, `RequestRedirect`, `Headers`,
  `Response`, `AbortController`, `AbortSignal`), plus `js-sys`,
  `wasm-bindgen` and `wasm-bindgen-futures`.  All are wasm32-only.

### 3a. Carried over from the handoff 02 review

- **Widen the timer's `cfg`.**  `wasm_timer` is compiled under
  `all(target_arch = "wasm32", feature = "delivery-timeout")`.  Make it
  `all(target_arch = "wasm32", any(feature = "delivery-timeout", feature = "challenge-http"))`.
  Let `challenge-http` enable `js-sys` and `wasm-bindgen` on wasm32, as
  `delivery-timeout` does.  Confirm with the Workers check for
  `ssr,challenge-http` alone.
- **`wasm-bindgen-futures` for `fetch`.**  Awaiting the `fetch` promise
  through `JsFuture` is fine: a `fetch` promise always settles, including
  when aborted, so its callbacks are freed.  That is unlike the cleared timer
  handoff 02 avoided.  Add `wasm-bindgen-futures` as a wasm32-only optional
  dependency enabled by `challenge-http`.
- **Abort on drop.**  Hold the `AbortController` in a guard.  If the
  verification future is dropped before the response arrives (the request
  was cancelled), the guard's `Drop` aborts it, so no request runs on.
  Once the response has arrived, `Drop` does nothing.  Test it in
  `tests/worker`:
  - **`fetch_verifier::a_dropped_verification_aborts_its_request`:** the stub
    records the request's `AbortSignal`; drop the verification future after
    one poll; the signal reports `aborted`.

### 4. CI

Add `challenge-http` to handoff 01's Workers step and to handoff 02's
browser-job step.

## Tests

**Native, L1 (`challenge/http/tests.rs`, existing local responder):**
- **`the_form_body_carries_remoteip_when_provided`:** the responder captures
  the body; with an IP it contains `remoteip=<ip>`; without, no `remoteip`.
- **`verify_is_verify_request_without_an_ip`:** the body is identical for
  both calls.

**Native, L2 (`tests/server/challenge.rs`):**
- **`the_client_ip_reaches_the_verifier`:**
  - `ScriptedVerifier` implements `verify_request` and records
    `remote_ip`;
  - with `ChallengeClientIp` provided, it sees that IP;
  - without it, it sees `None`;
  - both request forms.
- **Logging.**  `logging::no_personal_data_or_secret_is_logged` adds a
  challenge outcome with a client IP and forbids the IP's text in every
  event.

**`tests/worker/` (headless Chrome, a stubbed `fetch`):**
- **`fetch_verifier::the_request_refuses_redirects_and_posts_the_form`:**
  - the stub records the `Request`;
  - method `POST`;
  - `redirect` `"manual"`;
  - content type form-encoded;
  - body with `secret`, `response`, `remoteip`.
- **`fetch_verifier::a_redirect_is_unavailable`:** the stub answers 302 →
  `Unavailable`.
- **`fetch_verifier::a_verdict_is_parsed`:** 200 with
  `{"success":true}` → passed; `{"success":false,"error-codes":["x"]}` → not
  passed, with codes.
- **`fetch_verifier::a_silent_vendor_is_a_timeout`:** a stub that never
  resolves and `with_timeout(50 ms)` → `Timeout`.
- **`fetch_verifier::an_empty_secret_sends_nothing`:** `Misconfigured`, zero
  stub calls.

The stub replaces `globalThis.fetch`, as the browser suite's `FetchStub`
does.  Reuse or copy its pattern; keep it in `tests/worker/support`.

**Traceability:** FR-ABUSE-10 and FR-ABUSE-12 rows gain the worker tests; a
new line for the client-IP server test.

## Documentation in this handoff

- **API rustdoc** for the new types and method.
- **`docs/src/reference/api.md`:** `ChallengeRequest`, `ChallengeClientIp`,
  `verify_request`, the aliases from 01.
- **`docs/src/security/challenge.md`, privacy per vendor:** "the visitor's IP
  is sent when the site provides `ChallengeClientIp`".
- **CHANGELOG:**
  - *Added:* the client IP, and verification on Workers.
  - *Migration:* none; the method has a default.

## Acceptance

1. **Native gates, both suites, the examples** pass.  The new native tests
   pass.
2. **The Workers step, now with `challenge-http`,** passes: the four
   compile errors RFC 011 measured are gone.
3. **The worker tests** pass in headless Chrome, locally and in CI.
4. **Deliberate breaks:**
   - **A.**  On wasm32, set `RequestRedirect::Follow`.  The redirect test
     fails.
   - **B.**  Drop `remoteip` from the form body.  The native body test fails.
   - **C.**  Log the client IP in the challenge step.  The logging test
     fails.

   Restore each.
5. **Grep.**  `git grep -n 'remote_ip\|ChallengeClientIp' crates/leptos-hl-contact/src/server.rs`
   shows no `tracing::` line with it.

## Review request

`.git-exclude/review-request/011-cloudflare-workers/03-fetch-verifier-and-client-ip.md`
