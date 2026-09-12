// tests.rs — unit tests for the parent module.

use super::*;
#[test]
fn field_errors_default_is_empty() {
    assert!(ContactFieldErrors::default().is_empty());
}

#[test]
fn field_errors_roundtrip_json() {
    let errs = ContactFieldErrors {
        name: Some("required".into()),
        email: Some("invalid email".into()),
        subject: None,
        message: Some("too long".into()),
    };
    let json = errs.to_json();
    let back: ContactFieldErrors = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name.as_deref(), Some("required"));
    assert_eq!(back.email.as_deref(), Some("invalid email"));
    assert!(back.subject.is_none());
}

#[test]
fn from_error_str_parses_prefixed_payload() {
    let errs = ContactFieldErrors {
        name: Some("required".into()),
        ..Default::default()
    };
    let msg = errs.clone().into_server_fn_message();
    let parsed = ContactFieldErrors::from_error_str(&msg).unwrap();
    assert_eq!(parsed.name.as_deref(), Some("required"));
}

#[test]
fn from_error_str_returns_none_for_plain_string() {
    assert!(ContactFieldErrors::from_error_str("generic error").is_none());
}

/// `ServerFnError::Args` displays as
/// `"error deserializing server function arguments: {payload}"`, so the
/// sentinel is never at the start of the string a client sees.
#[test]
fn from_error_str_accepts_framework_display_prefix() {
    let s = "error deserializing server function arguments: field_errors:{\"name\":\"required\"}";
    let parsed = ContactFieldErrors::from_error_str(s).unwrap();
    assert_eq!(parsed.name.as_deref(), Some("required"));
}

#[test]
fn from_error_str_still_accepts_bare_sentinel() {
    let errs = ContactFieldErrors {
        email: Some("A valid email address is required".into()),
        ..Default::default()
    };
    let msg = errs.into_server_fn_message();
    assert!(msg.starts_with(FIELD_ERROR_PREFIX));
    let parsed = ContactFieldErrors::from_error_str(&msg).unwrap();
    assert_eq!(
        parsed.email.as_deref(),
        Some("A valid email address is required")
    );
    assert!(parsed.name.is_none());
}

#[test]
fn from_error_str_rejects_missing_sentinel() {
    assert!(ContactFieldErrors::from_error_str("{\"name\":\"required\"}").is_none());
}

#[test]
fn from_error_str_rejects_bad_json_after_sentinel() {
    assert!(ContactFieldErrors::from_error_str("field_errors:{not json").is_none());
}

// ---------------------------------------------------------------------------
// from_server_fn_error
// ---------------------------------------------------------------------------

/// The error type of the `SubmitContact` server action, i.e. `ServerFnError`
/// with its default type parameter.  `NoCustomError` is not named explicitly:
/// `server_fn` 0.8.13 deprecates it ahead of removal in 0.9, and relying on
/// the default keeps `-D warnings` clean without an `#[allow]`.
type TestServerFnError = leptos::server_fn::error::ServerFnError;

#[test]
fn from_server_fn_error_parses_args_variant() {
    let errs = ContactFieldErrors {
        name: Some("Name must be 1–80 characters".into()),
        message: Some("Message must be 1–4 000 characters".into()),
        ..Default::default()
    };
    let err: TestServerFnError =
        leptos::server_fn::error::ServerFnError::Args(errs.into_server_fn_message());

    let parsed = ContactFieldErrors::from_server_fn_error(&err).unwrap();
    assert_eq!(parsed.name.as_deref(), Some("Name must be 1–80 characters"));
    assert_eq!(
        parsed.message.as_deref(),
        Some("Message must be 1–4 000 characters")
    );
    assert!(parsed.email.is_none());
    assert!(parsed.subject.is_none());
}

/// The token-failure message is an `Args` variant carrying plain text; it
/// belongs on the generic-banner path, not beside a field.
#[test]
fn from_server_fn_error_ignores_args_without_payload() {
    let err: TestServerFnError = leptos::server_fn::error::ServerFnError::Args(
        "Invalid or expired security token. Please reload the page.".into(),
    );
    assert!(ContactFieldErrors::from_server_fn_error(&err).is_none());
}

#[test]
fn from_server_fn_error_ignores_server_error_variant() {
    let err: TestServerFnError = leptos::server_fn::error::ServerFnError::ServerError(
        "Failed to send message. Please try again later.".into(),
    );
    assert!(ContactFieldErrors::from_server_fn_error(&err).is_none());
}
