// security.rs — Security utilities for leptos-hl-contact.
//
// This module holds defence-in-depth helpers shared by other modules (for
// example header-injection sanitisation); the anti-automation token lives in
// `form-token`.

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
/// Each CR and LF is replaced by one space, so a CRLF pair becomes two
/// spaces.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::security::sanitize_header_value;
///
/// let safe = sanitize_header_value("Hello\nInjected: header");
/// assert_eq!(safe, "Hello Injected: header");
/// assert_eq!(sanitize_header_value("a\r\nb"), "a  b");
/// ```
pub fn sanitize_header_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
