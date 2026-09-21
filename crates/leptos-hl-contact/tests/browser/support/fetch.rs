//! `window.fetch`, answered by the test.

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// `window.fetch` replaced for the life of the value.  Every request's URL is
/// recorded and answered by the test; nothing leaves the page.
///
/// The client calls the global `fetch` by name at call time (gloo-net), so
/// replacing the property is enough.  The answer must be a real `Response`.
pub struct FetchStub {
    urls: Rc<RefCell<Vec<String>>>,
    bodies: Rc<RefCell<Vec<String>>>,
    original: JsValue,
    _closure: Closure<dyn FnMut(JsValue) -> js_sys::Promise>,
}

impl FetchStub {
    /// `respond` maps a request URL to a status and body.  Success bodies are
    /// JSON; errors are `Variant|message` with a 4xx or 5xx status.
    pub fn install(respond: impl Fn(&str) -> (u16, String) + 'static) -> Self {
        Self::install_answering(move |url| Some(respond(url)))
    }

    /// Requests ending with `path` never get an answer, so the submission
    /// stays in flight; anything else is an unexpected request.
    pub fn never_answering(path: &'static str) -> Self {
        Self::install_answering(move |url| {
            (!url.ends_with(path)).then(|| (500, UNEXPECTED.to_owned()))
        })
    }

    /// `answer` returns `None` for a request that must never be answered.
    fn install_answering(answer: impl Fn(&str) -> Option<(u16, String)> + 'static) -> Self {
        let window = web_sys::window().expect("window");
        let original = js_sys::Reflect::get(&window, &JsValue::from_str("fetch")).expect("fetch");
        let urls = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&urls);
        let bodies = Rc::new(RefCell::new(Vec::new()));
        let body_log = Rc::clone(&bodies);
        let closure =
            Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |request: JsValue| {
                let url = request
                    .dyn_ref::<web_sys::Request>()
                    .map(web_sys::Request::url)
                    .or_else(|| request.as_string())
                    .unwrap_or_default();
                log.borrow_mut().push(url.clone());
                // A body is readable only asynchronously; read a clone, so the
                // request itself is left as it was.  It is there once the page
                // has settled.
                if let Some(text) = request
                    .dyn_ref::<web_sys::Request>()
                    .and_then(|r| r.clone().ok())
                    .and_then(|r| r.text().ok())
                {
                    let body_log = Rc::clone(&body_log);
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(body) = wasm_bindgen_futures::JsFuture::from(text).await {
                            body_log
                                .borrow_mut()
                                .push(body.as_string().unwrap_or_default());
                        }
                    });
                }
                let Some((status, body)) = answer(&url) else {
                    return js_sys::Promise::new(&mut |_resolve, _reject| {});
                };
                let init = web_sys::ResponseInit::new();
                init.set_status(status);
                let response = web_sys::Response::new_with_opt_str_and_init(Some(&body), &init)
                    .expect("a Response");
                js_sys::Promise::resolve(&JsValue::from(response))
            });
        js_sys::Reflect::set(&window, &JsValue::from_str("fetch"), closure.as_ref())
            .expect("replace fetch");
        Self {
            urls,
            bodies,
            original,
            _closure: closure,
        }
    }

    /// Answers the token endpoint with `token`, and anything else with a
    /// server error, so an unexpected request cannot pass for a success.
    pub fn issuing(token: &'static str) -> Self {
        Self::install(move |url| {
            if url.ends_with("/api/form_token") {
                (200, format!("\"{token}\""))
            } else {
                (500, UNEXPECTED.to_owned())
            }
        })
    }

    /// Requests whose URL ends with `path`.
    pub fn count(&self, path: &str) -> usize {
        self.urls
            .borrow()
            .iter()
            .filter(|url| url.ends_with(path))
            .count()
    }

    pub fn urls(&self) -> Vec<String> {
        self.urls.borrow().clone()
    }

    /// The bodies of the requests made so far, once the page has settled, with
    /// the brackets of a `fields[key]` name un-escaped so a test can read it.
    pub fn bodies(&self) -> Vec<String> {
        self.bodies
            .borrow()
            .iter()
            .map(|body| body.replace("%5B", "[").replace("%5D", "]"))
            .collect()
    }
}

/// The body answered to a request a test did not expect.
pub const UNEXPECTED: &str = "ServerError|unexpected request in test";

impl Drop for FetchStub {
    fn drop(&mut self) {
        let window = web_sys::window().expect("window");
        let _ = js_sys::Reflect::set(&window, &JsValue::from_str("fetch"), &self.original);
    }
}
