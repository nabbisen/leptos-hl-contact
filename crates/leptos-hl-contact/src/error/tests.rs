// tests.rs — unit tests for the parent module.

use super::*;
#[test]
fn field_errors_default_is_empty() {
    assert!(ContactFieldErrors::default().is_empty());
}

#[test]
fn field_errors_roundtrip_json() {
    let errs = ContactFieldErrors {
        name: Some(FieldError::Code(FieldErrorCode::Required)),
        email: Some(FieldError::Code(FieldErrorCode::Format)),
        subject: None,
        message: Some(FieldError::Text("too long".into())),
    };
    let json = errs.to_json();
    let back: ContactFieldErrors = serde_json::from_str(&json).unwrap();
    assert_eq!(back.name, Some(FieldError::Code(FieldErrorCode::Required)));
    assert_eq!(back.email, Some(FieldError::Code(FieldErrorCode::Format)));
    assert!(back.subject.is_none());
}

#[test]
fn from_error_str_parses_prefixed_payload() {
    let errs = ContactFieldErrors {
        name: Some(FieldError::Code(FieldErrorCode::Required)),
        ..Default::default()
    };
    let msg = errs.clone().into_server_fn_message();
    let parsed = ContactFieldErrors::from_error_str(&msg).unwrap();
    assert_eq!(
        parsed.name,
        Some(FieldError::Code(FieldErrorCode::Required))
    );
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
    let s = "error deserializing server function arguments: \
             field_errors:{\"name\":{\"kind\":\"required\"}}";
    let parsed = ContactFieldErrors::from_error_str(s).unwrap();
    assert_eq!(
        parsed.name,
        Some(FieldError::Code(FieldErrorCode::Required))
    );
}

#[test]
fn from_error_str_still_accepts_bare_sentinel() {
    let errs = ContactFieldErrors {
        email: Some(FieldError::Code(FieldErrorCode::Format)),
        ..Default::default()
    };
    let msg = errs.into_server_fn_message();
    assert!(msg.starts_with(FIELD_ERROR_PREFIX));
    let parsed = ContactFieldErrors::from_error_str(&msg).unwrap();
    assert_eq!(parsed.email, Some(FieldError::Code(FieldErrorCode::Format)));
    assert!(parsed.name.is_none());
}

#[test]
fn from_error_str_rejects_missing_sentinel() {
    assert!(ContactFieldErrors::from_error_str("{\"name\":{\"kind\":\"required\"}}").is_none());
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
        name: Some(FieldError::Code(FieldErrorCode::Length { min: 1, max: 80 })),
        message: Some(FieldError::Code(FieldErrorCode::Length {
            min: 1,
            max: 4000,
        })),
        ..Default::default()
    };
    let err: TestServerFnError =
        leptos::server_fn::error::ServerFnError::Args(errs.into_server_fn_message());

    let parsed = ContactFieldErrors::from_server_fn_error(&err).unwrap();
    assert_eq!(
        parsed.name,
        Some(FieldError::Code(FieldErrorCode::Length { min: 1, max: 80 }))
    );
    assert_eq!(
        parsed.message,
        Some(FieldError::Code(FieldErrorCode::Length {
            min: 1,
            max: 4000
        }))
    );
    assert!(parsed.email.is_none());
    assert!(parsed.subject.is_none());
}

/// The token-failure message is an `Args` variant carrying plain text; it
/// belongs on the generic-banner path, not beside a field.
#[test]
fn from_server_fn_error_ignores_args_without_payload() {
    let err: TestServerFnError = leptos::server_fn::error::ServerFnError::Args(
        ContactErrorCode::TokenInvalid.into_server_fn_message(),
    );
    assert!(ContactFieldErrors::from_server_fn_error(&err).is_none());
}

#[test]
fn from_server_fn_error_ignores_server_error_variant() {
    let err: TestServerFnError = leptos::server_fn::error::ServerFnError::ServerError(
        ContactErrorCode::DeliveryFailed.into_server_fn_message(),
    );
    assert!(ContactFieldErrors::from_server_fn_error(&err).is_none());
}

// ---------------------------------------------------------------------------
// Codes on the wire
// ---------------------------------------------------------------------------

/// Every variant must survive the round trip its JSON form implies; the
/// spelling is the wire contract with older and newer clients.
#[test]
fn field_error_code_serde_round_trip() {
    let cases = [
        (FieldErrorCode::Required, r#"{"kind":"required"}"#),
        (
            FieldErrorCode::Length { min: 1, max: 80 },
            r#"{"kind":"length","min":1,"max":80}"#,
        ),
        (FieldErrorCode::Format, r#"{"kind":"format"}"#),
        (FieldErrorCode::LineBreaks, r#"{"kind":"line_breaks"}"#),
    ];
    for (code, json) in cases {
        assert_eq!(serde_json::to_string(&code).unwrap(), json, "{code:?}");
        let back: FieldErrorCode = serde_json::from_str(json).unwrap();
        assert_eq!(back, code);
    }
}

/// `untagged` is what lets a current client read a 0.3 server's sentences.
#[test]
fn field_error_reads_both_text_and_code() {
    let text: FieldError = serde_json::from_str(r#""Name must be 1–80 characters""#).unwrap();
    assert_eq!(
        text,
        FieldError::Text("Name must be 1–80 characters".into())
    );

    let code: FieldError = serde_json::from_str(r#"{"kind":"length","min":1,"max":80}"#).unwrap();
    assert_eq!(
        code,
        FieldError::Code(FieldErrorCode::Length { min: 1, max: 80 })
    );
}

#[test]
fn contact_error_code_round_trips_through_the_wire_string() {
    for code in [
        ContactErrorCode::TokenInvalid,
        ContactErrorCode::NotConfigured,
        ContactErrorCode::DeliveryFailed,
        ContactErrorCode::Unexpected,
    ] {
        let msg = code.into_server_fn_message();
        assert!(msg.starts_with(CONTACT_ERROR_PREFIX));
        assert_eq!(ContactErrorCode::from_str_code(code.as_str()), Some(code));
    }
}

#[test]
fn contact_error_code_parses_from_args_and_server_error() {
    let args: TestServerFnError = leptos::server_fn::error::ServerFnError::Args(
        ContactErrorCode::TokenInvalid.into_server_fn_message(),
    );
    assert_eq!(
        ContactErrorCode::from_server_fn_error(&args),
        Some(ContactErrorCode::TokenInvalid)
    );

    let server: TestServerFnError = leptos::server_fn::error::ServerFnError::ServerError(
        ContactErrorCode::NotConfigured.into_server_fn_message(),
    );
    assert_eq!(
        ContactErrorCode::from_server_fn_error(&server),
        Some(ContactErrorCode::NotConfigured)
    );
}

/// The framework prefixes `Args` with its own English sentence, so the
/// parser must find the marker anywhere in the string.
#[test]
fn contact_error_code_parses_behind_framework_text() {
    let err: TestServerFnError = leptos::server_fn::error::ServerFnError::Args(
        "error deserializing server function arguments: contact_error:delivery_failed".into(),
    );
    assert_eq!(
        ContactErrorCode::from_server_fn_error(&err),
        Some(ContactErrorCode::DeliveryFailed)
    );
}

/// An unrecognised code — a newer server, say — yields `None` so the client
/// shows its generic fallback rather than nothing at all.
#[test]
fn contact_error_code_rejects_unknown_codes() {
    assert!(ContactErrorCode::from_str_code("teapot").is_none());

    let err: TestServerFnError =
        leptos::server_fn::error::ServerFnError::ServerError("contact_error:teapot".into());
    assert!(ContactErrorCode::from_server_fn_error(&err).is_none());

    let plain: TestServerFnError =
        leptos::server_fn::error::ServerFnError::ServerError("something else".into());
    assert!(ContactErrorCode::from_server_fn_error(&plain).is_none());
}

#[test]
fn field_errors_get_returns_the_right_field() {
    let errs = ContactFieldErrors {
        email: Some(FieldError::Code(FieldErrorCode::Format)),
        ..Default::default()
    };
    assert_eq!(
        errs.get(ContactField::Email),
        Some(&FieldError::Code(FieldErrorCode::Format))
    );
    assert!(errs.get(ContactField::Name).is_none());
}
