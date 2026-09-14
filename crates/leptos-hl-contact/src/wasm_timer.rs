// wasm_timer.rs — a timer for a wasm32 server (Cloudflare Workers).
//
// A Worker has no tokio runtime, and its global scope has no `window`, so the
// timer is the global `setTimeout`, looked up on `js_sys::global()`
// (RFC 011 D7).  Compiled on a wasm32 server only.

use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

use js_sys::{Function, Reflect};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};

/// What the timer callback and the future share.
#[derive(Default)]
struct Shared {
    fired: bool,
    waker: Option<Waker>,
}

/// A future that resolves once its duration has passed.
///
/// `Sleep` owns the callback it gave `setTimeout`.  Dropping it before the
/// timer fires clears the timer first, then frees the callback, so no timer
/// and no callback outlives the future.
pub(crate) struct Sleep {
    shared: Rc<RefCell<Shared>>,
    /// `setTimeout`'s handle, for `clearTimeout`.
    id: JsValue,
    /// Dropped after `Drop::drop` has cleared the timer: JavaScript never
    /// calls a freed callback.
    _callback: Closure<dyn FnMut()>,
}

/// The longest delay `setTimeout` honours.  A longer one overflows and fires at
/// once, so it is clamped (about 24.8 days).
const MAX_DELAY_MS: u128 = i32::MAX as u128;

/// A future that resolves after `duration`.
pub(crate) fn sleep(duration: Duration) -> Sleep {
    let millis = duration.as_millis().min(MAX_DELAY_MS) as f64;
    let shared = Rc::new(RefCell::new(Shared::default()));
    let callback = Closure::<dyn FnMut()>::new({
        let shared = Rc::clone(&shared);
        move || {
            let waker = {
                let mut shared = shared.borrow_mut();
                shared.fired = true;
                shared.waker.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }
    });
    let global = js_sys::global();
    let id = global_function(&global, "setTimeout")
        .call2(&global, callback.as_ref(), &JsValue::from_f64(millis))
        .expect("setTimeout accepts a function and a delay");
    Sleep {
        shared,
        id,
        _callback: callback,
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut shared = self.shared.borrow_mut();
        if shared.fired {
            Poll::Ready(())
        } else {
            shared.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if !self.shared.borrow().fired {
            let global = js_sys::global();
            let _ = global_function(&global, "clearTimeout").call1(&global, &self.id);
        }
    }
}

/// A function on the global scope: `window` in a browser, the Worker's global
/// scope on Cloudflare Workers.
fn global_function(global: &JsValue, name: &str) -> Function {
    Reflect::get(global, &JsValue::from_str(name))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .unwrap_or_else(|| panic!("`{name}` is not a function on the global scope"))
}
