// tests.rs — unit tests for the parent module, against a local responder
// (the pattern `challenge/http/tests.rs` uses).

use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

use super::*;
use crate::model::SiteFieldValue;

/// Accept one connection, capture the request, answer with `response`.
///
/// Returns the URL to deliver to and the captured request text.
async fn respond_once(response: &'static str) -> (String, oneshot::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/emails", listener.local_addr().unwrap());
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_request(&mut socket).await;
        let _ = tx.send(request);
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.ok();
    });
    (url, rx)
}

/// Read headers and a `Content-Length` body.
async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = socket.read(&mut chunk).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let text = String::from_utf8_lossy(&buf);
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|l| {
                    l.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if buf.len() >= end + 4 + length {
                break;
            }
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

fn status_only(code: u16) -> &'static str {
    Box::leak(
        format!("HTTP/1.1 {code} status\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
            .into_boxed_str(),
    )
}

fn json_response(body: &str) -> &'static str {
    json_response_with_status(200, body)
}

fn json_response_with_status(code: u16, body: &str) -> &'static str {
    Box::leak(
        format!(
            "HTTP/1.1 {code} status\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_boxed_str(),
    )
}

/// The body, as a JSON value, of a captured request.
fn body_of(request: &str) -> serde_json::Value {
    let start = request
        .find("\r\n\r\n")
        .expect("headers, a blank line, a body")
        + 4;
    serde_json::from_str(&request[start..]).expect("a JSON body")
}

fn config() -> ResendConfig {
    ResendConfig::new("test-key", "noreply@example.com", "admin@example.com")
}

fn delivery(url: &str) -> ResendDelivery {
    ResendDelivery::new(config()).with_url(url)
}

fn sample_input() -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        Some("Hello".into()),
        "This is a test message.".into(),
        String::new(),
    )
}

fn site_value(key: &str, label: &str, value: &str, value_label: Option<&str>) -> SiteFieldValue {
    SiteFieldValue {
        key: key.into(),
        label: label.into(),
        value: value.into(),
        value_label: value_label.map(Into::into),
    }
}

// ---- the request -----------------------------------------------------------

/// RFC 017 D3: the URL path, the bearer header, and the JSON field names and
/// values, `reply_to` included.
#[tokio::test]
async fn the_request_is_a_bearer_authorized_json_post_with_reply_to() {
    let (url, rx) = respond_once(json_response(r#"{"id":"abc"}"#)).await;
    delivery(&url)
        .deliver(sample_input())
        .await
        .expect("delivered");

    let request = rx.await.unwrap();
    assert!(request.starts_with("POST /emails "), "{request}");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer test-key"),
        "{request}"
    );
    assert!(
        request
            .to_ascii_lowercase()
            .contains("content-type: application/json"),
        "{request}"
    );

    let body = body_of(&request);
    assert_eq!(body["from"], "noreply@example.com");
    assert_eq!(body["to"], "admin@example.com");
    assert_eq!(body["reply_to"], "alice@example.com");
    assert_eq!(body["subject"], "Hello");
}

/// RFC 017 D3: the subject is composed exactly as `delivery/smtp.rs`
/// composes it, prefix included.
#[tokio::test]
async fn the_subject_carries_the_configured_prefix() {
    let (url, rx) = respond_once(json_response(r#"{"id":"abc"}"#)).await;
    let delivery = ResendDelivery::new(config().with_subject_prefix("[Contact]")).with_url(&url);
    delivery.deliver(sample_input()).await.expect("delivered");

    let body = body_of(&rx.await.unwrap());
    assert_eq!(body["subject"], "[Contact] Hello");
}

/// RFC 017 D3: the body text is `delivery/body.rs`'s output for the same
/// input, including a site-defined field.
#[tokio::test]
async fn the_body_text_matches_delivery_body_rs() {
    let (url, rx) = respond_once(json_response(r#"{"id":"abc"}"#)).await;
    let mut input = sample_input();
    input.site_fields = vec![site_value(
        "organisation",
        "Organisation",
        "Example Ltd",
        None,
    )];
    let expected = build_plain_text_body(&input);

    delivery(&url).deliver(input).await.expect("delivered");

    let body = body_of(&rx.await.unwrap());
    assert_eq!(body["text"], expected);
}

// ---- the answer --------------------------------------------------------------

/// RFC 017 D4: each status row, and the message carries the status and
/// nothing else — not the vendor's own error text.
#[tokio::test]
async fn each_status_row_maps_to_its_code_and_carries_no_vendor_text() {
    const VENDOR_TEXT: &str = "VENDOR-ERROR-DETAIL-MARKER";
    let vendor_body = format!(r#"{{"message":"{VENDOR_TEXT}"}}"#);

    for (code, expect_configuration, expect_message_build) in [
        (401, true, false),
        (403, true, false),
        (422, false, true),
        (429, false, false),
        (500, false, false),
        (418, false, false),
    ] {
        let (url, _rx) = respond_once(json_response_with_status(code, &vendor_body)).await;
        let result = delivery(&url).deliver(sample_input()).await;

        let text = match (&result, expect_configuration, expect_message_build) {
            (Err(ContactDeliveryError::Configuration(t)), true, _) => t,
            (Err(ContactDeliveryError::MessageBuild(t)), _, true) => t,
            (Err(ContactDeliveryError::Transport(t)), false, false) => t,
            other => panic!("HTTP {code}: unexpected result {other:?}"),
        };
        assert_eq!(text, &format!("HTTP {code}"), "HTTP {code}");
        assert!(!text.contains(VENDOR_TEXT), "HTTP {code}: {text}");
    }
}

/// A 2xx with an `id` in the answer succeeds.
#[tokio::test]
async fn a_2xx_with_an_id_succeeds() {
    let (url, _rx) = respond_once(json_response(r#"{"id":"re_abc123"}"#)).await;
    delivery(&url)
        .deliver(sample_input())
        .await
        .expect("delivered");
}

/// A 2xx without an `id`, or with an unparsable body, still succeeds: a
/// missing or unreadable id is not an error.
#[tokio::test]
async fn a_2xx_without_an_id_still_succeeds() {
    for body in [
        json_response("{}"),
        json_response("not json"),
        status_only(200),
    ] {
        let (url, _rx) = respond_once(body).await;
        delivery(&url)
            .deliver(sample_input())
            .await
            .expect("delivered");
    }
}

/// RFC 017 D4, FR-DEL-08: a silent endpoint ends at the configured limit as
/// `ContactDeliveryError::Timeout`, naming that limit, so the visitor sees
/// `delivery_timeout` — the same code and text a slow SMTP relay produces.
/// Found missing while writing the traceability table (handoff 03): the
/// status-row test above never exercises this path.
#[tokio::test]
async fn a_silent_endpoint_is_a_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/emails", listener.local_addr().unwrap());
    let _held = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(socket);
    });

    let started = std::time::Instant::now();
    let limit = Duration::from_millis(200);
    let cfg = config().with_timeout(limit);
    let result = ResendDelivery::new(cfg)
        .with_url(&url)
        .deliver(sample_input())
        .await;

    match result {
        Err(ContactDeliveryError::Timeout(got)) => assert_eq!(got, limit),
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(started.elapsed() < Duration::from_secs(2), "the cap held");
}

// ---- fail closed --------------------------------------------------------------

/// An empty key, sender or recipient is `Configuration`, and sends nothing.
#[tokio::test]
async fn an_empty_value_is_configuration_and_sends_nothing() {
    for (label, cfg) in [
        (
            "api_key",
            ResendConfig::new("", "noreply@example.com", "admin@example.com"),
        ),
        (
            "from_address",
            ResendConfig::new("key", "", "admin@example.com"),
        ),
        (
            "to_address",
            ResendConfig::new("key", "noreply@example.com", ""),
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/emails", listener.local_addr().unwrap());
        let result = ResendDelivery::new(cfg)
            .with_url(&url)
            .deliver(sample_input())
            .await;
        assert!(
            matches!(result, Err(ContactDeliveryError::Configuration(_))),
            "{label}: {result:?}"
        );

        let accepted = tokio::time::timeout(Duration::from_millis(200), listener.accept()).await;
        assert!(accepted.is_err(), "{label}: no connection may be made");
    }
}

// ---- configuration -------------------------------------------------------------

/// FR-CFG-04, RFC 020 D1 (row 3): both `Debug` impls must actually run —
/// `debug.contains("<redacted>")` fails under `<impl Debug for
/// ResendDelivery>::fmt -> Ok(Default::default())`, which the earlier
/// `!debug.contains("super-secret-key")` alone did not catch (an empty
/// string trivially "does not contain" the key too).
#[test]
fn debug_redacts_the_key() {
    let config = ResendConfig::new(
        "super-secret-key",
        "noreply@example.com",
        "admin@example.com",
    );
    let debug = format!("{config:?}");
    assert!(!debug.contains("super-secret-key"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");

    let delivery = ResendDelivery::new(ResendConfig::new(
        "super-secret-key",
        "noreply@example.com",
        "admin@example.com",
    ));
    let debug = format!("{delivery:?}");
    assert!(!debug.contains("super-secret-key"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
    assert!(debug.contains("ResendDelivery"), "{debug}");
}

#[test]
fn the_default_timeout_is_ten_seconds() {
    let config = ResendConfig::new("key", "noreply@example.com", "admin@example.com");
    assert_eq!(config.timeout, Duration::from_secs(10));
    assert_eq!(
        config.with_timeout(Duration::from_secs(3)).timeout,
        Duration::from_secs(3)
    );
}

// ---- break checks, required (removed by hand, not left in the tree) ----------
//
// 1. Drop `reply_to` from `ResendRequestBody` (or send an empty string):
//    `the_request_is_a_bearer_authorized_json_post_with_reply_to` fails on
//    `body["reply_to"]`.
// 2. Pass the vendor's response text into the error instead of the status
//    (e.g. `ContactDeliveryError::Transport(String::from_utf8_lossy(&response.body).into_owned())`):
//    `each_status_row_maps_to_its_code_and_carries_no_vendor_text` fails on
//    the `!text.contains(VENDOR_TEXT)` assertion, and the server suite's
//    `resend::a_failing_delivery_logs_the_status_and_never_a_request_value`
//    fails too.
