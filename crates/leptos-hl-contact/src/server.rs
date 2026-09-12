// server.rs — Leptos server function for contact form submission.

use leptos::prelude::*;
use leptos::server_fn::error::ServerFnError;

#[cfg(feature = "ssr")]
use crate::{
    delivery::ContactDeliveryContext,
    error::{ContactErrorCode, ContactValidationError},
    model::ContactInput,
};

/// Submit a contact form enquiry.
///
/// Leptos server function compiled to `POST /api/submit_contact`.
///
/// # Arguments
///
/// | Argument     | Required | Notes |
/// |--------------|----------|-------|
/// | `name`       | Yes | Enquirer display name |
/// | `email`      | Yes | Used as `Reply-To`; never as `From` |
/// | `subject`    | No  | Optional subject line |
/// | `message`    | Yes | Plain-text body, up to [`MESSAGE_MAX_LEN`](crate::model::MESSAGE_MAX_LEN) characters |
/// | `website`    | —   | Honeypot; must be empty |
/// | `form_token` | —   | `Option<String>`; verified when `FormTokenContext` is in context |
/// | `csrf_token` | —   | **Deprecated** 0.4 name; used only when `form_token` is absent |
///
/// # Form token
///
/// When the `form-token` feature is enabled **and**
/// [`FormTokenContext`](crate::form_token::FormTokenContext) is present in the
/// Leptos context, the submitted token is verified with
/// [`verify_form_token`](crate::form_token::verify_form_token).  A token
/// younger than the configured minimum age is rejected with
/// [`TooFast`](crate::error::ContactErrorCode::TooFast), which is retryable;
/// every other failure is reported as `token_invalid`.
///
/// When the feature is enabled but `FormTokenContext` is **not** provided, the
/// call is **rejected** with a configuration error.  This fail-closed
/// behaviour prevents a silently unprotected form when the context injection
/// is accidentally omitted.
///
/// # Server-side policy
///
/// When [`ContactServerPolicy`](crate::config::ContactServerPolicy) is provided via
/// context, `require_subject` and `max_message_len` are enforced server-side.
///
/// # Success redirect
///
/// When [`ContactSuccessRedirect`](crate::config::ContactSuccessRedirect) is
/// provided via context, a successful delivery runs it before returning, so
/// the visitor is sent to the configured page with or without JavaScript.
/// Without it, behaviour is unchanged: a JavaScript client shows the inline
/// success message and a no-JavaScript client reloads the form page.
///
/// # Security
///
/// - Server-side validation is always performed.
/// - A non-empty `website` silently succeeds (honeypot).
/// - Delivery errors are logged server-side; only a generic string reaches the client.
/// - SMTP credentials never leave the server.
#[server(endpoint = "submit_contact")]
pub async fn submit_contact(
    name: String,
    email: String,
    subject: Option<String>,
    message: String,
    website: String,
    /// Token issued by `ContactForm`.  `Option<String>` so that forms lacking
    /// the hidden field do not fail at deserialization.  When the
    /// `form-token` feature is active and `FormTokenContext` is provided, a
    /// missing token is treated as invalid.
    form_token: Option<String>,
    /// **Deprecated.**  The 0.4 name for `form_token`, accepted for one minor
    /// so a page rendered by 0.4 still submits successfully to 0.5.  Used
    /// only when `form_token` is absent.
    csrf_token: Option<String>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use leptos::context::use_context;

        // 1. Form token — fail-closed when the `form-token` feature is on.
        #[cfg(feature = "form-token")]
        {
            use crate::form_token::{FormTokenContext, FormTokenError, verify_form_token};
            let Some(config) = use_context::<FormTokenContext>() else {
                tracing::error!(
                    "form-token feature is enabled but FormTokenContext is not provided \
                     — check that you supply it in the context closure"
                );
                return Err(ServerFnError::ServerError(
                    ContactErrorCode::NotConfigured.into_server_fn_message(),
                ));
            };
            // `csrf_token` is the 0.4 field name, accepted for one minor.
            let submitted = form_token
                .as_deref()
                .or(csrf_token.as_deref())
                .unwrap_or("");
            if let Err(e) = verify_form_token(submitted, None, &config) {
                tracing::warn!(error = ?e, "form token rejected");
                let code = match e {
                    // Retryable: the same token is valid a moment later, so
                    // a fast human is delayed rather than turned away.
                    FormTokenError::TooYoung => ContactErrorCode::TooFast,
                    _ => ContactErrorCode::TokenInvalid,
                };
                return Err(ServerFnError::Args(code.into_server_fn_message()));
            }
        }
        // Without `form-token` the parameters are still part of the wire
        // contract but nothing reads them; bind them so the combination
        // compiles warning-free.
        #[cfg(not(feature = "form-token"))]
        let _ = (&form_token, &csrf_token);

        // 2. Normalise raw input.
        let input = ContactInput::from_raw(name, email, subject, message, website);

        // 3. Honeypot — silent success.
        match input.check_honeypot() {
            Ok(()) => {}
            Err(ContactValidationError::HoneypotTriggered) => return Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "unexpected honeypot error");
                return Err(ServerFnError::ServerError(
                    ContactErrorCode::Unexpected.into_server_fn_message(),
                ));
            }
        }

        // 4. Server-side field validation.
        let field_errors = input.validate_fields();
        if !field_errors.is_empty() {
            tracing::debug!(
                name_err = field_errors.name.is_some(),
                email_err = field_errors.email.is_some(),
                subject_err = field_errors.subject.is_some(),
                message_err = field_errors.message.is_some(),
                "contact form validation failed"
            );
            return Err(ServerFnError::Args(field_errors.into_server_fn_message()));
        }

        // 5. Server-side policy (require_subject, max_message_len).
        {
            use crate::config::ContactServerPolicy;
            if let Some(policy) = use_context::<ContactServerPolicy>() {
                let errs = policy.check(&input);
                if !errs.is_empty() {
                    return Err(ServerFnError::Args(errs.into_server_fn_message()));
                }
            }
        }

        // 6. Delivery backend.
        let Some(delivery) = use_context::<ContactDeliveryContext>() else {
            tracing::error!("ContactDeliveryContext not provided — check server setup");
            return Err(ServerFnError::ServerError(
                ContactErrorCode::NotConfigured.into_server_fn_message(),
            ));
        };

        // 7. Read the optional success redirect *before* awaiting delivery:
        // Leptos context is not reachable after an await point here.
        let success_redirect = use_context::<crate::config::ContactSuccessRedirect>();

        // 8. Deliver.
        if let Err(e) = delivery.deliver(input).await {
            tracing::error!(error = %e, "contact form delivery failed");
            return Err(ServerFnError::ServerError(
                ContactErrorCode::DeliveryFailed.into_server_fn_message(),
            ));
        }

        // 9. Success redirect, so the no-JS path can confirm too.
        if let Some(redirect) = success_redirect {
            redirect.apply();
        }

        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(ServerFnError::ServerError("SSR not enabled".into()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "ssr"))]
mod tests;
