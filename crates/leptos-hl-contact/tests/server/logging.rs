//! Log hygiene.

use std::sync::Arc;

use leptos_hl_contact::{ChallengeContext, ChallengePolicy, FilterDecision};

use crate::support::{
    DELIVERY_ERROR_DETAIL, Fields, FixedFilter, Harness, NeverDelivery, ScriptedVerifier, Setup,
    TEST_SECRET, TokenMode, capture_logs, nonce_of,
};

const NAME: &str = "Zelda Quimby-Marker";
const EMAIL: &str = "zelda.marker@example.test";
const SUBJECT: &str = "SUBJECT-MARKER-9d2";
const MESSAGE: &str = "MESSAGE-MARKER-7c1 with a line of text";
const CHALLENGE_TOKEN: &str = "CHALLENGE-TOKEN-MARKER";
const CLIENT_IP: &str = "198.51.100.73";

fn personal(fields: Fields) -> Fields {
    fields
        .set("name", NAME)
        .set("email", EMAIL)
        .set("subject", SUBJECT)
        .set("message", MESSAGE)
}

/// T9, FR-OBS-02, NFR-PRIV-01: across a representative set of outcomes — a
/// delivery, the honeypot, a validation failure, a token failure, a binding
/// failure, a failed challenge with the visitor's IP in context, a filter's
/// `Reject` and `SilentDrop`, a delivery error, a delivery timeout, and missing
/// configuration — no log event or span field contains the visitor's name,
/// email, subject or message, the form token, the binding cookie's value, the
/// challenge token, the visitor's IP, or the form-token secret.
///
/// The capture is checked first: each outcome's own event must be present,
/// so the test cannot pass by capturing nothing.
// The clock is paused so the delivery timeout below passes without waiting.
#[tokio::test(start_paused = true)]
async fn no_personal_data_or_secret_is_logged() {
    let (logs, _guard) = capture_logs();
    let mut forbidden = vec![
        NAME.to_owned(),
        EMAIL.to_owned(),
        SUBJECT.to_owned(),
        "MESSAGE-MARKER".to_owned(),
        CHALLENGE_TOKEN.to_owned(),
        CLIENT_IP.to_owned(),
        TEST_SECRET.to_owned(),
    ];

    // Delivered, honeypot, validation failure, token failure.
    let h = Harness::new(Setup::default());
    let token = h.token();
    forbidden.push(token.clone());
    h.submit_fetch(&personal(Fields::valid(&token))).await;
    h.submit_fetch(&personal(Fields::valid(&token)).set("website", "http://bot.example"))
        .await;
    h.submit_fetch(&personal(Fields::valid(&token)).set("email", "not-an-email"))
        .await;
    h.submit_nojs(&personal(Fields::valid(&token)).set("form_token", "tampered-token-value"))
        .await;
    forbidden.push("tampered-token-value".to_owned());

    // Binding failure: the rendered cookie, sent with the wrong nonce.
    let bound = Harness::new(Setup {
        token: TokenMode::Bound,
        ..Setup::default()
    });
    let page = bound.render("/contact", None).await;
    let bound_token = page.token().expect("token");
    forbidden.push(bound_token.clone());
    forbidden.push(nonce_of(&bound_token).to_owned());
    bound
        .submit_fetch(&personal(Fields::valid(&bound_token)))
        .await;

    // A failed challenge, with the visitor's IP (RFC 011 D5).
    let challenged = Harness::new(Setup {
        challenge: Some(ChallengeContext {
            verifier: Arc::new(ScriptedVerifier::failing(&["invalid-input-response"])),
            policy: ChallengePolicy::default(),
        }),
        client_ip: Some(CLIENT_IP.parse().unwrap()),
        ..Setup::default()
    });
    challenged
        .submit_fetch(&personal(challenged.fields()).set("cf-turnstile-response", CHALLENGE_TOKEN))
        .await;

    // A filter's Reject and SilentDrop.
    for decision in [FilterDecision::Reject, FilterDecision::SilentDrop] {
        let filtered = Harness::new(Setup {
            filter: Some(Arc::new(FixedFilter::new(decision, "Marker"))),
            ..Setup::default()
        });
        filtered.submit_fetch(&personal(filtered.fields())).await;
    }

    // A delivery error: logged with its transport detail (FR-OBS-03).
    let failing = Harness::new(Setup {
        failing_delivery: true,
        ..Setup::default()
    });
    failing.submit_fetch(&personal(failing.fields())).await;

    // A delivery timeout: logged with the limit only.
    let slow = Harness::new(Setup {
        delivery_context: Some(Arc::new(leptos_hl_contact::DeliveryTimeout::new(
            NeverDelivery,
            std::time::Duration::from_millis(50),
        ))),
        ..Setup::default()
    });
    slow.submit_fetch(&personal(slow.fields())).await;

    // Missing configuration.
    let unconfigured = Harness::new(Setup {
        delivery: false,
        ..Setup::default()
    });
    unconfigured
        .submit_fetch(&personal(unconfigured.fields()))
        .await;

    for expected in [
        "form token rejected",
        "challenge failed",
        "submission rejected by filter",
        "submission silently dropped by filter",
        "ContactDeliveryContext not provided",
        "contact form delivery failed",
        "contact form delivery timed out",
    ] {
        assert!(
            logs.any_contains(expected),
            "the capture did not see `{expected}`; it saw:\n{}",
            logs.lines().join("\n")
        );
    }

    // FR-OBS-03: operators get the transport detail.
    assert!(
        logs.any_contains(DELIVERY_ERROR_DETAIL),
        "the delivery failure's transport detail is logged"
    );

    for line in logs.lines() {
        for needle in &forbidden {
            assert!(
                !line.contains(needle.as_str()),
                "`{needle}` logged in: {line}"
            );
        }
    }
}
