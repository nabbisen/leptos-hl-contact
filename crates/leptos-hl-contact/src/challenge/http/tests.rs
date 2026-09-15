// tests.rs — HttpChallengeVerifier against a local responder, plus live
// vendor tests that run only with `--ignored`.

use std::time::Duration;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};

use super::*;

/// Accept one connection, capture the request, answer with `response`.
///
/// Returns the URL to verify against and the captured request text.
async fn respond_once(response: &'static str) -> (String, oneshot::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/siteverify", listener.local_addr().unwrap());
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
    let mut chunk = [0u8; 1024];
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

fn ok_json(body: &str) -> &'static str {
    Box::leak(
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_boxed_str(),
    )
}

fn verifier(url: &str) -> HttpChallengeVerifier {
    HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "test-secret").with_verify_url(url)
}

// ---- response shapes ---------------------------------------------------------

#[tokio::test]
async fn turnstile_and_hcaptcha_success_shape() {
    let (url, _) = respond_once(ok_json(
        r#"{"success":true,"challenge_ts":"2026-09-13T00:00:00Z","hostname":"example.com","error-codes":[],"action":"","cdata":""}"#,
    ))
    .await;
    let outcome = verifier(&url).verify("tok").await.expect("answered");
    assert!(outcome.passed);
    assert_eq!(outcome.score, None);
    assert!(outcome.error_codes.is_empty());
}

#[tokio::test]
async fn recaptcha_v3_success_shape_carries_score_and_action() {
    let (url, _) = respond_once(ok_json(
        r#"{"success":true,"score":0.9,"action":"contact","challenge_ts":"2026-09-13T00:00:00Z","hostname":"example.com"}"#,
    ))
    .await;
    let outcome = verifier(&url).verify("tok").await.expect("answered");
    assert!(outcome.passed);
    assert_eq!(outcome.score, Some(0.9));
    assert_eq!(outcome.action.as_deref(), Some("contact"));
}

#[tokio::test]
async fn recaptcha_v2_success_shape_has_no_score() {
    let (url, _) = respond_once(ok_json(
        r#"{"success":true,"challenge_ts":"2026-09-13T00:00:00Z","hostname":"example.com"}"#,
    ))
    .await;
    let outcome = verifier(&url).verify("tok").await.expect("answered");
    assert!(outcome.passed);
    assert_eq!((outcome.score, outcome.action), (None, None));
}

#[tokio::test]
async fn failure_carries_the_vendor_error_codes() {
    let (url, _) = respond_once(ok_json(
        r#"{"success":false,"error-codes":["invalid-input-response","timeout-or-duplicate"]}"#,
    ))
    .await;
    let outcome = verifier(&url).verify("tok").await.expect("answered");
    assert!(!outcome.passed);
    assert_eq!(
        outcome.error_codes,
        ["invalid-input-response", "timeout-or-duplicate"]
    );
}

// ---- the request --------------------------------------------------------------

#[tokio::test]
async fn the_request_is_a_form_post_of_secret_and_response() {
    let (url, request) = respond_once(ok_json(r#"{"success":true}"#)).await;
    verifier(&url).verify("the token").await.expect("answered");
    let request = request.await.unwrap();
    assert!(request.starts_with("POST /siteverify "), "{request}");
    assert!(
        request
            .to_ascii_lowercase()
            .contains("content-type: application/x-www-form-urlencoded"),
        "{request}"
    );
    assert!(
        request.ends_with("secret=test-secret&response=the+token"),
        "{request}"
    );
}

/// FR-ABUSE-10, RFC 011 D5: the visitor's IP goes to the vendor as `remoteip`
/// when the site provides it, and nothing of the kind otherwise.
#[tokio::test]
async fn the_form_body_carries_remoteip_when_provided() {
    let ip: std::net::IpAddr = "203.0.113.7".parse().unwrap();

    let (url, request) = respond_once(ok_json(r#"{"success":true}"#)).await;
    verifier(&url)
        .verify_request(&ChallengeRequest::new("the token").with_remote_ip(ip))
        .await
        .expect("answered");
    let request = request.await.unwrap();
    assert!(
        request.ends_with("secret=test-secret&response=the+token&remoteip=203.0.113.7"),
        "{request}"
    );

    let (url, request) = respond_once(ok_json(r#"{"success":true}"#)).await;
    verifier(&url)
        .verify_request(&ChallengeRequest::new("the token"))
        .await
        .expect("answered");
    let request = request.await.unwrap();
    assert!(!request.contains("remoteip"), "{request}");
}

/// RFC 011 D5: `verify` is `verify_request` without an IP — the same body on
/// the wire.
#[tokio::test]
async fn verify_is_verify_request_without_an_ip() {
    let body = |request: String| request.split("\r\n\r\n").nth(1).unwrap().to_owned();

    let (url, by_verify) = respond_once(ok_json(r#"{"success":true}"#)).await;
    verifier(&url).verify("the token").await.expect("answered");
    let (url, by_request) = respond_once(ok_json(r#"{"success":true}"#)).await;
    verifier(&url)
        .verify_request(&ChallengeRequest::new("the token"))
        .await
        .expect("answered");

    let (by_verify, by_request) = (
        body(by_verify.await.unwrap()),
        body(by_request.await.unwrap()),
    );
    assert_eq!(by_verify, by_request);
    assert_eq!(by_verify, "secret=test-secret&response=the+token");
}

// ---- errors: all Unavailable, Timeout or Misconfigured -----------------------------

#[tokio::test]
async fn a_server_error_is_unavailable() {
    let (url, _) = respond_once(
        "HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
    )
    .await;
    match verifier(&url).verify("tok").await {
        Err(ChallengeError::Unavailable(m)) => assert!(m.contains("500"), "{m}"),
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

/// Review C2: a redirect would resend the body, secret included, to the
/// `Location`.  It must not be followed, and must count as unavailable.
#[tokio::test]
async fn a_redirect_is_not_followed_and_is_unavailable() {
    let elsewhere = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let elsewhere_url = format!("http://{}/steal", elsewhere.local_addr().unwrap());
    let redirect: &'static str = Box::leak(
        format!(
            "HTTP/1.1 307 Temporary Redirect\r\nlocation: {elsewhere_url}\r\ncontent-length: 0\r\nconnection: close\r\n\r\n"
        )
        .into_boxed_str(),
    );
    let (url, _) = respond_once(redirect).await;

    match verifier(&url).verify("tok").await {
        Err(ChallengeError::Unavailable(m)) => assert!(m.contains("307"), "{m}"),
        other => panic!("expected Unavailable, got {other:?}"),
    }
    let followed = tokio::time::timeout(Duration::from_millis(300), elsewhere.accept()).await;
    assert!(
        followed.is_err(),
        "the redirect target must see no connection"
    );
}

/// Not JSON, or JSON without `success`: the vendor did not answer, so this
/// is unavailable, not a failed challenge.
#[tokio::test]
async fn malformed_bodies_are_unavailable_not_failed() {
    for body in [
        "<html>maintenance</html>",
        r#"{"hostname":"example.com"}"#,
        r#"{"success":"yes"}"#,
    ] {
        let (url, _) = respond_once(ok_json(body)).await;
        match verifier(&url).verify("tok").await {
            Err(ChallengeError::Unavailable(m)) => assert_eq!(m, "malformed response"),
            other => panic!("{body}: expected Unavailable, got {other:?}"),
        }
    }
}

#[tokio::test]
async fn a_silent_server_is_a_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/siteverify", listener.local_addr().unwrap());
    // Accept and hold the connection without ever answering.
    let _held = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        drop(socket);
    });
    let started = std::time::Instant::now();
    let result = verifier(&url)
        .with_timeout(Duration::from_millis(200))
        .verify("tok")
        .await;
    assert!(matches!(result, Err(ChallengeError::Timeout)), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(2), "the cap held");
}

#[tokio::test]
async fn a_refused_connection_is_unavailable() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/siteverify", listener.local_addr().unwrap());
    drop(listener);
    assert!(matches!(
        verifier(&url).verify("tok").await,
        Err(ChallengeError::Unavailable(_))
    ));
}

/// Fail closed without sending anything anywhere.
#[tokio::test]
async fn an_empty_secret_is_misconfigured_and_sends_nothing() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/siteverify", listener.local_addr().unwrap());
    let v = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "").with_verify_url(&url);
    assert!(matches!(
        v.verify("tok").await,
        Err(ChallengeError::Misconfigured(_))
    ));
    let accepted = tokio::time::timeout(Duration::from_millis(200), listener.accept()).await;
    assert!(accepted.is_err(), "no connection may be made");
}

// ---- configuration -------------------------------------------------------------

#[test]
fn debug_redacts_the_secret() {
    let v = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "super-secret-value");
    let debug = format!("{v:?}");
    assert!(!debug.contains("super-secret-value"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
}

#[test]
fn each_provider_uses_its_vendor_endpoint_by_default() {
    let url = |p| HttpChallengeVerifier::new(p, "s").endpoint().to_owned();
    assert_eq!(url(ChallengeProvider::Turnstile), TURNSTILE_URL);
    assert_eq!(url(ChallengeProvider::HCaptcha), HCAPTCHA_URL);
    assert_eq!(url(ChallengeProvider::RecaptchaV2), RECAPTCHA_URL);
    assert_eq!(
        url(ChallengeProvider::RecaptchaV3 {
            action: "contact".into()
        }),
        RECAPTCHA_URL
    );
    assert_eq!(
        HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "s")
            .with_verify_url("http://proxy/siteverify")
            .endpoint(),
        "http://proxy/siteverify"
    );
}

#[test]
fn the_default_timeout_is_five_seconds() {
    let v = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "s");
    assert_eq!(v.timeout, Duration::from_secs(5));
    assert_eq!(
        v.with_timeout(Duration::from_secs(2)).timeout,
        Duration::from_secs(2)
    );
}

// ---- live: the vendors' published test keys ---------------------------------------
//
//     cargo test -p leptos-hl-contact --features challenge-http -- --ignored live_

const TURNSTILE_PASS_SECRET: &str = "1x0000000000000000000000000000000AA";
const TURNSTILE_FAIL_SECRET: &str = "2x0000000000000000000000000000000AA";
const TURNSTILE_TOKEN: &str = "XXXX.DUMMY.TOKEN.XXXX";
const HCAPTCHA_SECRET: &str = "0x0000000000000000000000000000000000000000";
const HCAPTCHA_TOKEN: &str = "10000000-aaaa-bbbb-cccc-000000000001";
const RECAPTCHA_SECRET: &str = "6LeIxAcTAAAAAGG-vFI1TnRWxMZNFuojJ4WifJWe";

#[tokio::test]
#[ignore = "calls Cloudflare"]
async fn live_turnstile_pass_secret_passes() {
    let outcome = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, TURNSTILE_PASS_SECRET)
        .verify(TURNSTILE_TOKEN)
        .await
        .expect("Cloudflare answered");
    println!("turnstile pass: {outcome:?}");
    assert!(outcome.passed);
}

#[tokio::test]
#[ignore = "calls Cloudflare"]
async fn live_turnstile_fail_secret_fails() {
    let outcome = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, TURNSTILE_FAIL_SECRET)
        .verify(TURNSTILE_TOKEN)
        .await
        .expect("Cloudflare answered");
    println!("turnstile fail: {outcome:?}");
    assert!(!outcome.passed);
    assert!(!outcome.error_codes.is_empty());
}

#[tokio::test]
#[ignore = "calls hCaptcha"]
async fn live_hcaptcha_test_secret_passes() {
    let outcome = HttpChallengeVerifier::new(ChallengeProvider::HCaptcha, HCAPTCHA_SECRET)
        .verify(HCAPTCHA_TOKEN)
        .await
        .expect("hCaptcha answered");
    println!("hcaptcha: {outcome:?}");
    assert!(outcome.passed);
}

/// Google's test secret accepts any token, so this shows the round trip, not
/// a real check.
#[tokio::test]
#[ignore = "calls Google"]
async fn live_recaptcha_test_secret_passes() {
    let outcome = HttpChallengeVerifier::new(ChallengeProvider::RecaptchaV2, RECAPTCHA_SECRET)
        .verify("any-token")
        .await
        .expect("Google answered");
    println!("recaptcha: {outcome:?}");
    assert!(outcome.passed);
}
