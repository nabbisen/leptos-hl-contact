//! `globalThis.fetch`, answered by the test: a vendor's siteverify endpoint
//! without a network.

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// `globalThis.fetch` replaced for the life of the value.  Every `Request` is
/// recorded; nothing leaves the page.
///
/// The global scope is taken from `js_sys::global()`, as the verifier takes
/// it, so the stub is the `fetch` the verifier calls.
pub struct FetchStub {
    requests: Rc<RefCell<Vec<web_sys::Request>>>,
    original: JsValue,
    _closure: Closure<dyn FnMut(JsValue) -> js_sys::Promise>,
}

impl FetchStub {
    /// Every request is answered with `status` and `body`.
    pub fn answering(status: u16, body: &str) -> Self {
        Self::install(Some((status, body.to_owned())))
    }

    /// No request is ever answered.  The stub ignores the request's signal, so
    /// its promise never settles, unlike a real `fetch`.
    pub fn silent() -> Self {
        Self::install(None)
    }

    fn install(answer: Option<(u16, String)>) -> Self {
        let global = js_sys::global();
        let original = js_sys::Reflect::get(&global, &JsValue::from_str("fetch")).expect("fetch");
        let requests = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&requests);
        let closure =
            Closure::<dyn FnMut(JsValue) -> js_sys::Promise>::new(move |request: JsValue| {
                let request: web_sys::Request = request.dyn_into().expect("a Request");
                log.borrow_mut().push(request);
                let Some((status, body)) = &answer else {
                    return js_sys::Promise::new(&mut |_resolve, _reject| {});
                };
                let init = web_sys::ResponseInit::new();
                init.set_status(*status);
                let response = web_sys::Response::new_with_opt_str_and_init(Some(body), &init)
                    .expect("a Response");
                js_sys::Promise::resolve(&JsValue::from(response))
            });
        js_sys::Reflect::set(&global, &JsValue::from_str("fetch"), closure.as_ref())
            .expect("replace fetch");
        Self {
            requests,
            original,
            _closure: closure,
        }
    }

    /// The requests the verifier sent, in order.
    pub fn requests(&self) -> Vec<web_sys::Request> {
        self.requests.borrow().clone()
    }
}

impl Drop for FetchStub {
    fn drop(&mut self) {
        let _ = js_sys::Reflect::set(
            &js_sys::global(),
            &JsValue::from_str("fetch"),
            &self.original,
        );
    }
}
