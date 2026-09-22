//! `ResendDelivery` over `fetch` on a wasm32 server (RFC 017 D3, D5), against
//! a stubbed `fetch`.

use std::{future::poll_fn, task::Poll};

use leptos_hl_contact::{
    ContactDelivery, ContactDeliveryError, ContactInput,
    delivery::resend::{ResendConfig, ResendDelivery},
};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::FetchStub;

/// Never contacted: `fetch` is stubbed in every test.
fn delivery() -> ResendDelivery {
    ResendDelivery::new(ResendConfig::new(
        "test-key",
        "noreply@example.com",
        "admin@example.com",
    ))
}

fn input() -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        Some("Hello".into()),
        "This is a test message.".into(),
        String::new(),
    )
}

/// RFC 017 D3, NFR-PORT-02: the same request shape as the native path — a
/// bearer-authorized JSON `POST` carrying `reply_to`.
#[wasm_bindgen_test]
async fn the_request_is_a_bearer_authorized_json_post() {
    let stub = FetchStub::answering(200, r#"{"id":"re_abc123"}"#);
    delivery().deliver(input()).await.expect("delivered");

    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    let sent = &requests[0];
    assert_eq!(sent.method(), "POST");
    assert!(sent.url().ends_with("/emails"), "{}", sent.url());
    assert_eq!(
        sent.headers().get("authorization").unwrap().as_deref(),
        Some("Bearer test-key")
    );
    assert_eq!(
        sent.headers().get("content-type").unwrap().as_deref(),
        Some("application/json")
    );

    let body = JsFuture::from(sent.text().unwrap()).await.unwrap();
    let body: serde_json::Value = serde_json::from_str(body.as_string().unwrap().as_str()).unwrap();
    assert_eq!(body["from"], "noreply@example.com");
    assert_eq!(body["to"], "admin@example.com");
    assert_eq!(body["reply_to"], "alice@example.com");
}

/// RFC 017 D4: a 5xx is `Transport`, naming the status and nothing else.
#[wasm_bindgen_test]
async fn a_5xx_is_transport() {
    let _stub = FetchStub::answering(500, "");
    match delivery().deliver(input()).await {
        Err(ContactDeliveryError::Transport(m)) => assert_eq!(m, "HTTP 500"),
        other => panic!("expected Transport, got {other:?}"),
    }
}

/// RFC 017 D3, D5: a delivery dropped mid-flight, as when the submission is
/// cancelled, aborts its request.
#[wasm_bindgen_test]
async fn a_dropped_delivery_aborts_its_request() {
    let stub = FetchStub::silent();
    let d = delivery();
    let mut future = d.deliver(input());
    let first = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await;
    assert!(first.is_pending(), "the vendor has not answered");

    let requests = stub.requests();
    assert_eq!(requests.len(), 1, "the first poll sent the request");
    assert!(!requests[0].signal().aborted(), "in flight");

    drop(future);
    assert!(
        requests[0].signal().aborted(),
        "dropping aborts the request"
    );
}
