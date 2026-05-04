// server.rs — Leptos server function for contact form submission.

use leptos::prelude::*;
use leptos::server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
use crate::{delivery::ContactDeliveryContext, error::ContactValidationError, model::ContactInput};

/// Submit a contact form enquiry.
///
/// This is a Leptos **server function** — it compiles to a public HTTP endpoint.
/// Treat it like any other public API: validate all input server-side, never
/// expose internal errors, and require HTTPS in production.
///
/// # Security
///
/// - Server-side validation is always performed.
/// - A non-empty `website` silently succeeds (honeypot).
/// - Delivery errors are logged server-side only.
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

        let input = ContactInput::from_raw(name, email, subject, message, website);

        // Honeypot — silent success on trigger.
        match input.check_honeypot() {
            Ok(()) => {}
            Err(ContactValidationError::HoneypotTriggered) => return Ok(()),
            Err(e) => return Err(ServerFnError::ServerError(e.to_string())),
        }

        // Server-side validation.
        if let Err(e) = input.validate_input() {
            tracing::debug!(error = %e, "contact form validation failed");
            return Err(ServerFnError::Args(
                "Please check the fields and try again.".into(),
            ));
        }

        // Resolve delivery backend from Leptos context.
        let Some(delivery) = use_context::<ContactDeliveryContext>() else {
            tracing::error!("ContactDeliveryContext not provided");
            return Err(ServerFnError::ServerError(
                "Contact form is not configured.".into(),
            ));
        };

        // Deliver.
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
