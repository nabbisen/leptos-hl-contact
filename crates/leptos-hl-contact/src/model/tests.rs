// tests.rs — unit tests for the parent module.

use super::*;

fn valid_input() -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        Some("Hello".into()),
        "This is my message.".into(),
        String::new(),
    )
}

#[test]
fn valid_input_passes_validation() {
    let input = valid_input();
    assert!(input.validate_input().is_ok());
}

#[test]
fn empty_name_fails() {
    let mut input = valid_input();
    input.name = String::new();
    assert!(input.validate_input().is_err());
}

#[test]
fn invalid_email_fails() {
    let mut input = valid_input();
    input.email = "not-an-email".into();
    assert!(input.validate_input().is_err());
}

#[test]
fn too_long_message_fails() {
    let mut input = valid_input();
    input.message = "x".repeat(MESSAGE_MAX_LEN + 1);
    assert!(input.validate_input().is_err());
}

/// The constant drives the `#[validate]` attribute, so the boundary must sit
/// exactly at `MESSAGE_MAX_LEN` rather than at a literal that could drift.
#[test]
fn message_ceiling_constant_is_enforced_by_validator() {
    let mut input = valid_input();

    input.message = "x".repeat(MESSAGE_MAX_LEN);
    assert!(
        input.validate_input().is_ok(),
        "the ceiling itself must pass"
    );

    input.message = "x".repeat(MESSAGE_MAX_LEN + 1);
    assert!(input.validate_input().is_err(), "one over must fail");
}

/// `validator`'s `length` counts characters, so a message of multibyte
/// characters at the ceiling passes even though it is several times that many
/// bytes.
#[test]
fn message_length_counts_characters() {
    let mut input = valid_input();
    input.message = "あ".repeat(MESSAGE_MAX_LEN);
    assert!(
        input.message.len() > MESSAGE_MAX_LEN,
        "precondition: multibyte"
    );
    assert!(input.validate_input().is_ok());
}

#[test]
fn newline_in_name_fails() {
    let mut input = valid_input();
    input.name = "Alice\nEvil".into();
    assert!(input.validate_input().is_err());
}

#[test]
fn newline_in_subject_fails() {
    let mut input = valid_input();
    input.subject = Some("Subject\nInjected".into());
    assert!(input.validate_input().is_err());
}

#[test]
fn honeypot_input_is_detected() {
    let mut input = valid_input();
    input.website = "http://bot.example.com".into();
    assert!(input.check_honeypot().is_err());
}

#[test]
fn subject_fallback_works() {
    let mut input = valid_input();
    input.subject = None;
    assert_eq!(input.effective_subject("No subject"), "No subject");
}

#[test]
fn empty_subject_uses_fallback() {
    let mut input = valid_input();
    input.subject = Some("  ".into());
    // from_raw trims and filters blank subjects
    let input2 = ContactInput::from_raw(
        input.name.clone(),
        input.email.clone(),
        Some("  ".into()),
        input.message.clone(),
        String::new(),
    );
    assert_eq!(input2.effective_subject("Fallback"), "Fallback");
}

// ---------------------------------------------------------------------------
// validate_fields produces codes, not sentences
// ---------------------------------------------------------------------------

use crate::error::{ContactField, FieldError, FieldErrorCode};

fn code_for(input: &ContactInput, field: ContactField) -> FieldErrorCode {
    match input.validate_fields().get(field) {
        Some(FieldError::Code(c)) => c.clone(),
        other => panic!("expected a code for {field:?}, got {other:?}"),
    }
}

/// `validator` reports an empty required string as `length` with `min: 1`,
/// not as a `required` code — so the client renders the length text.  The
/// server emits `Required` only from the policy.
#[test]
fn empty_name_yields_length_code() {
    let mut input = valid_input();
    input.name = String::new();
    assert_eq!(
        code_for(&input, ContactField::Name),
        FieldErrorCode::Length { min: 1, max: 80 }
    );
}

#[test]
fn over_long_name_yields_the_same_length_code() {
    let mut input = valid_input();
    input.name = "x".repeat(81);
    assert_eq!(
        code_for(&input, ContactField::Name),
        FieldErrorCode::Length { min: 1, max: 80 }
    );
}

#[test]
fn bad_email_yields_format_code() {
    let mut input = valid_input();
    input.email = "not-an-email".into();
    assert_eq!(
        code_for(&input, ContactField::Email),
        FieldErrorCode::Format
    );
}

#[test]
fn newline_in_name_yields_line_breaks_code() {
    let mut input = valid_input();
    input.name = "Alice\nEvil".into();
    assert_eq!(
        code_for(&input, ContactField::Name),
        FieldErrorCode::LineBreaks
    );
}

#[test]
fn over_long_subject_yields_length_code_with_zero_min() {
    let mut input = valid_input();
    input.subject = Some("x".repeat(121));
    assert_eq!(
        code_for(&input, ContactField::Subject),
        FieldErrorCode::Length { min: 0, max: 120 }
    );
}

#[test]
fn over_long_message_yields_length_code_at_the_ceiling() {
    let mut input = valid_input();
    input.message = "x".repeat(MESSAGE_MAX_LEN + 1);
    assert_eq!(
        code_for(&input, ContactField::Message),
        FieldErrorCode::Length {
            min: 1,
            max: MESSAGE_MAX_LEN
        }
    );
}
