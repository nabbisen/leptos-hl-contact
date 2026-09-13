//! `setTimeout`, `clearTimeout` and `Date.now`, under the test's control.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use wasm_bindgen::{JsCast, JsValue, closure::Closure};

struct Timer {
    id: i32,
    delay_ms: i32,
    /// `None` once fired or cleared.
    callback: Option<js_sys::Function>,
}

/// A clock the test moves.  A scheduled callback runs only when the test
/// fires it, so an hour passes in no time.
///
/// Leptos schedules through `window.setTimeout` and reads `Date.now`, both
/// looked up at call time, so replacing the properties is enough.
pub struct Clock {
    timers: Rc<RefCell<Vec<Timer>>>,
    now_ms: Rc<Cell<f64>>,
    originals: Vec<(JsValue, &'static str, JsValue)>,
    _set_timeout: Closure<dyn FnMut(JsValue, JsValue) -> i32>,
    _clear_timeout: Closure<dyn FnMut(JsValue)>,
    _now: Closure<dyn FnMut() -> f64>,
}

impl Clock {
    /// Installs the clock, reading `now_secs` (Unix seconds).
    pub fn install(now_secs: u64) -> Self {
        let window: JsValue = web_sys::window().expect("window").into();
        let date =
            js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Date")).expect("Date");
        let timers: Rc<RefCell<Vec<Timer>>> = Rc::default();
        let now_ms = Rc::new(Cell::new(now_secs as f64 * 1000.0));

        let set_timeout = {
            let timers = Rc::clone(&timers);
            Closure::<dyn FnMut(JsValue, JsValue) -> i32>::new(
                move |callback: JsValue, delay: JsValue| {
                    let mut timers = timers.borrow_mut();
                    let id = timers.len() as i32 + 1;
                    timers.push(Timer {
                        id,
                        delay_ms: delay.as_f64().unwrap_or(0.0) as i32,
                        callback: Some(callback.unchecked_into()),
                    });
                    id
                },
            )
        };
        let clear_timeout = {
            let timers = Rc::clone(&timers);
            Closure::<dyn FnMut(JsValue)>::new(move |id: JsValue| {
                let id = id.as_f64().unwrap_or(-1.0) as i32;
                if let Some(timer) = timers.borrow_mut().iter_mut().find(|t| t.id == id) {
                    timer.callback = None;
                }
            })
        };
        let now = {
            let now_ms = Rc::clone(&now_ms);
            Closure::<dyn FnMut() -> f64>::new(move || now_ms.get())
        };

        let mut originals = Vec::new();
        for (target, name, replacement) in [
            (&window, "setTimeout", set_timeout.as_ref()),
            (&window, "clearTimeout", clear_timeout.as_ref()),
            (&date, "now", now.as_ref()),
        ] {
            let key = JsValue::from_str(name);
            let original = js_sys::Reflect::get(target, &key).expect(name);
            js_sys::Reflect::set(target, &key, replacement).expect(name);
            originals.push((target.clone(), name, original));
        }

        Self {
            timers,
            now_ms,
            originals,
            _set_timeout: set_timeout,
            _clear_timeout: clear_timeout,
            _now: now,
        }
    }

    pub fn set_now(&self, now_secs: u64) {
        self.now_ms.set(now_secs as f64 * 1000.0);
    }

    /// Delays, in milliseconds, of timers scheduled and neither fired nor
    /// cleared, in scheduling order.
    pub fn pending(&self) -> Vec<i32> {
        self.timers
            .borrow()
            .iter()
            .filter(|t| t.callback.is_some())
            .map(|t| t.delay_ms)
            .collect()
    }

    /// Runs the first pending timer scheduled with `delay_ms`, as if that
    /// much time had passed.  Returns whether one was pending.
    pub fn fire(&self, delay_ms: i32) -> bool {
        // The borrow ends before the callback runs: it may schedule again.
        let callback = self
            .timers
            .borrow_mut()
            .iter_mut()
            .find(|t| t.callback.is_some() && t.delay_ms == delay_ms)
            .and_then(|t| t.callback.take());
        match callback {
            Some(callback) => {
                callback.call0(&JsValue::NULL).expect("timer callback");
                true
            }
            None => false,
        }
    }
}

impl Drop for Clock {
    fn drop(&mut self) {
        for (target, name, original) in &self.originals {
            let _ = js_sys::Reflect::set(target, &JsValue::from_str(name), original);
        }
    }
}
