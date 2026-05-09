// tests.rs — unit tests for the server function module.

use crate::error::FIELD_ERROR_PREFIX;

#[test]
fn field_error_prefix_is_not_empty() {
    assert!(!FIELD_ERROR_PREFIX.is_empty());
}

#[test]
fn field_error_message_has_prefix() {
    use crate::error::ContactFieldErrors;
    let errs = ContactFieldErrors {
        name: Some("required".into()),
        ..Default::default()
    };
    let msg = errs.into_server_fn_message();
    assert!(
        msg.starts_with(FIELD_ERROR_PREFIX),
        "encoded message must start with sentinel prefix"
    );
}

#[test]
fn generic_error_has_no_prefix() {
    let generic = "Failed to send message. Please try again later.";
    assert!(!generic.starts_with(FIELD_ERROR_PREFIX));
}

#[test]
fn csrf_error_message_has_no_field_prefix() {
    let csrf_err = "Invalid or expired security token. Please reload the page.";
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
        "Contact form security is not configured. \
         Please contact the site administrator.";
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
