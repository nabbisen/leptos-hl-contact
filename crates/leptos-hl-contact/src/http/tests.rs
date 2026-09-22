// tests.rs — unit tests for the parent module.
//
// `HttpClient` itself, native only; the challenge and delivery suites cover
// the request shape and the error mapping through their own public entry
// points.

use std::time::Duration;

use tokio::{net::TcpListener, sync::oneshot};

use super::*;

/// Accept one connection, answer with `body`, and report whether one was
/// made at all (nothing else about the request matters here).
async fn respond_once(body: &'static [u8]) -> (String, oneshot::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut discard = [0u8; 1024];
        let _ = socket.read(&mut discard).await;
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
        socket.shutdown().await.ok();
        let _ = tx.send(());
    });
    (url, rx)
}

fn request(url: &str) -> HttpRequest<'_> {
    HttpRequest {
        url,
        headers: &[],
        body: HttpBody::Form(&[]),
        limit: Duration::from_secs(5),
    }
}

/// RFC 017 handoff 01 review: a body of exactly `MAX_RESPONSE_BODY` bytes is
/// returned whole.
#[tokio::test]
async fn a_body_at_the_cap_is_returned_whole() {
    let body: &'static [u8] = Box::leak(vec![b'x'; MAX_RESPONSE_BODY].into_boxed_slice());
    let (url, _rx) = respond_once(body).await;

    let response = HttpClient::new().post(request(&url)).await.expect("ok");
    assert_eq!(response.status, 200);
    assert_eq!(response.body.len(), MAX_RESPONSE_BODY);
}

/// One byte over the cap is `Unusable`, not parsed.
#[tokio::test]
async fn a_body_one_byte_over_the_cap_is_unusable() {
    let body: &'static [u8] = Box::leak(vec![b'x'; MAX_RESPONSE_BODY + 1].into_boxed_slice());
    let (url, _rx) = respond_once(body).await;

    match HttpClient::new().post(request(&url)).await {
        Err(HttpError::Unusable(reason)) => assert_eq!(reason, "response too large"),
        Ok(response) => panic!(
            "expected Unusable, got a {}-byte response",
            response.body.len()
        ),
        Err(_) => panic!("expected Unusable, got a different error"),
    }
}
