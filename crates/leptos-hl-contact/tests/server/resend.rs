//! What a failing Resend delivery logs (RFC 017 D4, D6): the status, and
//! never a value from the request.
//!
//! `ResendDelivery` is called directly, as the site-field validation tests
//! call `validate_site_fields` directly: there is no router step that would
//! change what reaches the log.

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use leptos_hl_contact::{
    ContactDelivery, ContactInput,
    delivery::resend::{ResendConfig, ResendDelivery},
};

use crate::support::capture_logs;

/// A distinctive fake key and value in every field, so their absence from
/// the captured output cannot be a coincidence.
const PROBE_KEY: &str = "zz-probe-resend-key";
const PROBE_NAME: &str = "Zz Probe Name";
const PROBE_EMAIL: &str = "zz-probe@example.test";
const PROBE_SUBJECT: &str = "Zz Probe Subject";
const PROBE_MESSAGE: &str = "Zz probe message, more than one word.";
/// What the vendor's own answer says — never logged either.
const PROBE_VENDOR_TEXT: &str = "zz-probe-vendor-detail";

fn probed_input() -> ContactInput {
    ContactInput::from_raw(
        PROBE_NAME.into(),
        PROBE_EMAIL.into(),
        Some(PROBE_SUBJECT.into()),
        PROBE_MESSAGE.into(),
        String::new(),
    )
}

/// A bare TCP responder answering one request with a `500` whose JSON body
/// carries the vendor probe, so the test also proves the vendor's own text
/// never reaches the log (D4: "the message carries the status and nothing
/// else").
async fn failing_responder() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/emails", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut discard = [0u8; 4096];
        let _ = socket.read(&mut discard).await;
        let body = format!(r#"{{"message":"{PROBE_VENDOR_TEXT}"}}"#);
        let response = format!(
            "HTTP/1.1 500 Internal Server Error\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();
    });
    url
}

/// FR-OBS-02, FR-OBS-03, RFC 017 D4, D6: a failing delivery logs the status,
/// and no captured line contains a request value or the key — not the
/// vendor's own error text either.
#[tokio::test]
async fn a_failing_delivery_logs_the_status_and_never_a_request_value() {
    let (logs, _guard) = capture_logs();
    let url = failing_responder().await;

    let delivery = ResendDelivery::new(ResendConfig::new(
        PROBE_KEY,
        "noreply@example.com",
        "admin@example.com",
    ))
    .with_url(&url);

    let result = delivery.deliver(probed_input()).await;
    assert!(result.is_err(), "the delivery must fail");

    assert!(
        logs.any_contains("HTTP 500"),
        "the status is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
    for probe in [
        PROBE_KEY,
        PROBE_NAME,
        PROBE_EMAIL,
        PROBE_SUBJECT,
        PROBE_MESSAGE,
        PROBE_VENDOR_TEXT,
    ] {
        for line in logs.lines() {
            assert!(!line.contains(probe), "a request value was logged: {line}");
        }
    }
}

/// A distinctive id, so a successful delivery's log line is not mistaken for
/// one that captured nothing.
const PROBE_ID: &str = "re_zz_probe_id";

async fn succeeding_responder_with_id() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/emails", listener.local_addr().unwrap());
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut discard = [0u8; 4096];
        let _ = socket.read(&mut discard).await;
        let body = format!(r#"{{"id":"{PROBE_ID}"}}"#);
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();
    });
    url
}

/// RFC 017 D4: a successful delivery with an `id` in the answer logs it.
#[tokio::test]
async fn a_successful_delivery_with_an_id_logs_it() {
    let (logs, _guard) = capture_logs();
    let url = succeeding_responder_with_id().await;

    let delivery = ResendDelivery::new(ResendConfig::new(
        "test-key",
        "noreply@example.com",
        "admin@example.com",
    ))
    .with_url(&url);

    delivery.deliver(probed_input()).await.expect("delivered");

    assert!(
        logs.any_contains(PROBE_ID),
        "the id is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
}
