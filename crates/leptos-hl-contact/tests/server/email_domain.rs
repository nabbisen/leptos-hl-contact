//! The email-domain check, through the router (RFC 018 D2, D3): `check`
//! itself is `pub(crate)`, so every test here goes through `submit_contact`
//! exactly as a visitor's request does, against a local DoH-shaped
//! responder.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

use leptos_hl_contact::{ChallengeContext, ChallengePolicy, FieldError, FieldErrorCode};

use crate::support::{Harness, ScriptedVerifier, Setup, capture_logs};

/// A domain with no mail route: a null MX (RFC 7505).
const REJECTING_DOMAIN: &str = "zz-probe-no-route.example.test";

/// Answer every connection with the same `body`, once each, forever — so a
/// test that expects exactly one query does not hang if a bug makes a
/// second.
async fn responder(status_line: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dns-query", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut discard = [0u8; 1024];
            let _ = socket.read(&mut discard).await;
            let head = format!(
                "{status_line}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(head.as_bytes()).await;
            let _ = socket.write_all(body.as_bytes()).await;
            socket.shutdown().await.ok();
        }
    });
    url
}

/// A responder that counts every connection it accepts, and answers each
/// with `body` — used to prove a lookup was **not** made.
async fn counting_responder(
    status_line: &'static str,
    body: &'static str,
) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/dns-query", listener.local_addr().unwrap());
    let count = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&count);
    tokio::spawn(async move {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            counted.fetch_add(1, Ordering::SeqCst);
            let mut discard = [0u8; 1024];
            let _ = socket.read(&mut discard).await;
            let head = format!(
                "{status_line}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
                body.len()
            );
            let _ = socket.write_all(head.as_bytes()).await;
            let _ = socket.write_all(body.as_bytes()).await;
            socket.shutdown().await.ok();
        }
    });
    (url, count)
}

const NULL_MX: &str = r#"{"Status":0,"Answer":[{"name":"x","type":15,"TTL":300,"data":"0 ."}]}"#;
const SERVFAIL: &str = r#"{"Status":2,"Comment":["probe"]}"#;
const MX_PRESENT: &str =
    r#"{"Status":0,"Answer":[{"name":"x","type":15,"TTL":300,"data":"0 mx.example.test."}]}"#;

/// FR-VAL-02, FR-OBS-01, RFC 018 D2, D3: a domain with no route is refused
/// under the email field, in both request forms, and nothing is delivered.
#[tokio::test]
async fn a_domain_with_no_route_is_refused_in_both_forms() {
    let url = responder("HTTP/1.1 200 OK", NULL_MX).await;
    let h = Harness::new(Setup {
        email_domain_check: Some(url),
        ..Setup::default()
    });
    let fields = h
        .fields()
        .set("email", &format!("visitor@{REJECTING_DOMAIN}"));

    let fetch = h.submit_fetch(&fields).await;
    assert_eq!(
        fetch.field_errors().and_then(|e| e.email),
        Some(FieldError::Code(FieldErrorCode::EmailDomain)),
        "{}",
        fetch.body
    );

    let nojs = h.submit_nojs(&fields).await;
    assert!(nojs.is_nojs_error(), "no-JS");
    let page = h.follow(&nojs).await;
    assert_eq!(page.aria_invalid_count(), 1, "no-JS");
    assert_eq!(page.banner(), None, "no-JS");

    assert_eq!(h.deliveries(), 0);
}

/// RFC 018 D2: the same rejecting domain, with no `EmailDomainCheck` in
/// context, is delivered — the check never ran at all.
#[tokio::test]
async fn absent_from_context_the_check_never_runs() {
    let h = Harness::new(Setup::default());
    let fields = h
        .fields()
        .set("email", &format!("visitor@{REJECTING_DOMAIN}"));
    let reply = h.submit_fetch(&fields).await;
    assert!(reply.status.is_success(), "{}", reply.body);
    assert_eq!(h.deliveries(), 1);
}

/// RFC 018 D1: SERVFAIL is a lookup failure, not the domain saying no —
/// the submission is delivered, and a `warn` names the fixed reason.
#[tokio::test]
async fn a_servfail_answer_still_delivers_and_logs_a_warning() {
    let (logs, _guard) = capture_logs();
    let url = responder("HTTP/1.1 200 OK", SERVFAIL).await;
    let h = Harness::new(Setup {
        email_domain_check: Some(url),
        ..Setup::default()
    });
    let reply = h.submit_fetch(&h.fields()).await;

    assert!(reply.status.is_success(), "{}", reply.body);
    assert_eq!(h.deliveries(), 1);
    assert!(
        logs.any_contains("email domain check unavailable"),
        "{:?}",
        logs.lines()
    );
    assert!(logs.any_contains("servfail"), "{:?}", logs.lines());
}

/// RFC 018 D2: a honeypot hit ends the submission before the domain check
/// ever runs — the stub resolver sees no connection at all.
#[tokio::test]
async fn a_honeypot_hit_never_reaches_the_resolver() {
    let (url, count) = counting_responder("HTTP/1.1 200 OK", NULL_MX).await;
    let h = Harness::new(Setup {
        email_domain_check: Some(url),
        ..Setup::default()
    });
    let fields = h.fields().set("website", "http://bot.example");
    let reply = h.submit_fetch(&fields).await;

    assert!(reply.status.is_success(), "{}", reply.body);
    assert_eq!(h.deliveries(), 0, "the honeypot silently drops it");
    assert_eq!(count.load(Ordering::SeqCst), 0, "no lookup was made");
}

/// RFC 018 D2: an address that already fails syntax validation is refused
/// exactly as it always was — `format`, not `email_domain` — and the domain
/// check is never reached, so the stub resolver sees no connection.
#[tokio::test]
async fn a_syntax_invalid_address_is_refused_without_a_lookup() {
    let (url, count) = counting_responder("HTTP/1.1 200 OK", NULL_MX).await;
    let h = Harness::new(Setup {
        email_domain_check: Some(url),
        ..Setup::default()
    });
    let fields = h.fields().set("email", "not-an-email");
    let reply = h.submit_fetch(&fields).await;

    assert_eq!(
        reply.field_errors().and_then(|e| e.email),
        Some(FieldError::Code(FieldErrorCode::Format)),
        "{}",
        reply.body
    );
    assert_eq!(count.load(Ordering::SeqCst), 0, "no lookup was made");
}

/// RFC 018 D2: the domain check runs before the challenge, so a submission
/// it refuses never reaches the verifier at all.
#[tokio::test]
async fn the_challenge_is_not_reached_when_the_domain_is_refused() {
    let url = responder("HTTP/1.1 200 OK", NULL_MX).await;
    let verifier = Arc::new(ScriptedVerifier::passing());
    let h = Harness::new(Setup {
        email_domain_check: Some(url),
        challenge: Some(ChallengeContext {
            verifier: Arc::clone(&verifier) as _,
            policy: ChallengePolicy::default(),
        }),
        ..Setup::default()
    });
    let fields = h
        .fields()
        .set("email", &format!("visitor@{REJECTING_DOMAIN}"))
        .set("cf-turnstile-response", "a-token");
    let reply = h.submit_fetch(&fields).await;

    assert_eq!(
        reply.field_errors().and_then(|e| e.email),
        Some(FieldError::Code(FieldErrorCode::EmailDomain)),
        "{}",
        reply.body
    );
    assert!(
        verifier.seen().is_empty(),
        "the challenge must not be asked: {:?}",
        verifier.seen()
    );
}

/// FR-OBS-01, NFR-PRIV-*: neither the probe address nor the probe domain
/// appears in any captured line, whichever verdict the lookup reaches.
#[tokio::test]
async fn no_address_and_no_domain_is_ever_logged() {
    let (logs, _guard) = capture_logs();

    for body in [NULL_MX, SERVFAIL, MX_PRESENT] {
        let url = responder("HTTP/1.1 200 OK", body).await;
        let h = Harness::new(Setup {
            email_domain_check: Some(url),
            ..Setup::default()
        });
        let probe_email = format!("zz-probe-address@{REJECTING_DOMAIN}");
        h.submit_fetch(&h.fields().set("email", &probe_email)).await;
    }

    for line in logs.lines() {
        assert!(!line.contains("zz-probe-address"), "the address: {line}");
        assert!(!line.contains(REJECTING_DOMAIN), "the domain: {line}");
    }
}
