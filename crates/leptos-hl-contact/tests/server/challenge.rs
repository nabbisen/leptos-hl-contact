//! The challenge decision table, over the wire.

use std::sync::Arc;

use leptos_hl_contact::{ChallengeContext, ChallengePolicy, NoJsPolicy};

use crate::support::{Fields, Harness, ScriptedVerifier, Setup};

fn with_challenge(verifier: &Arc<ScriptedVerifier>, no_js: NoJsPolicy) -> Harness {
    Harness::new(Setup {
        challenge: Some(ChallengeContext {
            verifier: Arc::clone(verifier) as _,
            policy: ChallengePolicy {
                no_js,
                ..ChallengePolicy::default()
            },
        }),
        ..Setup::default()
    })
}

/// Submit in both forms and check the outcome in each: `None` means
/// delivered; `Some((code, banner))` means rejected with that code over fetch
/// and that banner text after the no-JavaScript redirect.
async fn expect_both(h: &Harness, fields: &Fields, rejected: Option<(&str, &str)>, row: &str) {
    let fetch = h.submit_fetch(fields).await;
    let nojs = h.submit_nojs(fields).await;
    match rejected {
        None => {
            assert_eq!(fetch.contact_error(), None, "{row}, fetch: {}", fetch.body);
            assert!(
                nojs.is_nojs_success(),
                "{row}, no-JS: {:?}",
                nojs.location()
            );
        }
        Some((code, banner)) => {
            assert_eq!(fetch.contact_error().as_deref(), Some(code), "{row}, fetch");
            assert!(nojs.is_nojs_error(), "{row}, no-JS: {:?}", nojs.location());
            assert_eq!(
                h.follow(&nojs).await.banner().as_deref(),
                Some(banner),
                "{row}, no-JS"
            );
        }
    }
}

/// FR-ABUSE-10, FR-ABUSE-11, FR-ABUSE-12, T19: all seven rows of RFC 005's
/// decision table through `ScriptedVerifier`, in both request forms, with the
/// vendors' hyphenated field names on the wire and the token reaching the
/// verifier unchanged.
#[tokio::test]
async fn challenge_decision_table_rows_1_to_7() {
    // Row 1: no context, no token — proceed.
    let h = Harness::new(Setup::default());
    expect_both(&h, &h.fields(), None, "row 1").await;
    assert_eq!(h.deliveries(), 2, "row 1");

    // Row 2: no context, a token — not_configured, as a server error.
    let h = Harness::new(Setup::default());
    let fields = h.fields().set("cf-turnstile-response", "row-2-token");
    expect_both(
        &h,
        &fields,
        Some(("not_configured", "This form is not available right now.")),
        "row 2",
    )
    .await;
    assert!(h.submit_fetch(&fields).await.is_server_error(), "row 2");
    assert_eq!(h.deliveries(), 0, "row 2");

    // Row 3: context, no token, `Reject` — challenge_required.
    let verifier = Arc::new(ScriptedVerifier::passing());
    let h = with_challenge(&verifier, NoJsPolicy::Reject);
    expect_both(
        &h,
        &h.fields(),
        Some(("challenge_required", "Please complete the security check.")),
        "row 3",
    )
    .await;
    assert!(
        verifier.seen().is_empty(),
        "row 3: no token, no vendor call"
    );
    assert_eq!(h.deliveries(), 0, "row 3");

    // Row 4: context, no token, `AcceptWithHoneypotOnly` — proceed.
    let verifier = Arc::new(ScriptedVerifier::passing());
    let h = with_challenge(&verifier, NoJsPolicy::AcceptWithHoneypotOnly);
    expect_both(&h, &h.fields(), None, "row 4").await;
    assert!(verifier.seen().is_empty(), "row 4");
    assert_eq!(h.deliveries(), 2, "row 4");

    // Row 5: passed — proceed.  Turnstile's field name.
    let verifier = Arc::new(ScriptedVerifier::passing());
    let h = with_challenge(&verifier, NoJsPolicy::Reject);
    expect_both(
        &h,
        &h.fields().set("cf-turnstile-response", "row-5-token"),
        None,
        "row 5",
    )
    .await;
    assert_eq!(verifier.seen(), ["row-5-token", "row-5-token"], "row 5");
    assert_eq!(h.deliveries(), 2, "row 5");

    // Row 6: not passed — challenge_failed, an `Args` error.  hCaptcha's name.
    let verifier = Arc::new(ScriptedVerifier::failing(&["invalid-input-response"]));
    let h = with_challenge(&verifier, NoJsPolicy::Reject);
    let fields = h.fields().set("h-captcha-response", "row-6-token");
    expect_both(
        &h,
        &fields,
        Some((
            "challenge_failed",
            "The security check did not pass. Please try again.",
        )),
        "row 6",
    )
    .await;
    assert!(!h.submit_fetch(&fields).await.is_server_error(), "row 6");
    assert!(verifier.seen().iter().all(|t| t == "row-6-token"), "row 6");
    assert_eq!(h.deliveries(), 0, "row 6");

    // Row 7: the verifier errors — challenge_unavailable, fail-closed, a
    // server error.  reCAPTCHA's field name.
    let verifier = Arc::new(ScriptedVerifier::unavailable());
    let h = with_challenge(&verifier, NoJsPolicy::Reject);
    let fields = h.fields().set("g-recaptcha-response", "row-7-token");
    expect_both(
        &h,
        &fields,
        Some((
            "challenge_unavailable",
            "The security check is unavailable right now. Please try again later.",
        )),
        "row 7",
    )
    .await;
    assert!(h.submit_fetch(&fields).await.is_server_error(), "row 7");
    assert!(verifier.seen().iter().all(|t| t == "row-7-token"), "row 7");
    assert_eq!(h.deliveries(), 0, "row 7");
}

/// FR-ABUSE-10, NFR-PRIV-02, RFC 011 D5 (P-40): the IP a site provides as
/// `ChallengeClientIp` reaches `verify_request`, in both request forms;
/// without it the verifier sees `None`.
#[tokio::test]
async fn the_client_ip_reaches_the_verifier() {
    let ip: std::net::IpAddr = "203.0.113.7".parse().unwrap();
    for client_ip in [Some(ip), None] {
        let verifier = Arc::new(ScriptedVerifier::passing());
        let h = Harness::new(Setup {
            challenge: Some(ChallengeContext {
                verifier: Arc::clone(&verifier) as _,
                policy: ChallengePolicy::default(),
            }),
            client_ip,
            ..Setup::default()
        });
        let fields = h.fields().set("cf-turnstile-response", "ip-token");
        expect_both(&h, &fields, None, "client IP").await;
        assert_eq!(verifier.seen_ips(), [client_ip, client_ip], "{client_ip:?}");
        assert_eq!(verifier.seen(), ["ip-token", "ip-token"], "{client_ip:?}");
    }
}
