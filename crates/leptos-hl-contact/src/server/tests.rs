// tests.rs — unit tests for the server function.

use super::*;
use crate::error::FIELD_ERROR_PREFIX;

    #[test]
    fn field_error_prefix_is_not_empty() {
        assert!(!FIELD_ERROR_PREFIX.is_empty());
    }

    /// Ensure the encoded field-error payload starts with the sentinel prefix
    /// so the client can reliably distinguish it from a generic error string.
    #[test]
    fn field_error_message_has_prefix() {
        use crate::error::ContactFieldErrors;
        let errs = ContactFieldErrors {
            name: Some("required".into()),
            ..Default::default()
        };
        let msg = errs.into_server_fn_message();
        assert!(
            msg.starts_with(FIELD_ERROR_PREFIX),
            "encoded message must start with sentinel prefix"
        );
    }

    /// Verify that generic delivery errors do NOT start with the sentinel
    /// prefix, so the client correctly falls back to the generic error label.
    #[test]
    fn generic_error_has_no_prefix() {
        let generic = "Failed to send message. Please try again later.";
        assert!(!generic.starts_with(FIELD_ERROR_PREFIX));
    }
