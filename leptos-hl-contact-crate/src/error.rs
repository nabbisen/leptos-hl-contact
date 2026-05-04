// error.rs — Public error types for leptos-hl-contact.
//
// Rules:
//   - ContactDeliveryError is returned from the delivery trait.
//   - Internal details (SMTP credentials, transport errors) are logged server-side
//     and never exposed to the client.

use thiserror::Error;

/// Error returned by a [`ContactDelivery`](crate::delivery::ContactDelivery) implementation.
///
/// Variants carry enough information for server-side logging, but callers should
/// convert them to a generic user-facing message before sending to the client.
#[derive(Debug, Error)]
pub enum ContactDeliveryError {
    /// The delivery backend is not configured or is unavailable.
    #[error("delivery backend configuration error: {0}")]
    Configuration(String),

    /// A transient transport error occurred (e.g. SMTP connection failure).
    #[error("transport error: {0}")]
    Transport(String),

    /// The message could not be built (e.g. invalid address format detected at build time).
    #[error("message build error: {0}")]
    MessageBuild(String),

    /// An unexpected internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Error returned when contact input validation fails on the server side.
///
/// Distinct from [`ContactDeliveryError`]: validation errors may be surfaced to
/// the user (as generic form errors), while delivery errors must stay server-side.
#[derive(Debug, Error)]
pub enum ContactValidationError {
    /// One or more fields failed validation.
    ///
    /// The inner string contains a structured description suitable for logging.
    /// Do **not** forward this verbatim to the client.
    #[error("validation failed: {0}")]
    InvalidInput(String),

    /// The honeypot field contained a non-empty value.
    ///
    /// Treat this as a silent success to the caller; log the event server-side.
    #[error("honeypot triggered")]
    HoneypotTriggered,
}
