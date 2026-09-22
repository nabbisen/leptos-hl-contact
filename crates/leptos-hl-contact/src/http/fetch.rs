// http/fetch.rs — the shared POST on a wasm32 server (Cloudflare Workers),
// through the global `fetch` (RFC 011 D3, RFC 017 D1).
//
// The same guarantees as the native path: redirects are never followed, a
// time limit applies, no error carries the URL, and the status is returned
// rather than judged here.

use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::Poll,
};

use js_sys::{Function, Promise, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, Headers, Request, RequestInit, RequestRedirect, Response, UrlSearchParams,
};

use super::{HttpBody, HttpError, HttpRequest, HttpResponse, MAX_RESPONSE_BODY};
use crate::wasm_timer;

/// Aborts its request when dropped before the exchange has finished.
///
/// The caller's future owns it.  So a call that is dropped mid-flight (the
/// submission was cancelled) or that runs out of time never leaves its
/// request running.  Awaiting the `fetch` promise through `JsFuture` frees
/// its callbacks, because a `fetch` promise always settles, an aborted one
/// included.
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

/// Post `request`, within its own time limit.
pub(super) async fn send(request: HttpRequest<'_>) -> Result<HttpResponse, HttpError> {
    let controller = AbortController::new().map_err(|_| unusable("no AbortController"))?;
    let js_request = build_request(&request, &controller)?;
    let mut guard = AbortOnDrop {
        controller,
        finished: false,
    };

    let mut exchange = pin!(exchange(js_request));
    let mut timer = wasm_timer::sleep(request.limit);
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
        // The timer won.  Returning drops `guard` unfinished, which aborts
        // the request: the abort is ours, so this is a timeout.
        None => Err(HttpError::Timeout),
    }
}

/// A `POST` that refuses redirects and listens to `controller`.
fn build_request(
    request: &HttpRequest<'_>,
    controller: &AbortController,
) -> Result<Request, HttpError> {
    let headers = Headers::new().map_err(|_| unusable("no Headers"))?;
    let body: JsValue = match &request.body {
        HttpBody::Form(pairs) => {
            let form = UrlSearchParams::new().map_err(|_| unusable("no URLSearchParams"))?;
            for (name, value) in *pairs {
                form.append(name, value);
            }
            headers
                .set("Content-Type", "application/x-www-form-urlencoded")
                .map_err(|_| unusable("no Headers"))?;
            form.into()
        }
        HttpBody::Json(json) => {
            headers
                .set("Content-Type", "application/json")
                .map_err(|_| unusable("no Headers"))?;
            JsValue::from_str(json)
        }
    };
    for (name, value) in request.headers {
        headers
            .set(name, value)
            .map_err(|_| unusable("no Headers"))?;
    }

    let init = RequestInit::new();
    init.set_method("POST");
    init.set_headers(&headers);
    init.set_body(&body);
    // Never resend a secret or a key to a `Location` of someone else's choosing.
    init.set_redirect(RequestRedirect::Manual);
    init.set_signal(Some(&controller.signal()));

    // The URL is not in the message: it could be a proxy address.
    Request::new_with_str_and_init(request.url, &init).map_err(|_| unusable("invalid URL"))
}

/// Send `request`, and return its status and body without judging either.
async fn exchange(request: Request) -> Result<HttpResponse, HttpError> {
    let global = js_sys::global();
    let fetch = Reflect::get(&global, &JsValue::from_str("fetch"))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .ok_or_else(|| unusable("no global fetch"))?;
    let promise = fetch
        .call1(&global, &request)
        .ok()
        .and_then(|value| value.dyn_into::<Promise>().ok())
        .ok_or_else(network_error)?;

    let response: Response = JsFuture::from(promise)
        .await
        .map_err(|_| network_error())?
        .dyn_into()
        .map_err(|_| unusable("not a Response"))?;

    let status = response.status();

    // When the answer states its length, an oversized one is refused before
    // `array_buffer` ever reads it (RFC 017 handoff 02 review, C2).  Without
    // the header — a chunked or unlabelled answer — the check below still
    // catches it, after the whole body has already been read; see
    // `MAX_RESPONSE_BODY`'s own doc comment for what that gap covers.
    if let Ok(Some(length)) = response.headers().get("content-length")
        && let Ok(length) = length.parse::<usize>()
        && length > MAX_RESPONSE_BODY
    {
        return Err(unusable("response too large"));
    }

    let buffer = response.array_buffer().map_err(|_| network_error())?;
    let buffer = JsFuture::from(buffer).await.map_err(|_| network_error())?;
    let array = Uint8Array::new(&buffer);
    if array.length() as usize > MAX_RESPONSE_BODY {
        return Err(unusable("response too large"));
    }
    Ok(HttpResponse {
        status,
        body: array.to_vec(),
    })
}

fn unusable(reason: &'static str) -> HttpError {
    HttpError::Unusable(reason)
}

/// A rejected `fetch` or body read.  JavaScript's message is not used: it may
/// name the URL.
fn network_error() -> HttpError {
    HttpError::Unusable("network error")
}
