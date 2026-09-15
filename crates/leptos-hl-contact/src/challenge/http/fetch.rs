// fetch.rs — `HttpChallengeVerifier`'s request on a wasm32 server (Cloudflare
// Workers), through the global `fetch` (RFC 011 D3).
//
// The same guarantees as the native path: redirects are never followed, a
// time limit applies, and no error message carries the URL.

use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::Poll,
    time::Duration,
};

use js_sys::{Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, Headers, Request, RequestInit, RequestRedirect, Response, UrlSearchParams,
};

use super::parse_response;
use crate::{
    challenge::{ChallengeError, ChallengeOutcome},
    wasm_timer,
};

/// Aborts its request when dropped before the exchange has finished.
///
/// The verification future owns it.  So a verification that is dropped
/// mid-flight (the submission was cancelled) or that runs out of time never
/// leaves its request running.  Awaiting the `fetch` promise through
/// `JsFuture` frees its callbacks, because a `fetch` promise always settles,
/// an aborted one included.
struct AbortOnDrop {
    controller: AbortController,
    finished: bool,
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if !self.finished {
            self.controller.abort();
        }
    }
}

/// Post `body` to `url` and parse the vendor's answer, within `limit`.
pub(super) async fn send(
    url: &str,
    body: &[(&'static str, String)],
    limit: Duration,
) -> Result<ChallengeOutcome, ChallengeError> {
    let controller = AbortController::new().map_err(|_| unavailable("no AbortController"))?;
    let request = build_request(url, body, &controller)?;
    let mut guard = AbortOnDrop {
        controller,
        finished: false,
    };

    let mut exchange = pin!(exchange(request));
    let mut timer = wasm_timer::sleep(limit);
    let finished = poll_fn(|cx| {
        if let Poll::Ready(result) = exchange.as_mut().poll(cx) {
            return Poll::Ready(Some(result));
        }
        match Pin::new(&mut timer).poll(cx) {
            Poll::Ready(()) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    })
    .await;

    match finished {
        Some(result) => {
            guard.finished = true;
            result
        }
        // The timer won.  Returning drops `guard` unfinished, which aborts the
        // request: the abort is ours, so this is a timeout.
        None => Err(ChallengeError::Timeout),
    }
}

/// A form `POST` that refuses redirects and listens to `controller`.
fn build_request(
    url: &str,
    body: &[(&'static str, String)],
    controller: &AbortController,
) -> Result<Request, ChallengeError> {
    let form = UrlSearchParams::new().map_err(|_| unavailable("no URLSearchParams"))?;
    for (name, value) in body {
        form.append(name, value);
    }
    let headers = Headers::new().map_err(|_| unavailable("no Headers"))?;
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|_| unavailable("no Headers"))?;

    let init = RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&form);
    // Never resend the secret to a `Location` of someone else's choosing.
    init.set_redirect(RequestRedirect::Manual);
    init.set_signal(Some(&controller.signal()));

    // The URL is not in the message: it could be a proxy address.
    Request::new_with_str_and_init(url, &init).map_err(|_| unavailable("invalid verify URL"))
}

/// Send `request`, check the status and parse the body.
async fn exchange(request: Request) -> Result<ChallengeOutcome, ChallengeError> {
    let global = js_sys::global();
    let fetch = Reflect::get(&global, &JsValue::from_str("fetch"))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .ok_or_else(|| unavailable("no global fetch"))?;
    let promise = fetch
        .call1(&global, &request)
        .ok()
        .and_then(|value| value.dyn_into::<Promise>().ok())
        .ok_or_else(network_error)?;

    let response: Response = JsFuture::from(promise)
        .await
        .map_err(|_| network_error())?
        .dyn_into()
        .map_err(|_| unavailable("not a Response"))?;

    // A 3xx from workerd, and a browser's opaque redirect (status 0), are
    // outside the range too.
    let status = response.status();
    if !(200..=299).contains(&status) {
        return Err(ChallengeError::Unavailable(format!("HTTP {status}")));
    }

    let buffer = response.array_buffer().map_err(|_| network_error())?;
    let buffer = JsFuture::from(buffer).await.map_err(|_| network_error())?;
    parse_response(&Uint8Array::new(&buffer).to_vec())
}

fn unavailable(reason: &str) -> ChallengeError {
    ChallengeError::Unavailable(reason.to_owned())
}

/// A rejected `fetch` or body read.  JavaScript's message is not used: it may
/// name the URL.
fn network_error() -> ChallengeError {
    unavailable("network error")
}
