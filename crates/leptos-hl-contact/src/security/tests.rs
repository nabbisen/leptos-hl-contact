// tests.rs — unit tests for the parent module.

use super::*;
#[test]
    fn header_injection_attempt_is_sanitised() {
        let input = "Subject\r\nBcc: evil@example.com";
        let result = sanitize_header_value(input);
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
    }
