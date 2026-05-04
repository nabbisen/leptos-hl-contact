// error.rs — Public error types for leptos-hl-contact.
//
// Security contract:
//   - ContactDeliveryError stays server-side; never forward its message to
//     the client.
//   - ContactFieldErrors is designed for safe client display; it contains
//     only validated field names and generic length/format messages.
//   - ContactValidationError is a server-internal type; do not expose its
//     InvalidInput message to the client.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// ContactFieldErrors
// ---------------------------------------------------------------------------

/// Sentinel prefix used to identify [`ContactFieldErrors`] payloads inside
/// `ServerFnError` message strings.
pub const FIELD_ERROR_PREFIX: &str = "field_errors:";

/// Per-field validation error messages, safe to display to end-users.
///
/// Returned by [`submit_contact`](crate::server::submit_contact) when
/// server-side validation fails.  Each field holds `Some(message)` when that
/// field failed, or `None` when it passed.
///
/// The component parses this from the `ServerFnError` payload and renders
/// each message next to the corresponding input.
///
/// # Security
///
/// Messages are generic ("required", "too long", "invalid email") — they
/// never echo user input back or reveal internal stack traces.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContactFieldErrors {
    /// Error for the `name` field.
    pub name: Option<String>,
    /// Error for the `email` field.
    pub email: Option<String>,
    /// Error for the `subject` field.
    pub subject: Option<String>,
    /// Error for the `message` field.
    pub message: Option<String>,
}

impl ContactFieldErrors {
    /// Returns `true` when every field is `None`.
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.email.is_none()
            && self.subject.is_none()
            && self.message.is_none()
    }

    /// Serialise to a compact JSON string for embedding in a
    /// `ServerFnError::Args` payload.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialise from the JSON string embedded in a `ServerFnError` message.
    ///
    /// Returns `None` when the string is not a `ContactFieldErrors` payload.
    pub fn from_error_str(s: &str) -> Option<Self> {
        let json = s.strip_prefix(FIELD_ERROR_PREFIX)?;
        serde_json::from_str(json).ok()
    }

    /// Encode this value into a `ServerFnError::Args` message string.
    pub fn into_server_fn_message(self) -> String {
        format!("{}{}", FIELD_ERROR_PREFIX, self.to_json())
    }
}

// ---------------------------------------------------------------------------
// ContactDeliveryError
// ---------------------------------------------------------------------------

/// Error returned by a [`ContactDelivery`](crate::delivery::ContactDelivery)
/// implementation.
///
/// Keep these on the server — log them and return only a generic string to
/// the client.
#[derive(Debug, Error)]
pub enum ContactDeliveryError {
    /// The delivery backend is not configured or unavailable.
    #[error("delivery backend configuration error: {0}")]
    Configuration(String),

    /// A transient transport error (e.g. SMTP connection failure).
    #[error("transport error: {0}")]
    Transport(String),

    /// The message could not be built (e.g. invalid address at build time).
    #[error("message build error: {0}")]
    MessageBuild(String),

    /// An unexpected internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

// ---------------------------------------------------------------------------
// ContactValidationError
// ---------------------------------------------------------------------------

/// Server-internal validation error.  Do **not** forward to the client.
#[derive(Debug, Error)]
pub enum ContactValidationError {
    /// One or more fields failed validation.
    #[error("validation failed: {0}")]
    InvalidInput(String),

    /// The honeypot field contained a non-empty value.
    #[error("honeypot triggered")]
    HoneypotTriggered,
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
