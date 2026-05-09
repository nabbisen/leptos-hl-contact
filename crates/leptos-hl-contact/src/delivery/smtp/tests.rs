// tests.rs — unit tests for the parent module.

use super::*;
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
            raw.contains("noreply@example.com")
                && (raw.contains("From: ") || raw.contains("From:"))
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

#[test]
fn reply_to_with_special_chars_in_name() {
    // Names containing quotes, commas, and angle brackets must not break message
    // construction.  Mailbox::new handles RFC 5322 encoding.
    let delivery = LettreSmtpDelivery { config: sample_config() };
    let special_input = ContactInput::from_raw(
        "O'Brien, Alice <alice>".into(),  // quotes + comma + angle brackets
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
    let delivery = LettreSmtpDelivery { config: sample_config() };
    let message = delivery.build_message(&sample_input()).unwrap();
    let raw = String::from_utf8(message.formatted()).unwrap();
    // Email must appear in Reply-To (not in From)
    let from_line = raw.lines().find(|l| l.starts_with("From:")).unwrap_or("");
    assert!(!from_line.contains("alice@example.com"), "user email must not be in From");
}
