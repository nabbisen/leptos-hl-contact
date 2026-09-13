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
    original: JsValue,
    _closure: Closure<dyn FnMut(JsValue) -> js_sys::Promise>,
}

impl FetchStub {
    /// `respond` maps a request URL to a status and body.  Success bodies are
    /// JSON; errors are `Variant|message` with a 4xx or 5xx status.
    pub fn install(respond: impl Fn(&str) -> (u16, String) + 'static) -> Self {
        let window = web_sys::window().expect("window");
        let original = js_sys::Reflect::get(&window, &JsValue::from_str("fetch")).expect("fetch");
        let urls = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&urls);
        let closure =
            Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |request: JsValue| {
                let url = request
                    .dyn_ref::<web_sys::Request>()
                    .map(web_sys::Request::url)
                    .or_else(|| request.as_string())
                    .unwrap_or_default();
                log.borrow_mut().push(url.clone());
                let (status, body) = respond(&url);
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
}

/// The body answered to a request a test did not expect.
pub const UNEXPECTED: &str = "ServerError|unexpected request in test";

impl Drop for FetchStub {
    fn drop(&mut self) {
        let window = web_sys::window().expect("window");
        let _ = js_sys::Reflect::set(&window, &JsValue::from_str("fetch"), &self.original);
    }
}
