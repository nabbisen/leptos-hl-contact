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

/// Sentinel prefix used to identify a [`ContactErrorCode`] inside
/// `ServerFnError` message strings.
pub const CONTACT_ERROR_PREFIX: &str = "contact_error:";

// ---------------------------------------------------------------------------
// Field error codes
// ---------------------------------------------------------------------------

/// Which field an error belongs to.
///
/// Used to pick between the general and the email-specific `format` label.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactField {
    /// The `name` field.
    Name,
    /// The `email` field.
    Email,
    /// The `subject` field.
    Subject,
    /// The `message` field.
    Message,
}

/// Why one field failed, as a code the client turns into text.
///
/// Lengths are counted in characters (Unicode scalar values), matching the
/// validator and the component's `maxlength`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FieldErrorCode {
    /// The field is required and was absent or blank.
    Required,
    /// The field's length is outside the permitted range.
    Length {
        /// Minimum number of characters.
        min: usize,
        /// Maximum number of characters.
        max: usize,
    },
    /// The value is syntactically invalid, for example an email address.
    Format,
    /// The value contains a CR or LF, which is rejected to prevent email
    /// header injection.
    LineBreaks,
}

/// One field's error: a code, or text a server rendered itself.
///
/// The `Text` variant exists for compatibility: a 0.3 server sent English
/// sentences, and `serde(untagged)` lets a current client still read them.
/// A current server only ever sends [`Code`](Self::Code).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldError {
    /// A code the client renders from [`ContactErrorLabels`](crate::config::ContactErrorLabels).
    Code(FieldErrorCode),
    /// Pre-rendered text, shown unchanged.
    Text(String),
}

// ---------------------------------------------------------------------------
// ContactErrorCode
// ---------------------------------------------------------------------------

/// A whole-submission failure, as a code the client turns into text.
///
/// These reach the client as `contact_error:<code>` inside a
/// `ServerFnError`; see [`from_server_fn_error`](Self::from_server_fn_error).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactErrorCode {
    /// The form token was missing, malformed, or expired.
    TokenInvalid,
    /// The form was submitted sooner after loading than the configured
    /// minimum age.  Retryable: the same token works moments later.
    TooFast,
    /// A required context value is absent, so the form cannot accept
    /// submissions.
    NotConfigured,
    /// The delivery backend refused or failed.
    DeliveryFailed,
    /// Something unforeseen went wrong.
    Unexpected,
    /// A challenge is configured but the submission carried no token.
    ChallengeRequired,
    /// The challenge vendor rejected the token, or its score or action did
    /// not satisfy the policy.
    ChallengeFailed,
    /// The challenge vendor could not be asked.  Fail-closed.
    ChallengeUnavailable,
    /// A `ContactFilter` refused the submission.
    Rejected,
}

impl ContactErrorCode {
    /// The wire spelling, without the prefix.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TokenInvalid => "token_invalid",
            Self::TooFast => "too_fast",
            Self::NotConfigured => "not_configured",
            Self::DeliveryFailed => "delivery_failed",
            Self::Unexpected => "unexpected",
            Self::ChallengeRequired => "challenge_required",
            Self::ChallengeFailed => "challenge_failed",
            Self::ChallengeUnavailable => "challenge_unavailable",
            Self::Rejected => "rejected",
        }
    }

    /// Parse a wire spelling.  Returns `None` for anything unrecognised, so
    /// a newer server's code falls back to the generic banner rather than
    /// rendering nothing.
    pub fn from_str_code(s: &str) -> Option<Self> {
        match s {
            "token_invalid" => Some(Self::TokenInvalid),
            "too_fast" => Some(Self::TooFast),
            "not_configured" => Some(Self::NotConfigured),
            "delivery_failed" => Some(Self::DeliveryFailed),
            "unexpected" => Some(Self::Unexpected),
            "challenge_required" => Some(Self::ChallengeRequired),
            "challenge_failed" => Some(Self::ChallengeFailed),
            "challenge_unavailable" => Some(Self::ChallengeUnavailable),
            "rejected" => Some(Self::Rejected),
            _ => None,
        }
    }

    /// Encode into a `ServerFnError` message string.
    pub fn into_server_fn_message(self) -> String {
        format!("{}{}", CONTACT_ERROR_PREFIX, self.as_str())
    }

    /// Extract a code from a server-function error.
    ///
    /// Looks in both the `Args` and `ServerError` variants, and tolerates
    /// leading text: the framework prefixes `Args` with
    /// `"error deserializing server function arguments: "`.
    pub fn from_server_fn_error<E>(
        err: &leptos::server_fn::error::ServerFnError<E>,
    ) -> Option<Self> {
        use leptos::server_fn::error::ServerFnError;
        let s = match err {
            ServerFnError::Args(s) | ServerFnError::ServerError(s) => s,
            _ => return None,
        };
        let at = s.find(CONTACT_ERROR_PREFIX)?;
        Self::from_str_code(s[at + CONTACT_ERROR_PREFIX.len()..].trim())
    }
}

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
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactFieldErrors {
    /// Error for the `name` field.
    pub name: Option<FieldError>,
    /// Error for the `email` field.
    pub email: Option<FieldError>,
    /// Error for the `subject` field.
    pub subject: Option<FieldError>,
    /// Error for the `message` field.
    pub message: Option<FieldError>,
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
    /// The sentinel is located anywhere in `s`, not only at the start: the
    /// framework's `Display` for `ServerFnError::Args` prefixes the payload
    /// with `"error deserializing server function arguments: "`, so a caller
    /// holding the displayed string never sees the sentinel first.
    ///
    /// Returns `None` when the sentinel is absent or the text after it is not
    /// valid `ContactFieldErrors` JSON.
    ///
    /// Prefer [`from_server_fn_error`](Self::from_server_fn_error) when the
    /// error value itself is available; this function is the fallback for
    /// callers that hold only a string.
    pub fn from_error_str(s: &str) -> Option<Self> {
        let at = s.find(FIELD_ERROR_PREFIX)?;
        let json = &s[at + FIELD_ERROR_PREFIX.len()..];
        serde_json::from_str(json).ok()
    }

    /// Extract a field-error payload from a server-function error.
    ///
    /// This is the intended way for a client to detect one.  Only the
    /// [`Args`](leptos::server_fn::error::ServerFnError::Args) variant can
    /// carry a payload; every other variant — including an `Args` holding
    /// plain text such as the token-failure message — yields `None` and
    /// belongs on the generic-error path.
    ///
    /// Matching the variant means the check never depends on the framework's
    /// English `Display` text.
    pub fn from_server_fn_error<E>(
        err: &leptos::server_fn::error::ServerFnError<E>,
    ) -> Option<Self> {
        match err {
            leptos::server_fn::error::ServerFnError::Args(s) => Self::from_error_str(s),
            _ => None,
        }
    }

    /// The error for one field, if any.
    pub fn get(&self, field: ContactField) -> Option<&FieldError> {
        match field {
            ContactField::Name => self.name.as_ref(),
            ContactField::Email => self.email.as_ref(),
            ContactField::Subject => self.subject.as_ref(),
            ContactField::Message => self.message.as_ref(),
        }
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
///
/// `submit_contact` writes the error's `Display` text to the server log.  An
/// implementation puts the category and the transport detail in it — status
/// codes, the relay's reply — and never the submission: no name, email
/// address, subject, message, token or credential.
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
