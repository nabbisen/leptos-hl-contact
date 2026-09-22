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

/// The body of an outgoing request.
pub(crate) enum HttpBody<'a> {
    /// `application/x-www-form-urlencoded`.
    Form(&'a [(&'static str, String)]),
    /// `application/json`.
    #[allow(
        dead_code,
        reason = "constructed by delivery-resend, RFC 017 handoff 02"
    )]
    Json(&'a str),
}

/// One POST, described completely so the transport has nothing left to
/// decide.
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
pub(crate) enum HttpError {
    /// The limit passed.  The request was aborted where the target allows it.
    Timeout,
    /// A transport failure.  **Never carries a URL.**
    ///
    /// Constructed only natively (`reqwest`); a wasm32 server reports every
    /// such case through `Unusable` instead, as it always has.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    Transport(String),
    /// The environment lacks something the request needs, or the answer was
    /// not usable at all: a fixed, caller-independent reason.
    ///
    /// Constructed only on a wasm32 server (`http/fetch.rs`); the native path
    /// (`reqwest`) reports every such case through `Transport` instead.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
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
        let response = builder.send().await.map_err(transport_error)?;
        let status = response.status().as_u16();
        let body = response.bytes().await.map_err(transport_error)?.to_vec();
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

    pub(crate) async fn post(&self, request: HttpRequest<'_>) -> Result<HttpResponse, HttpError> {
        fetch::send(request).await
    }
}

// The request over the global `fetch`, on a wasm32 server (RFC 011 D3).
#[cfg(target_arch = "wasm32")]
mod fetch;
