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
