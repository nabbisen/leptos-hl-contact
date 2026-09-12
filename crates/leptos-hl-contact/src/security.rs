// security.rs — Security utilities for leptos-hl-contact.
//
// This module holds defence-in-depth helpers shared by other modules (for
// example header-injection sanitisation); the anti-automation token lives in
// `csrf`.

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
mod tests;
