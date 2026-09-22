// tests.rs — unit tests for the parent module.
//
// Moved from `delivery/smtp/tests.rs` (RFC 017 D1), unchanged: the four
// tests that call `build_plain_text_body` directly, and the fixtures they
// need.  `delivery/smtp/tests.rs` keeps the tests that build a whole message.

use super::*;
use crate::model::SiteFieldValue;

fn sample_input() -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        Some("Hello".into()),
        "This is a test message.".into(),
        String::new(),
    )
}

#[test]
fn body_includes_expected_fields() {
    let input = sample_input();
    let body = build_plain_text_body(&input);
    assert!(body.contains("Alice"));
    assert!(body.contains("alice@example.com"));
    assert!(body.contains("Hello"));
    assert!(body.contains("This is a test message."));
}

/// What the body was before site fields existed, byte for byte.
const BODY_0_7: &str = "New contact form submission\n\
===========================\n\
\n\
Name:\n\
Alice\n\
\n\
Email:\n\
alice@example.com\n\
\n\
Subject:\n\
Hello\n\
\n\
Message:\n\
This is a test message.\n";

fn site_value(key: &str, label: &str, value: &str, value_label: Option<&str>) -> SiteFieldValue {
    SiteFieldValue {
        key: key.into(),
        label: label.into(),
        value: value.into(),
        value_label: value_label.map(Into::into),
    }
}

/// RFC 015 D4: with no site fields the body is exactly 0.7's, whole string.
#[test]
fn a_body_without_site_fields_is_byte_identical_to_0_7() {
    assert_eq!(build_plain_text_body(&sample_input()), BODY_0_7);
}

/// FR-FIELD-06, RFC 015 D4: one block per value, in the given order, after
/// `Subject:` and before `Message:`, in the body's own style.  A choice reads
/// `choice label (choice key)`, and the other kinds read as their value.
#[test]
fn site_fields_have_one_block_each_between_the_subject_and_the_message() {
    let mut input = sample_input();
    input.site_fields = vec![
        site_value("organisation", "Organisation", "Example Ltd", None),
        site_value("topic", "Topic", "sales", Some("Sales")),
    ];

    assert_eq!(
        build_plain_text_body(&input),
        "New contact form submission\n\
===========================\n\
\n\
Name:\n\
Alice\n\
\n\
Email:\n\
alice@example.com\n\
\n\
Subject:\n\
Hello\n\
\n\
Organisation:\n\
Example Ltd\n\
\n\
Topic:\n\
Sales (sales)\n\
\n\
Message:\n\
This is a test message.\n"
    );
}

/// RFC 015 D4: a value of several lines is kept as it was typed, line breaks
/// included.
#[test]
fn a_multi_line_text_value_is_kept_intact() {
    let mut input = sample_input();
    input.site_fields = vec![site_value(
        "timing",
        "Timing",
        "First line\n\nThird line, after a blank one",
        None,
    )];

    let body = build_plain_text_body(&input);
    assert!(
        body.contains("Timing:\nFirst line\n\nThird line, after a blank one\n\nMessage:\n"),
        "{body}"
    );
}
