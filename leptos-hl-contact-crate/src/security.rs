// security.rs — Security utilities and documentation for leptos-hl-contact.
//
// This module does not export runtime functionality in the MVP.  Its purpose
// is to:
//
//   1. Centralise security-related helper functions that may be called from
//      multiple modules (e.g. header-injection sanitisation).
//   2. Serve as an anchor for security documentation in rustdoc.
//   3. Provide a home for future CSRF helpers and rate-limit guides.

// ---------------------------------------------------------------------------
// Header injection helpers
// ---------------------------------------------------------------------------

/// Strip carriage-return and line-feed characters from a string intended for
/// use in an email header (e.g. `Subject`, `From` display name).
///
/// Although [`ContactInput`](crate::model::ContactInput) validation already
/// rejects newlines, this function provides defence-in-depth for any code path
/// that builds header values from user input.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::security::sanitize_header_value;
///
/// let safe = sanitize_header_value("Hello\r\nInjected: header");
/// assert_eq!(safe, "Hello Injected: header");
/// ```
pub fn sanitize_header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_injection_attempt_is_sanitised() {
        let input = "Subject\r\nBcc: evil@example.com";
        let result = sanitize_header_value(input);
        assert!(!result.contains('\n'));
        assert!(!result.contains('\r'));
    }
}
