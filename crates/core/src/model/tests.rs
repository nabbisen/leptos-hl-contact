// tests.rs — unit tests for the parent module.

use super::*;
use validator::Validate;

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
        input.message = "x".repeat(4001);
        assert!(input.validate_input().is_err());
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
