// http.rs — a private HTTP client shared by every built-in feature that
// posts to an external endpoint (today `challenge-http`; RFC 017 D1 adds a
// delivery adapter later).  One POST, on both targets, with the properties
// every caller needs and none of them may weaken:
//
// - Redirects are refused, natively and on a wasm32 server, so a caller's
//   secret or key is never resent to a `Location` of someone else's
//   choosing.
// - The caller's own time limit applies, and a wasm32 request is aborted
//   when it is dropped before finishing (a cancelled submission, or the
//   limit passing).
// - No error carries the URL: it could be a proxy address.
// - The status is returned, not judged.  The caller decides what a non-2xx
//   answer means; this module never does.
//
// Moved from `challenge/http.rs` and `challenge/http/fetch.rs` (RFC 011 D3),
// which called `HttpChallengeVerifier`'s vendor-specific pieces directly.
// Nothing here knows about a challenge or a vendor.

use std::time::Duration;

/// The largest response body either target reads.
///
/// Every answer this crate reads is a small JSON document.  A body over this
/// is `HttpError::Unusable` instead of being parsed, so a misconfigured proxy
/// or a hostile endpoint cannot make the server hold an unbounded amount of
/// memory (RFC 017 handoff 01 review).
///
/// **What is actually bounded, per target (handoff 02 review, C2):**
/// - **Native:** the cap bounds the read itself.  `HttpClient::post` reads
///   `reqwest::Response::chunk()`s and stops as soon as the running total
///   would exceed the cap, so a hostile endpoint cannot make this allocate
///   more than one chunk past `MAX_RESPONSE_BODY`.
/// - **wasm32:** `fetch`'s `Response` exposes no chunked reader as simple as
///   `chunk()`.  When the answer carries `Content-Length`, that is checked
///   **before** `array_buffer()` is called, so an oversized answer that says
///   so is never read at all.  Without that header (a chunked or unlabelled
///   answer), the whole body is read first and the length is checked after —
///   the cap then bounds what a *caller* receives, not what the runtime
///   allocated to produce it.  Every endpoint this crate talks to in
///   production sends `Content-Length`; the gap is for an overridden
///   endpoint (`with_verify_url`, `with_url`) that does not.
const MAX_RESPONSE_BODY: usize = 64 * 1024;

/// The body of an outgoing request.
#[cfg_attr(
    not(any(feature = "challenge-http", feature = "delivery-resend")),
    allow(dead_code)
)]
pub(crate) enum HttpBody<'a> {
    /// `application/x-www-form-urlencoded`.
    ///
    /// Constructed only by the challenge verifiers (`challenge-http`); a
    /// `delivery-resend`-only build never builds a form body.
    #[cfg_attr(not(feature = "challenge-http"), allow(dead_code))]
    Form(&'a [(&'static str, String)]),
    /// `application/json`.
    ///
    /// Constructed only by the Resend adapter (`delivery-resend`); a
    /// `challenge-http`-only build never builds a JSON body.
    #[cfg_attr(not(feature = "delivery-resend"), allow(dead_code))]
    Json(&'a str),
}

/// One POST, described completely so the transport has nothing left to
/// decide.
///
/// Built only by `challenge-http` and `delivery-resend`; an
/// `email-domain-check`-only build uses [`HttpClient::get`] instead and
/// never constructs one.
#[cfg_attr(
    not(any(feature = "challenge-http", feature = "delivery-resend")),
    allow(dead_code)
)]
pub(crate) struct HttpRequest<'a> {
    pub url: &'a str,
    /// Headers beyond the content type, which `body` already implies.
    pub headers: &'a [(&'a str, String)],
    pub body: HttpBody<'a>,
    pub limit: Duration,
}

/// A completed exchange: a status and a body, neither judged here.
pub(crate) struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}

/// What can go wrong before a status is known.
#[derive(Debug)]
pub(crate) enum HttpError {
    /// The limit passed.  The request was aborted where the target allows it.
    Timeout,
    /// A transport failure.  **Never carries a URL.**
    ///
    /// Constructed only natively (`reqwest`); a wasm32 server reports every
    /// such case through `Unusable` instead, as it always has.  Its text is
    /// read only by `challenge-http` and `delivery-resend`; an
    /// `email-domain-check`-only build discards it (`query`'s fixed
    /// `"transport"` reason never names the detail, D5).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    #[cfg_attr(
        not(any(feature = "challenge-http", feature = "delivery-resend")),
        allow(dead_code)
    )]
    Transport(String),
    /// The environment lacks something the request needs, or the answer was
    /// not usable at all: a fixed, caller-independent reason.  Its text is
    /// read only by `challenge-http` and `delivery-resend`, the same as
    /// `Transport`'s above.
    #[cfg_attr(
        not(any(feature = "challenge-http", feature = "delivery-resend")),
        allow(dead_code)
    )]
    Unusable(&'static str),
}

/// Natively a reused `reqwest` client with redirects refused; on wasm32 a
/// unit type that posts through the global `fetch`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) struct HttpClient(reqwest::Client);

#[cfg(not(target_arch = "wasm32"))]
impl HttpClient {
    pub(crate) fn new() -> Self {
        Self(
            reqwest::Client::builder()
                // Never resend a secret or a key to a `Location` of someone
                // else's choosing.
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("the rustls TLS backend initialises"),
        )
    }

    #[cfg_attr(
        not(any(feature = "challenge-http", feature = "delivery-resend")),
        allow(dead_code)
    )]
    pub(crate) async fn post(&self, request: HttpRequest<'_>) -> Result<HttpResponse, HttpError> {
        let mut builder = self.0.post(request.url).timeout(request.limit);
        for (name, value) in request.headers {
            builder = builder.header(*name, value.clone());
        }
        builder = match request.body {
            HttpBody::Form(pairs) => builder.form(pairs),
            HttpBody::Json(json) => builder
                .header("content-type", "application/json")
                .body(json.to_owned()),
        };
        let mut response = builder.send().await.map_err(transport_error)?;
        let status = response.status().as_u16();

        // Read in chunks and stop as soon as the cap would be exceeded, so a
        // hostile or misconfigured endpoint cannot make this allocate more
        // than `MAX_RESPONSE_BODY` (RFC 017 handoff 02 review, C2).
        // `Response::chunk` needs no extra reqwest feature.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len() + chunk.len() > MAX_RESPONSE_BODY {
                return Err(HttpError::Unusable("response too large"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse { status, body })
    }

    /// A GET with no body (RFC 018 Amendment A1): the same redirect refusal,
    /// timeout and capped chunked read as [`Self::post`], for a caller that
    /// has nothing to send.
    #[cfg_attr(not(feature = "email-domain-check"), allow(dead_code))]
    pub(crate) async fn get(
        &self,
        url: &str,
        headers: &[(&str, String)],
        limit: Duration,
    ) -> Result<HttpResponse, HttpError> {
        let mut builder = self.0.get(url).timeout(limit);
        for (name, value) in headers {
            builder = builder.header(*name, value.clone());
        }
        let mut response = builder.send().await.map_err(transport_error)?;
        let status = response.status().as_u16();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if body.len() + chunk.len() > MAX_RESPONSE_BODY {
                return Err(HttpError::Unusable("response too large"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(HttpResponse { status, body })
    }
}

/// A timeout is `Timeout`; anything else on the wire is `Transport`.  The URL
/// is stripped: it could be a proxy address.
#[cfg(not(target_arch = "wasm32"))]
fn transport_error(error: reqwest::Error) -> HttpError {
    if error.is_timeout() {
        HttpError::Timeout
    } else {
        HttpError::Transport(error.without_url().to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct HttpClient;

#[cfg(target_arch = "wasm32")]
impl HttpClient {
    pub(crate) fn new() -> Self {
        Self
    }

    #[cfg_attr(
        not(any(feature = "challenge-http", feature = "delivery-resend")),
        allow(dead_code)
    )]
    pub(crate) async fn post(&self, request: HttpRequest<'_>) -> Result<HttpResponse, HttpError> {
        fetch::send(request).await
    }

    /// A GET with no body (RFC 018 Amendment A1).
    #[cfg_attr(not(feature = "email-domain-check"), allow(dead_code))]
    pub(crate) async fn get(
        &self,
        url: &str,
        headers: &[(&str, String)],
        limit: Duration,
    ) -> Result<HttpResponse, HttpError> {
        fetch::get(url, headers, limit).await
    }
}

// The request over the global `fetch`, on a wasm32 server (RFC 011 D3).
#[cfg(target_arch = "wasm32")]
mod fetch;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
