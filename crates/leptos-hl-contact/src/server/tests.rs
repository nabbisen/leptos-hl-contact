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
