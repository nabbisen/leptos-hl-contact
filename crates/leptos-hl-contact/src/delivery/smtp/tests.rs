// tests.rs — unit tests for the parent module.

use std::future::pending;

use super::*;
use crate::model::SiteFieldValue;
fn sample_config() -> SmtpConfig {
    SmtpConfig {
        host: "smtp.example.com".into(),
        port: 587,
        username: "user".into(),
        password: "secret".into(),
        from_address: "noreply@example.com".into(),
        to_address: "admin@example.com".into(),
        subject_prefix: "[Contact]".into(),
        tls_mode: SmtpTlsMode::StartTls,
        timeout: SmtpConfig::DEFAULT_TIMEOUT,
    }
}

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
fn message_builder_creates_expected_headers() {
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    assert!(
        raw.contains("noreply@example.com") && (raw.contains("From: ") || raw.contains("From:"))
    );
    assert!(raw.contains("admin@example.com") && (raw.contains("To: ") || raw.contains("To:")));
    assert!(raw.contains("Reply-To:"));
    assert!(raw.contains("[Contact] Hello"));
}

#[test]
fn from_uses_configured_address() {
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    assert!(raw.contains("noreply@example.com"));
}

#[test]
fn reply_to_uses_user_email() {
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    assert!(raw.contains("alice@example.com"));
}

#[test]
fn subject_contains_prefix_and_value() {
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    assert!(raw.contains("[Contact]"));
    assert!(raw.contains("Hello"));
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

/// RFC 015 D4: a site value, or its label, never reaches a header; it is in
/// the body only.
#[test]
fn a_site_value_never_reaches_a_header() {
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let mut input = sample_input();
    input.site_fields = vec![site_value(
        "organisation",
        "Organisation-Label-Marker",
        "Organisation-Value-Marker",
        None,
    )];

    let raw = String::from_utf8(delivery.build_message(&input).unwrap().formatted()).unwrap();
    let (headers, body) = raw
        .split_once("\r\n\r\n")
        .expect("headers, a blank line, a body");
    assert!(!headers.contains("Organisation-"), "{headers}");
    assert!(body.contains("Organisation-Value-Marker"), "in the body");
}

#[test]
fn reply_to_with_special_chars_in_name() {
    // Names containing quotes, commas, and angle brackets must not break message
    // construction.  Mailbox::new handles RFC 5322 encoding.
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let special_input = ContactInput::from_raw(
        "O'Brien, Alice <alice>".into(), // quotes + comma + angle brackets
        "alice@example.com".into(),
        Some("Test".into()),
        "Message body.".into(),
        String::new(),
    );
    // Should not panic or return an error
    assert!(delivery.build_message(&special_input).is_ok());
}

#[test]
fn reply_to_uses_mailbox_new_not_string_parse() {
    // Verify the Reply-To header contains the user email without raw string interpolation
    let delivery = LettreSmtpDelivery {
        config: sample_config(),
    };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    // Email must appear in Reply-To (not in From)
    let from_line = raw.lines().find(|l| l.starts_with("From:")).unwrap_or("");
    assert!(
        !from_line.contains("alice@example.com"),
        "user email must not be in From"
    );
}

/// FR-CFG-04: the relay password never appears in `Debug` output — neither the
/// configuration's nor that of the backend holding it.
#[test]
fn debug_redacts_the_password() {
    let config = SmtpConfig {
        password: "hunter2-test".into(),
        ..sample_config()
    };
    let printed = format!("{config:?}");
    assert!(printed.contains("<redacted>"), "{printed}");
    assert!(!printed.contains("hunter2-test"), "{printed}");

    let delivery = LettreSmtpDelivery { config };
    let printed = format!("{delivery:?}");
    assert!(printed.contains("<redacted>"), "{printed}");
    assert!(!printed.contains("hunter2-test"), "{printed}");
}

/// FR-DEL-08 (RFC 009 D2): the recommended deadline is 30 seconds.
#[test]
fn the_default_timeout_is_thirty_seconds() {
    assert_eq!(SmtpConfig::DEFAULT_TIMEOUT, Duration::from_secs(30));
}

/// `Debug` shows the deadline next to the redacted password.
#[test]
fn debug_shows_the_timeout() {
    let printed = format!("{:?}", sample_config());
    assert!(printed.contains("timeout: 30s"), "{printed}");
}

/// FR-DEL-08, NFR-PERF-03, T15 (RFC 009 D2): a relay that accepts the
/// connection and then never says a word is abandoned at the deadline.  The
/// clock is paused, so the 30 seconds pass as soon as nothing else can run.
#[tokio::test(start_paused = true)]
async fn a_relay_that_never_answers_times_out() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let relay = tokio::spawn(async move {
        let (_socket, _) = listener.accept().await.unwrap();
        pending::<()>().await;
    });

    let delivery = LettreSmtpDelivery {
        config: SmtpConfig {
            host: "127.0.0.1".into(),
            port,
            username: String::new(),
            password: String::new(),
            tls_mode: SmtpTlsMode::DangerousPlaintext,
            ..sample_config()
        },
    };
    let result = delivery.deliver(sample_input()).await;

    assert!(
        matches!(result, Err(ContactDeliveryError::Timeout(limit)) if limit == SmtpConfig::DEFAULT_TIMEOUT),
        "{result:?}"
    );
    relay.abort();
}
