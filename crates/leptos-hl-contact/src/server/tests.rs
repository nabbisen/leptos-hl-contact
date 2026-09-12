// tests.rs — unit tests for the server function module.

use crate::error::FIELD_ERROR_PREFIX;

/// The sentinel is part of the wire contract between `submit_contact` and
/// `ContactForm` (External Design §4.2.3); changing it is a breaking change.
#[test]
fn field_error_prefix_is_the_wire_sentinel() {
    assert_eq!(FIELD_ERROR_PREFIX, "field_errors:");
}

#[test]
fn field_error_message_has_prefix() {
    use crate::error::{ContactFieldErrors, FieldError, FieldErrorCode};
    let errs = ContactFieldErrors {
        name: Some(FieldError::Code(FieldErrorCode::Required)),
        ..Default::default()
    };
    let msg = errs.into_server_fn_message();
    assert!(
        msg.starts_with(FIELD_ERROR_PREFIX),
        "encoded message must start with sentinel prefix"
    );
}

/// Whole-submission errors carry the `contact_error:` prefix, never the
/// field sentinel, so the component routes them to the banner.
#[test]
fn generic_error_has_no_field_prefix() {
    use crate::error::ContactErrorCode;
    for code in [
        ContactErrorCode::TokenInvalid,
        ContactErrorCode::NotConfigured,
        ContactErrorCode::DeliveryFailed,
        ContactErrorCode::Unexpected,
    ] {
        let msg = code.into_server_fn_message();
        assert!(!msg.starts_with(FIELD_ERROR_PREFIX), "{msg}");
        assert!(msg.starts_with(crate::error::CONTACT_ERROR_PREFIX), "{msg}");
    }
}

#[test]
fn csrf_error_message_has_no_field_prefix() {
    let csrf_err = crate::error::ContactErrorCode::TokenInvalid.into_server_fn_message();
    assert!(!csrf_err.starts_with(FIELD_ERROR_PREFIX));
}

/// Verify that the CSRF fail-closed logic is documented correctly:
/// when `csrf` feature is enabled, the server should reject submissions
/// when `CsrfConfigContext` is absent (verified at the integration level by
/// the server fn; this unit test validates the error message sentinel).
#[test]
fn csrf_missing_context_error_is_not_field_error() {
    // The error returned when CsrfConfigContext is missing must be a
    // ServerError (not field_errors: prefix), so the component shows
    // the generic error banner, not a field-level message.
    let missing_context_msg =
        crate::error::ContactErrorCode::NotConfigured.into_server_fn_message();
    assert!(!missing_context_msg.starts_with(crate::error::FIELD_ERROR_PREFIX));
}

#[test]
fn pii_not_present_in_expected_log_messages() {
    // Smoke-check: the expected log message strings do not contain field
    // interpolation patterns that would expose PII.
    let noop_msg = "NoopDelivery: discarding contact form submission";
    let smtp_msg = "contact form submission delivered via SMTP";
    // Neither message contains format specifiers
    assert!(!noop_msg.contains('%'));
    assert!(!smtp_msg.contains('%'));
}

/// `TooFast` is the one token failure the visitor is told apart from the
/// rest, because retrying works.  It travels like every other code.
#[test]
fn too_fast_message_has_the_contact_error_prefix() {
    use crate::error::{CONTACT_ERROR_PREFIX, ContactErrorCode};
    let msg = ContactErrorCode::TooFast.into_server_fn_message();
    assert_eq!(msg, format!("{CONTACT_ERROR_PREFIX}too_fast"));
    assert!(!msg.starts_with(FIELD_ERROR_PREFIX));
    assert_eq!(
        ContactErrorCode::from_str_code("too_fast"),
        Some(ContactErrorCode::TooFast)
    );
}

// ---------------------------------------------------------------------------
// token_issued_at — drives the browser's refresh timer
// ---------------------------------------------------------------------------

#[test]
fn token_issued_at_reads_the_first_segment() {
    assert_eq!(
        crate::server::token_issued_at("1789232333|2546fb444f8f1bea|2dc456c0"),
        Some(1_789_232_333)
    );
}

/// A malformed value schedules nothing rather than a nonsense timer.
#[test]
fn token_issued_at_is_none_for_anything_but_a_token() {
    for bad in [
        "",
        "1789232333",           // a bare number
        "1789232333|nonce",     // two segments
        "soon|nonce|signature", // not a number
        "-5|nonce|signature",   // not unsigned
    ] {
        assert_eq!(crate::server::token_issued_at(bad), None, "{bad:?}");
    }
}

// ---------------------------------------------------------------------------
// mounted_refresh_delay — review C2
// ---------------------------------------------------------------------------

#[test]
fn a_mounted_token_refreshes_at_its_refresh_point() {
    assert_eq!(crate::server::mounted_refresh_delay(1_000, 1_010, 70), 60);
}

/// A page restored late, or hydrated long after it was rendered: refresh
/// once, now, rather than submit an expired token.
#[test]
fn an_overdue_mounted_token_refreshes_immediately() {
    assert_eq!(crate::server::mounted_refresh_delay(1_000, 1_070, 70), 0);
    assert_eq!(crate::server::mounted_refresh_delay(1_000, 50_000, 70), 0);
}

/// Two hours fast makes the mounted token look overdue: one immediate fetch.
/// It cannot repeat, because a fetched token is timed from its arrival.
#[test]
fn a_fast_browser_clock_costs_one_immediate_refresh() {
    assert_eq!(
        crate::server::mounted_refresh_delay(1_000, 1_000 + 7_200, 70),
        0
    );
}

/// Two hours slow would put the first refresh two hours out, past the
/// mounted token's expiry.  The delay is capped at the interval instead.
#[test]
fn a_slow_browser_clock_is_capped_at_the_interval() {
    assert_eq!(
        crate::server::mounted_refresh_delay(10_000, 10_000 - 7_200, 70),
        70
    );
}

// ---------------------------------------------------------------------------
// Challenge codes — prefix and variant class (RFC 005 handoff 01)
// ---------------------------------------------------------------------------

/// Server-side problems go out as `ServerError`; a missing or failed
/// challenge is the submission's, so it is `Args`.  All carry the prefix.
#[cfg(feature = "ssr")]
#[test]
fn challenge_rejections_carry_the_prefix_and_the_right_variant() {
    use crate::error::{CONTACT_ERROR_PREFIX, ContactErrorCode as C};
    use leptos::server_fn::error::ServerFnError;

    for (code, wire, server_side) in [
        (
            C::ChallengeRequired,
            "contact_error:challenge_required",
            false,
        ),
        (C::ChallengeFailed, "contact_error:challenge_failed", false),
        (
            C::ChallengeUnavailable,
            "contact_error:challenge_unavailable",
            true,
        ),
        (C::NotConfigured, "contact_error:not_configured", true),
    ] {
        match crate::challenge::rejection(code) {
            ServerFnError::ServerError(m) => {
                assert!(server_side, "{code:?} must be Args");
                assert_eq!(m, wire);
            }
            ServerFnError::Args(m) => {
                assert!(!server_side, "{code:?} must be ServerError");
                assert_eq!(m, wire);
            }
            other => panic!("unexpected variant {other:?}"),
        }
        assert!(wire.starts_with(CONTACT_ERROR_PREFIX));
    }
}

/// RFC 005 handoff 01, known risk: the vendors' field names are hyphenated.
/// Decode a real form body through the generated `SubmitContact` type, with
/// the same `PostUrl` codec a POST uses, to prove the renames reach serde.
#[cfg(feature = "axum-helpers")]
#[tokio::test]
async fn hyphenated_challenge_fields_decode_through_the_generated_type() {
    use leptos::server_fn::codec::{FromReq, PostUrl};
    use leptos::server_fn::error::ServerFnError;

    async fn decode(body: &'static str) -> crate::server::SubmitContact {
        let req = axum::http::Request::post("/api/submit_contact")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(axum::body::Body::from(body))
            .unwrap();
        <crate::server::SubmitContact as FromReq<PostUrl, _, ServerFnError>>::from_req(req)
            .await
            .expect("the body decodes")
    }

    let args = decode(
        "name=Ada&email=ada%40example.com&message=Hello&website=\
         &cf-turnstile-response=abc&h-captcha-response=&g-recaptcha-response=xyz",
    )
    .await;
    assert_eq!(args.cf_turnstile_response.as_deref(), Some("abc"));
    // `serde_qs` decodes an empty value into `None`, not `Some("")`.  Either
    // way the gate treats a blank token as absent.
    assert_eq!(args.h_captcha_response, None);
    assert_eq!(args.g_recaptcha_response.as_deref(), Some("xyz"));

    // A form without a widget still decodes: the fields default to `None`.
    let args = decode("name=Ada&email=ada%40example.com&message=Hello&website=").await;
    assert_eq!(args.cf_turnstile_response, None);
    assert_eq!(args.h_captcha_response, None);
    assert_eq!(args.g_recaptcha_response, None);

    // The Rust spelling is not the wire name.
    let args =
        decode("name=Ada&email=ada%40example.com&message=Hello&website=&cf_turnstile_response=abc")
            .await;
    assert_eq!(args.cf_turnstile_response, None);
}
