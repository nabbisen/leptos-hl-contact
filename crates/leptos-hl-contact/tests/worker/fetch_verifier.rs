//! `HttpChallengeVerifier` over `fetch` on a wasm32 server (RFC 011 D3, D5),
//! against a stubbed `fetch`.

use std::{
    future::poll_fn,
    net::{IpAddr, Ipv4Addr},
    task::Poll,
    time::Duration,
};

use leptos_hl_contact::{
    ChallengeError, ChallengeProvider, ChallengeRequest, ChallengeVerifier, HttpChallengeVerifier,
};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::RequestRedirect;

use crate::support::FetchStub;

/// Never contacted: `fetch` is stubbed in every test.
const VERIFY_URL: &str = "https://siteverify.invalid/siteverify";
const VISITOR_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7));

fn verifier() -> HttpChallengeVerifier {
    HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "test-secret")
        .with_verify_url(VERIFY_URL)
}

/// FR-ABUSE-10, FR-ABUSE-12, NFR-PORT-02 (RFC 011 D3, D5): the request is a
/// form `POST` of `secret`, `response` and `remoteip` that refuses redirects,
/// so the secret is never resent to a `Location`.  A request that finished is
/// not aborted.
#[wasm_bindgen_test]
async fn the_request_refuses_redirects_and_posts_the_form() {
    let stub = FetchStub::answering(200, r#"{"success":true}"#);
    let request = ChallengeRequest::new("the token").with_remote_ip(VISITOR_IP);
    let outcome = verifier().verify_request(&request).await.expect("answered");
    assert!(outcome.passed);

    let requests = stub.requests();
    assert_eq!(requests.len(), 1);
    let sent = &requests[0];
    assert_eq!(sent.url(), VERIFY_URL);
    assert_eq!(sent.method(), "POST");
    assert_eq!(sent.redirect(), RequestRedirect::Manual);
    assert_eq!(
        sent.headers().get("content-type").unwrap().as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    let body = JsFuture::from(sent.text().unwrap()).await.unwrap();
    assert_eq!(
        body.as_string().as_deref(),
        Some("secret=test-secret&response=the+token&remoteip=203.0.113.7")
    );
    assert!(
        !sent.signal().aborted(),
        "a finished request is not aborted"
    );
}

/// FR-ABUSE-12, NFR-PORT-02 (RFC 011 D3): a redirect is an answer the
/// verifier does not accept.
#[wasm_bindgen_test]
async fn a_redirect_is_unavailable() {
    let _stub = FetchStub::answering(302, "");
    match verifier().verify("tok").await {
        Err(ChallengeError::Unavailable(message)) => assert_eq!(message, "HTTP 302"),
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

/// FR-ABUSE-10, NFR-PORT-02 (RFC 011 D3): the body goes through the shared
/// parser.
#[wasm_bindgen_test]
async fn a_verdict_is_parsed() {
    {
        let _stub = FetchStub::answering(200, r#"{"success":true}"#);
        let outcome = verifier().verify("tok").await.expect("answered");
        assert!(outcome.passed);
        assert!(outcome.error_codes.is_empty());
    }
    let _stub = FetchStub::answering(200, r#"{"success":false,"error-codes":["x"]}"#);
    let outcome = verifier().verify("tok").await.expect("answered");
    assert!(!outcome.passed);
    assert_eq!(outcome.error_codes, ["x"]);
}

/// FR-ABUSE-12, NFR-PORT-02 (RFC 011 D3): a vendor that never answers ends at
/// the time limit as `Timeout`, and the request is aborted.  This waits 50 ms
/// for real.
#[wasm_bindgen_test]
async fn a_silent_vendor_is_a_timeout() {
    let stub = FetchStub::silent();
    let started = js_sys::Date::now();
    let result = verifier()
        .with_timeout(Duration::from_millis(50))
        .verify("tok")
        .await;
    let elapsed_ms = js_sys::Date::now() - started;

    assert!(matches!(result, Err(ChallengeError::Timeout)), "{result:?}");
    assert!(elapsed_ms >= 50.0, "returned after {elapsed_ms} ms");
    assert!(
        stub.requests()[0].signal().aborted(),
        "the request is aborted"
    );
}

/// FR-ABUSE-12, NFR-PORT-02 (RFC 011 D3): an empty secret fails closed
/// before anything is sent.
#[wasm_bindgen_test]
async fn an_empty_secret_sends_nothing() {
    let stub = FetchStub::answering(200, r#"{"success":true}"#);
    let result = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "")
        .with_verify_url(VERIFY_URL)
        .verify("tok")
        .await;
    assert!(
        matches!(result, Err(ChallengeError::Misconfigured(_))),
        "{result:?}"
    );
    assert!(stub.requests().is_empty(), "no request may be sent");
}

/// FR-ABUSE-12, NFR-PORT-02 (RFC 011 D3): a verification dropped mid-flight,
/// as when the submission is cancelled, aborts its request.
#[wasm_bindgen_test]
async fn a_dropped_verification_aborts_its_request() {
    let stub = FetchStub::silent();
    let verifier = verifier();
    let mut verification = verifier.verify("tok");
    let first = poll_fn(|cx| Poll::Ready(verification.as_mut().poll(cx))).await;
    assert!(first.is_pending(), "the vendor has not answered");

    let requests = stub.requests();
    assert_eq!(requests.len(), 1, "the first poll sent the request");
    assert!(!requests[0].signal().aborted(), "in flight");

    drop(verification);
    assert!(
        requests[0].signal().aborted(),
        "dropping aborts the request"
    );
}

/// The largest response body the shared `http` module reads, matching its
/// own `MAX_RESPONSE_BODY` (RFC 017 handoff 01 review).
const MAX_RESPONSE_BODY: usize = 64 * 1024;

/// A body of exactly the cap, valid JSON padded with leading whitespace
/// (insignificant to a JSON parser), is read and parsed whole.
#[wasm_bindgen_test]
async fn a_body_at_the_cap_is_returned_whole() {
    let json = r#"{"success":true}"#;
    let body = format!("{}{json}", " ".repeat(MAX_RESPONSE_BODY - json.len()));
    assert_eq!(body.len(), MAX_RESPONSE_BODY);

    let _stub = FetchStub::answering(200, &body);
    let outcome = verifier().verify("tok").await.expect("answered");
    assert!(outcome.passed);
}

/// One byte over the cap is `Unavailable`, not parsed.
#[wasm_bindgen_test]
async fn an_oversized_body_is_unavailable() {
    let body = " ".repeat(MAX_RESPONSE_BODY + 1);
    let _stub = FetchStub::answering(200, &body);
    match verifier().verify("tok").await {
        Err(ChallengeError::Unavailable(m)) => assert_eq!(m, "response too large"),
        other => panic!("expected Unavailable, got {other:?}"),
    }
}
