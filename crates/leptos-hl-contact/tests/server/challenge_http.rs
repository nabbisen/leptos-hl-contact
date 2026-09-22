//! What `HttpChallengeVerifier::with_verify_url` logs (RFC 017 handoff 02
//! review, C1): a warning when the override is not `https`, and never a
//! warning when it is.

use leptos_hl_contact::{ChallengeProvider, HttpChallengeVerifier};

use crate::support::capture_logs;

const WARNING: &str = "verify URL is not https: the secret will be sent in the clear";

/// An overridden `http://` URL warns once, at construction, naming neither
/// the URL nor the secret.
#[tokio::test]
async fn an_http_override_warns() {
    let (logs, _guard) = capture_logs();
    let secret = "zz-probe-challenge-secret";

    let _verifier = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, secret)
        .with_verify_url("http://127.0.0.1:1/siteverify");

    assert!(
        logs.any_contains(WARNING),
        "the warning is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
    for line in logs.lines() {
        assert!(!line.contains(secret), "the secret was logged: {line}");
        assert!(!line.contains("127.0.0.1"), "the URL was logged: {line}");
    }
}

/// An overridden `https://` URL logs no such warning.
#[tokio::test]
async fn an_https_override_does_not_warn() {
    let (logs, _guard) = capture_logs();

    let _verifier = HttpChallengeVerifier::new(ChallengeProvider::Turnstile, "s")
        .with_verify_url("https://proxy.example.test/siteverify");

    assert!(!logs.any_contains(WARNING), "{:?}", logs.lines());
}
