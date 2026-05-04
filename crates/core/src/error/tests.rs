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
