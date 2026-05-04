// model.rs — Core data model for contact form submissions.
//
// [`ContactInput`] is an internal model produced on the server after the raw
// server-function arguments have been normalised and validated.  It is never
// serialised to the client.

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::error::ContactValidationError;

// ---------------------------------------------------------------------------
// Custom validators
// ---------------------------------------------------------------------------

/// Rejects any string that contains a CR (`\r`) or LF (`\n`) character.
///
/// Used for `name` and `subject` to prevent email header injection.
fn no_newlines(value: &str) -> Result<(), validator::ValidationError> {
    if value.contains('\n') || value.contains('\r') {
        Err(validator::ValidationError::new("no_newlines"))
    } else {
        Ok(())
    }
}

/// Validator for optional string fields: rejects newlines when the value is present.
///
/// The `validator` crate passes the inner `&String` reference for `Option<String>` fields
/// when using a `custom` validator.
fn optional_no_newlines(value: &str) -> Result<(), validator::ValidationError> {
    no_newlines(value)
}

// ---------------------------------------------------------------------------
// ContactInput
// ---------------------------------------------------------------------------

/// Validated, normalised contact form submission.
///
/// This type lives entirely on the server.  SMTP credentials, the recipient
/// address, and any other secrets never appear here.
///
/// # Security
///
/// `name` and `subject` are validated to contain no newline characters to
/// prevent email header injection.  `email` is used as the `Reply-To` header
/// only — never as the `From` address.
///
/// # Feature flags
///
/// This type is available under both `ssr` and `hydrate` features so that
/// client-side code can reference its shape (e.g. for type-checking form
/// field names) without pulling in server-only dependencies.
#[derive(Clone, Debug, Serialize, Deserialize, Validate)]
pub struct ContactInput {
    /// Display name of the person submitting the form.
    ///
    /// Constraints: 1–80 characters, no newline characters.
    #[validate(
        length(min = 1, max = 80, message = "Name must be 1–80 characters"),
        custom(function = "no_newlines", message = "Name must not contain newlines")
    )]
    pub name: String,

    /// Email address used as the `Reply-To` header.
    ///
    /// Validated as an RFC 5321-style address by the `validator` crate.
    #[validate(email(message = "A valid email address is required"))]
    pub email: String,

    /// Optional subject line for the enquiry.
    ///
    /// Constraints: 0–120 characters, no newline characters.
    #[validate(
        length(max = 120, message = "Subject must be at most 120 characters"),
        custom(
            function = "optional_no_newlines",
            message = "Subject must not contain newlines"
        )
    )]
    pub subject: Option<String>,

    /// Body of the enquiry in plain text.
    ///
    /// Constraints: 1–4 000 characters.
    #[validate(length(min = 1, max = 4000, message = "Message must be 1–4 000 characters"))]
    pub message: String,

    /// Honeypot field — must be empty.
    ///
    /// This field is invisible to genuine users.  A non-empty value indicates
    /// an automated submission.  The server silently succeeds to avoid leaking
    /// detection logic to bots.
    #[serde(default)]
    pub website: String,
}

impl ContactInput {
    /// Construct a new [`ContactInput`] from raw server-function arguments.
    ///
    /// Performs trimming (whitespace normalisation) before storing values.
    /// Call [`ContactInput::check_honeypot`] and then [`Validate::validate`]
    /// after construction to complete the security checks.
    pub fn from_raw(
        name: String,
        email: String,
        subject: Option<String>,
        message: String,
        website: String,
    ) -> Self {
        Self {
            name: name.trim().to_owned(),
            email: email.trim().to_owned(),
            subject: subject
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty()),
            message: message.trim().to_owned(),
            website,
        }
    }

    /// Return `Err(ContactValidationError::HoneypotTriggered)` if the honeypot
    /// field is non-empty.
    ///
    /// This must be checked **before** returning any validation errors to the
    /// caller, so that bots cannot distinguish honeypot failures from genuine
    /// validation failures.
    pub fn check_honeypot(&self) -> Result<(), ContactValidationError> {
        if !self.website.trim().is_empty() {
            tracing::warn!("contact form honeypot triggered");
            return Err(ContactValidationError::HoneypotTriggered);
        }
        Ok(())
    }

    /// Run server-side validation via the [`Validate`] derive.
    ///
    /// Returns a structured error string suitable for logging (not for display
    /// to end-users).
    pub fn validate_input(&self) -> Result<(), ContactValidationError> {
        self.validate()
            .map_err(|e| ContactValidationError::InvalidInput(e.to_string()))
    }

    /// Run server-side validation and return per-field errors safe for client display.
    ///
    /// Unlike [`validate_input`](Self::validate_input), which returns an opaque
    /// server-internal message, this method returns a
    /// [`ContactFieldErrors`](crate::error::ContactFieldErrors) value with a
    /// generic human-readable message for each failed field.
    pub fn validate_fields(&self) -> crate::error::ContactFieldErrors {
        use validator::Validate as _;
        let mut out = crate::error::ContactFieldErrors::default();

        if let Err(ve) = self.validate() {
            for (field, errors) in ve.field_errors() {
                let msg = errors
                    .first()
                    .and_then(|e| e.message.as_deref())
                    .unwrap_or("Invalid value")
                    .to_owned();
                match field.as_ref() {
                    "name" => out.name = Some(msg),
                    "email" => out.email = Some(msg),
                    "subject" => out.subject = Some(msg),
                    "message" => out.message = Some(msg),
                    _ => {}
                }
            }
        }
        out
    }

    /// Resolve the effective subject line, falling back to a default when the
    /// caller did not supply one or it was blank after trimming.
    pub fn effective_subject(&self, fallback: &str) -> String {
        self.subject
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_owned()
    }
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
