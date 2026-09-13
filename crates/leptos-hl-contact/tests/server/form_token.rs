//! The form token, without binding.

use crate::support::{Fields, Harness, Setup, TokenMode, signed_token};

/// FR-ABUSE-04, T3: with the feature on and no `FormTokenContext`, every
/// submission fails closed as `not_configured`, in both request forms.
#[tokio::test]
async fn a_missing_token_config_fails_closed() {
    let h = Harness::new(Setup {
        token: TokenMode::Absent,
        ..Setup::default()
    });

    let fetch = h.submit_fetch(&h.fields()).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("not_configured"));

    let page = h.follow(&h.submit_nojs(&h.fields()).await).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("This form is not available right now.")
    );
    assert_eq!(h.deliveries(), 0);
}

/// FR-VAL-06, FR-ABUSE-03: a missing, malformed or expired token is
/// `token_invalid`.  The expired token is signed with the test secret in the
/// documented format; a fresh token from the same signer is accepted first,
/// so the expiry case cannot pass on a bad signature.
#[tokio::test]
async fn a_missing_malformed_or_expired_token_is_rejected() {
    let h = Harness::new(Setup::default());
    let nonce = "00112233445566778899aabbccddeeff";

    let fresh = h
        .submit_fetch(&Fields::valid(&signed_token(0, nonce)))
        .await;
    assert!(
        fresh.status.is_success(),
        "the test signer is valid: {}",
        fresh.body
    );
    assert_eq!(h.deliveries(), 1);

    let cases = [
        ("missing", h.fields().without("form_token")),
        ("malformed", h.fields().set("form_token", "not-a-token")),
        ("expired", Fields::valid(&signed_token(3600 + 60, nonce))),
    ];
    for (case, fields) in cases {
        let fetch = h.submit_fetch(&fields).await;
        assert_eq!(
            fetch.contact_error().as_deref(),
            Some("token_invalid"),
            "{case}, fetch"
        );

        let page = h.follow(&h.submit_nojs(&fields).await).await;
        assert_eq!(
            page.banner().as_deref(),
            Some("Your session token expired. Please reload the page and try again."),
            "{case}, no-JS"
        );
    }
    assert_eq!(h.deliveries(), 1);
}

/// FR-ABUSE-13: a token younger than the minimum age is `too_fast`, which is
/// retryable, not `token_invalid`.
#[tokio::test]
async fn a_too_young_token_is_retryable() {
    let h = Harness::new(Setup {
        min_age_secs: 3600,
        ..Setup::default()
    });

    let fetch = h.submit_fetch(&h.fields()).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("too_fast"));

    let page = h.follow(&h.submit_nojs(&h.fields()).await).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("Please wait a moment and try again.")
    );
    assert_eq!(h.deliveries(), 0);
}

/// FR-VAL-06, NFR-COMPAT-04: the 0.4 field name was removed in 0.6.0.  A
/// valid token posted only as `csrf_token` no longer counts: the submission
/// has no `form_token` and is refused as `token_invalid`, in both request
/// forms, and nothing is delivered.
#[tokio::test]
async fn a_0_4_csrf_token_field_is_no_longer_accepted() {
    let h = Harness::new(Setup::default());
    let token = h.token();
    let fields = h.fields().without("form_token").set("csrf_token", &token);

    let fetch = h.submit_fetch(&fields).await;
    assert_eq!(
        fetch.contact_error().as_deref(),
        Some("token_invalid"),
        "{} {}",
        fetch.status,
        fetch.body
    );

    let nojs = h.submit_nojs(&fields).await;
    assert!(
        nojs.is_nojs_error(),
        "{} {:?}",
        nojs.status,
        nojs.location()
    );
    assert_eq!(
        h.follow(&nojs).await.banner().as_deref(),
        Some("Your session token expired. Please reload the page and try again.")
    );
    assert_eq!(h.deliveries(), 0);
}
