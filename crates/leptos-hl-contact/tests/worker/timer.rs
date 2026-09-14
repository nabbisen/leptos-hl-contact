//! `DeliveryTimeout` with the JavaScript timer on a wasm32 server (RFC 011 D7).

use std::{future::pending, time::Duration};

use leptos_hl_contact::{
    ContactDelivery, ContactDeliveryError, ContactInput, DeliveryFuture, DeliveryTimeout,
};
use wasm_bindgen_test::wasm_bindgen_test;

struct Never;

impl ContactDelivery for Never {
    fn deliver(&self, _input: ContactInput) -> DeliveryFuture<'_> {
        Box::pin(pending())
    }
}

/// Answers at once with the result its function returns.
struct Answers(fn() -> Result<(), ContactDeliveryError>);

impl ContactDelivery for Answers {
    fn deliver(&self, _input: ContactInput) -> DeliveryFuture<'_> {
        let result = (self.0)();
        Box::pin(async move { result })
    }
}

fn input() -> ContactInput {
    ContactInput::from_raw(
        "Ada".into(),
        "ada@example.com".into(),
        None,
        "Hello".into(),
        String::new(),
    )
}

/// FR-DEL-08, NFR-PERF-03, NFR-PORT-02 (RFC 011 D7): a delivery that never
/// finishes ends at the limit with `Timeout(limit)`.  This waits 50 ms for
/// real: a JavaScript clock cannot be paused here.
#[wasm_bindgen_test]
async fn a_delivery_that_never_finishes_times_out() {
    let limit = Duration::from_millis(50);
    let started = js_sys::Date::now();
    let result = DeliveryTimeout::new(Never, limit).deliver(input()).await;
    let elapsed_ms = js_sys::Date::now() - started;

    assert!(
        matches!(result, Err(ContactDeliveryError::Timeout(l)) if l == limit),
        "{result:?}"
    );
    assert!(elapsed_ms >= 50.0, "returned after {elapsed_ms} ms");
}

/// FR-DEL-08, NFR-PORT-02 (RFC 011 D7): a delivery that finishes in time
/// returns its own result, success or error, unchanged.
#[wasm_bindgen_test]
async fn a_delivery_that_finishes_in_time_returns_its_result() {
    let limit = Duration::from_secs(5);
    let ok = DeliveryTimeout::new(Answers(|| Ok(())), limit)
        .deliver(input())
        .await;
    assert!(ok.is_ok(), "{ok:?}");

    let refused = DeliveryTimeout::new(
        Answers(|| Err(ContactDeliveryError::Transport("relay said no".into()))),
        limit,
    )
    .deliver(input())
    .await;
    assert!(
        matches!(&refused, Err(ContactDeliveryError::Transport(m)) if m == "relay said no"),
        "{refused:?}"
    );
}
