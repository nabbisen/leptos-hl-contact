//! A challenge vendor's global (`window.turnstile` and the like).

use std::{cell::RefCell, rc::Rc};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// `window[name]` as a plain object whose `render` records the element and
/// the params object it is given.  No vendor script is loaded.
pub struct VendorStub {
    name: &'static str,
    rendered: Rc<RefCell<Vec<JsValue>>>,
    params: Rc<RefCell<Vec<JsValue>>>,
    _render: Closure<dyn FnMut(JsValue, JsValue) -> JsValue>,
    _remove: Closure<dyn FnMut(JsValue)>,
}

impl VendorStub {
    pub fn install(name: &'static str) -> Self {
        let rendered: Rc<RefCell<Vec<JsValue>>> = Rc::default();
        let params: Rc<RefCell<Vec<JsValue>>> = Rc::default();
        let log = Rc::clone(&rendered);
        let log_params = Rc::clone(&params);
        let render = Closure::<dyn FnMut(JsValue, JsValue) -> JsValue>::new(
            move |element: JsValue, params: JsValue| {
                log.borrow_mut().push(element);
                log_params.borrow_mut().push(params);
                JsValue::from_str("test-widget-id")
            },
        );
        let remove = Closure::<dyn FnMut(JsValue)>::new(|_id: JsValue| {});

        let global = js_sys::Object::new();
        js_sys::Reflect::set(&global, &JsValue::from_str("render"), render.as_ref())
            .expect("render");
        js_sys::Reflect::set(&global, &JsValue::from_str("remove"), remove.as_ref())
            .expect("remove");
        let window = web_sys::window().expect("window");
        js_sys::Reflect::set(&window, &JsValue::from_str(name), &global).expect("global");

        Self {
            name,
            rendered,
            params,
            _render: render,
            _remove: remove,
        }
    }

    /// The elements `render` was called with, in order.
    pub fn rendered(&self) -> Vec<JsValue> {
        self.rendered.borrow().clone()
    }

    /// The params objects `render` was called with, in the same order as
    /// [`rendered`](Self::rendered).
    pub fn params(&self) -> Vec<JsValue> {
        self.params.borrow().clone()
    }
}

impl Drop for VendorStub {
    fn drop(&mut self) {
        let window = web_sys::window().expect("window");
        let _ = js_sys::Reflect::delete_property(
            window.unchecked_ref::<js_sys::Object>(),
            &JsValue::from_str(self.name),
        );
    }
}
