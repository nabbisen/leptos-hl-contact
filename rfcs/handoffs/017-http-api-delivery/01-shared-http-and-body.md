# Handoff 017-01 — A private HTTP module shared with `challenge-http`, and the body builder moved

**RFC.** [RFC 017](../../accepted/017-http-api-delivery.md) D1
**Roadmap.** P-22
**Requirements.** unchanged by this handoff; NFR-PORT-02 and FR-ABUSE-12 must keep their evidence

## Goal

**A refactor, with no behaviour change.**
- One private module posts a request on both targets, so the adapter in 02
  does not need a second copy of the wasm32 `fetch` path.
- The plain-text body builder moves where both backends can call it.

**If anything here would change behaviour, stop and report.**

## Change scope

### 1. `src/http/` — the shared transport (new, private)

Suggested shape; a clearer one is welcome if it keeps the same properties:

```rust,ignore
pub(crate) enum HttpBody<'a> {
    /// `application/x-www-form-urlencoded`
    Form(&'a [(&'static str, String)]),
    /// `application/json`
    Json(&'a str),
}

pub(crate) struct HttpRequest<'a> {
    pub url: &'a str,
    pub headers: &'a [(&'a str, String)],   // beyond the content type
    pub body: HttpBody<'a>,
    pub limit: Duration,
}

pub(crate) struct HttpResponse { pub status: u16, pub body: Vec<u8> }

pub(crate) enum HttpError {
    /// The limit passed.  The request was aborted where the target allows it.
    Timeout,
    /// A transport failure.  **Never carries a URL.**
    Transport(String),
    /// The environment lacks something the request needs, or the answer was
    /// not usable: a fixed, caller-independent reason.
    Unusable(&'static str),
}

/// Natively a reused `reqwest` client with redirects refused; on wasm32 a
/// unit type.
pub(crate) struct HttpClient(…);

impl HttpClient {
    pub(crate) fn new() -> Self;
    pub(crate) async fn post(&self, request: HttpRequest<'_>) -> Result<HttpResponse, HttpError>;
}
```

**What must be preserved exactly, and is the reason this module exists:**
- **Redirects are refused:** `Policy::none()` natively, `RequestRedirect::Manual`
  on wasm32.
- **The time limit** is the request's own: `reqwest`'s `timeout` natively; on
  wasm32 the `poll_fn` race with `wasm_timer::sleep`.
- **Abort on drop** on wasm32: the `AbortOnDrop` guard moves here unchanged,
  including the comment that says why the abort is ours.
- **No URL in any error.**  `without_url()` natively; the fixed strings on
  wasm32.
- **The status is returned, not judged.**  The caller decides what a non-2xx
  means.

**Where the code comes from:** `challenge/http.rs`'s two `send` bodies and
all of `challenge/http/fetch.rs` except `parse_response`, which is the
challenge's own.

### 2. `src/challenge/http.rs` — call the module

- **`HttpChallengeVerifier`** holds an `HttpClient` instead of a
  `reqwest::Client`, and builds an `HttpRequest` with `HttpBody::Form`.
- **The mapping from `HttpError` to `ChallengeError` stays in the challenge**,
  and must produce **the same values as today**:
  - `Timeout` → `ChallengeError::Timeout`;
  - `Transport(text)` → `Unavailable(text)`, the same text `without_url()`
    gives now;
  - `Unusable(reason)` → `Unavailable(reason)`, the same fixed strings
    (`"no AbortController"`, `"no global fetch"`, `"invalid verify URL"`,
    `"not a Response"`, `"network error"`, `"no Headers"`,
    `"no URLSearchParams"`);
  - a non-2xx status → `Unavailable("HTTP {status}")`, as now.
- **`parse_response` and the empty-secret check do not move.**
- **`challenge/http/fetch.rs` is deleted** once its content lives in
  `src/http/`.

### 3. `src/delivery/body.rs` — the body builder moved (new, private)

- **Move `build_plain_text_body`** from `delivery/smtp.rs`, unchanged, with
  its rustdoc and its tests.
- **`smtp.rs` calls it.**  No output changes: the existing byte-for-byte body
  tests are the proof.

### 4. Features

- **No feature changes.**  `src/http/` compiles only when something needs it:
  gate it on the features that use it (`challenge-http` today), so a build
  without them is unchanged.
- **Report** `cargo tree` for a `ssr,smtp-lettre` build before and after:
  identical.

## Tests

- **No test is added, removed or edited** — that is the point.  If a test
  must change, stop and report why.
- **Run and report:** the unit suite, the server suite, the worker suite and
  the browser suite, with counts equal to 0.8.0's (292 / 44 / 13; worker 11,
  browser 19).
- **The live challenge tests** (`#[ignore]`d) are not required.  Say whether
  you ran them.

**Break check, required.**  Remove the redirect refusal from the shared
module (both targets, one at a time).
- **Expected:** the challenge's redirect tests fail —
  `challenge::http::a_redirect_is_not_followed_and_is_unavailable` natively,
  and `worker::fetch_verifier::a_redirect_is_unavailable` on wasm32.
- **Report** both failures, then restore.

## Gates

- **The shared gates,** on 1.98.x and 1.91.
- **The MSRV checks** on 1.88.
- **Both wasm suites** and the examples with `--locked`.
- **Feature combinations:** `--no-default-features --features ssr`;
  `ssr,smtp-lettre`; `ssr,challenge-http`; and the Workers set on wasm32.
- **CI** on the pushed commit.

## Review request

`.git-exclude/review-request/017-http-api-delivery/01-shared-http-and-body.md`:
- the commit;
- the module's final shape;
- a table of every error value before and after, showing they match;
- the `cargo tree` comparison;
- the break check;
- the gates and CI.
