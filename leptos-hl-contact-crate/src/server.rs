// server.rs — Leptos server function for contact form submission.

use leptos::prelude::*;
use leptos::server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
use crate::{
    delivery::ContactDeliveryContext,
    error::{ContactFieldErrors, ContactValidationError},
    model::ContactInput,
};

/// Submit a contact form enquiry.
///
/// This is a Leptos **server function** — it compiles to a public HTTP endpoint.
/// Treat it like any other public API: validate all input server-side, never
/// expose internal errors, and require HTTPS in production.
///
/// # Arguments
///
/// Arguments map 1-to-1 to HTML `<input name="…">` fields so that
/// `<ActionForm/>` works without JavaScript.
///
/// | Argument  | Required | Notes                               |
/// |-----------|----------|-------------------------------------|
/// | `name`    | Yes      | Enquirer display name               |
/// | `email`   | Yes      | Used as `Reply-To`; never as `From` |
/// | `subject` | No       | Optional subject line               |
/// | `message` | Yes      | Plain-text body, up to 4 000 chars  |
/// | `website` | —        | Honeypot; must be empty             |
///
/// # Errors
///
/// Returns [`ServerFnError::Args`] with a [`ContactFieldErrors`] JSON payload
/// (prefixed by [`FIELD_ERROR_PREFIX`](crate::error::FIELD_ERROR_PREFIX)) when
/// server-side validation fails.  The component decodes this to show per-field
/// messages.  Other errors return a generic delivery-failure string.
///
/// # Security
///
/// - Server-side validation is **always** performed.
/// - A non-empty `website` silently succeeds (honeypot).
/// - Delivery errors are logged server-side; only a generic string reaches the
///   client.
/// - SMTP credentials never leave the server.
#[server]
pub async fn submit_contact(
    name: String,
    email: String,
    subject: Option<String>,
    message: String,
    website: String,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use leptos::context::use_context;

        // 1. Normalise raw input.
        let input = ContactInput::from_raw(name, email, subject, message, website);

        // 2. Honeypot — silent success.
        match input.check_honeypot() {
            Ok(()) => {}
            Err(ContactValidationError::HoneypotTriggered) => return Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "unexpected honeypot error");
                return Err(ServerFnError::ServerError(
                    "An unexpected error occurred.".into(),
                ));
            }
        }

        // 3. Server-side validation — return per-field errors to the client.
        let field_errors = input.validate_fields();
        if !field_errors.is_empty() {
            // Log a brief summary (no PII, no field values).
            tracing::debug!(
                name_err = field_errors.name.is_some(),
                email_err = field_errors.email.is_some(),
                subject_err = field_errors.subject.is_some(),
                message_err = field_errors.message.is_some(),
                "contact form validation failed"
            );
            return Err(ServerFnError::Args(field_errors.into_server_fn_message()));
        }

        // 4. Retrieve delivery backend from Leptos context.
        let Some(delivery) = use_context::<ContactDeliveryContext>() else {
            tracing::error!("ContactDeliveryContext not provided — check server setup");
            return Err(ServerFnError::ServerError(
                "Contact form is not configured. Please contact the site administrator.".into(),
            ));
        };

        // 5. Deliver.
        if let Err(e) = delivery.deliver(input).await {
            tracing::error!(error = %e, "contact form delivery failed");
            return Err(ServerFnError::ServerError(
                "Failed to send message. Please try again later.".into(),
            ));
        }

        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(ServerFnError::ServerError("SSR not enabled".into()))
}

// ---------------------------------------------------------------------------
// Server-side security tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "ssr"))]
mod tests {
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
}
