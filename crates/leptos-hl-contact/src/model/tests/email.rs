//! Email addresses a contact form can reply to (RFC 010 D2, FR-VAL-02).

use super::{code_for, valid_input};
use crate::error::{ContactField, FieldErrorCode};

fn with_email(email: &str) -> crate::model::ContactInput {
    let mut input = valid_input();
    input.email = email.into();
    input
}

/// No error on `email`.
fn accepted(email: &str) -> bool {
    with_email(email).validate_fields().email.is_none()
}

/// An address of exactly `len` characters: a 64-character local part and a
/// domain of 63-character labels, the longest each part may be.
fn address_of_length(len: usize) -> String {
    let local = "a".repeat(64);
    let tail = len - local.len() - 1 - (63 + 1 + 63 + 1) - ".com".len();
    format!(
        "{local}@{}.{}.{}.com",
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(tail)
    )
}

/// FR-VAL-02: ordinary, multi-label and internationalised domains are accepted.
#[test]
fn reply_to_addresses_are_accepted() {
    for email in ["ada@example.com", "ada@mail.example.co.jp", "ada@例え.jp"] {
        assert!(accepted(email), "{email}");
    }
}

/// FR-VAL-02: a single-label domain cannot be replied to from a public mailbox.
#[test]
fn a_single_label_domain_is_a_format_error() {
    assert_eq!(
        code_for(&with_email("abc@bar"), ContactField::Email),
        FieldErrorCode::Format
    );
}

/// FR-VAL-02: address literals are refused, IPv4 and IPv6.
#[test]
fn an_address_literal_is_a_format_error() {
    for email in ["a@[127.0.0.1]", "a@[2001:db8::1]"] {
        assert_eq!(
            code_for(&with_email(email), ContactField::Email),
            FieldErrorCode::Format,
            "{email}"
        );
    }
}

/// FR-VAL-02: an empty domain label is refused wherever it sits.
#[test]
fn an_empty_domain_label_is_a_format_error() {
    for email in ["a@example.", "a@.com", "a@example..com"] {
        assert_eq!(
            code_for(&with_email(email), ContactField::Email),
            FieldErrorCode::Format,
            "{email}"
        );
    }
}

/// FR-VAL-02, FR-VAL-07: 254 characters, the SMTP path limit and the form's
/// `maxlength`, is accepted.
#[test]
fn a_254_character_address_is_accepted() {
    let email = address_of_length(254);
    assert_eq!(email.chars().count(), 254);
    assert!(accepted(&email), "{email}");
}

/// FR-VAL-02, FR-VAL-07: one character more is a length error.
#[test]
fn a_255_character_address_is_a_length_error() {
    let email = address_of_length(255);
    assert_eq!(email.chars().count(), 255);
    assert_eq!(
        code_for(&with_email(&email), ContactField::Email),
        FieldErrorCode::Length { min: 0, max: 254 }
    );
}

/// RFC 010 D2: when the address is both too long and invalid, the length code
/// is reported.
#[test]
fn a_long_invalid_address_reports_its_length() {
    let email = format!("{}@bar", "a".repeat(296));
    assert_eq!(email.chars().count(), 300);
    assert_eq!(
        code_for(&with_email(&email), ContactField::Email),
        FieldErrorCode::Length { min: 0, max: 254 }
    );
}

/// FR-VAL-02: a blank email keeps the code it had before these rules.
#[test]
fn a_blank_email_keeps_its_format_code() {
    assert_eq!(
        code_for(&with_email(""), ContactField::Email),
        FieldErrorCode::Format
    );
}
