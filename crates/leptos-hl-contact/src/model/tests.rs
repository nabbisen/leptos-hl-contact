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
