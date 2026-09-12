// server.rs — Leptos server function for contact form submission.

// `submit_contact` takes one argument per form field — the wire contract, not
// a design choice.  `#[server]` does not copy item attributes onto the client
// stub it generates for the browser build, so the allow has to be here.
#![allow(
    clippy::too_many_arguments,
    reason = "submit_contact's arguments are its form fields"
)]

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
/// | `cf-turnstile-response` | — | Turnstile's token field |
/// | `h-captcha-response` | — | hCaptcha's token field |
/// | `g-recaptcha-response` | — | reCAPTCHA's token field |
///
/// # Challenge
///
/// The first non-blank challenge field is the token.  With
/// [`ChallengeContext`](crate::challenge::ChallengeContext) in context it is
/// verified after the server policy and before delivery, following the
/// decision table in RFC 005: a token with no context is `not_configured`;
/// no token under `NoJsPolicy::Reject` is `challenge_required`; a failed
/// check is `challenge_failed`; a verifier error is `challenge_unavailable`.
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
    /// Cloudflare Turnstile's token, under the field name its widget injects.
    #[server(rename = "cf-turnstile-response")]
    #[server(default)]
    cf_turnstile_response: Option<String>,
    /// hCaptcha's token, under the field name its widget injects.
    #[server(rename = "h-captcha-response")]
    #[server(default)]
    h_captcha_response: Option<String>,
    /// reCAPTCHA's token, under the field name its widget injects.
    #[server(rename = "g-recaptcha-response")]
    #[server(default)]
    g_recaptcha_response: Option<String>,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use leptos::context::use_context;

        // 1. Form token — fail-closed when the `form-token` feature is on.
        #[cfg(feature = "form-token")]
        {
            use crate::form_token::{
                Binding, FormTokenBinding, FormTokenContext, FormTokenError, verify_form_token,
            };
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
            // Only `Binding::Cookie` consults it; an absent context and
            // `FormTokenBinding(None)` mean the same thing.
            let bound = if config.binding == Binding::Cookie {
                use_context::<FormTokenBinding>().and_then(|b| b.0)
            } else {
                None
            };
            if let Err(e) = verify_form_token(submitted, bound.as_deref(), &config) {
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

        // 6. Every configuration check, made now: nothing below may call a
        // vendor or a mail server before the server is known to be fully
        // configured.
        let Some(delivery) = use_context::<ContactDeliveryContext>() else {
            tracing::error!("ContactDeliveryContext not provided — check server setup");
            return Err(ServerFnError::ServerError(
                ContactErrorCode::NotConfigured.into_server_fn_message(),
            ));
        };
        let success_redirect = use_context::<crate::config::ContactSuccessRedirect>();
        let challenge_ctx = use_context::<crate::challenge::ChallengeContext>();

        // 7. Challenge — after every local check, so invalid input never costs
        // a vendor call.  Never log the token.
        {
            use crate::challenge::{self, Gate};

            let challenge_token = [
                cf_turnstile_response,
                h_captcha_response,
                g_recaptcha_response,
            ]
            .into_iter()
            .flatten()
            .find(|t| !t.trim().is_empty());

            match challenge::gate(challenge_ctx.as_ref(), challenge_token.as_deref()) {
                Gate::Proceed => {
                    if challenge_ctx.is_some() {
                        tracing::info!(
                            "challenge skipped: no token, policy AcceptWithHoneypotOnly"
                        );
                    }
                }
                Gate::Reject(code) => {
                    if code == ContactErrorCode::NotConfigured {
                        tracing::error!(
                            "challenge token received but no ChallengeContext is provided"
                        );
                    }
                    return Err(challenge::rejection(code));
                }
                Gate::Verify => {
                    let (Some(ctx), Some(token)) = (challenge_ctx, challenge_token) else {
                        unreachable!("gate returns Verify only with a context and a token");
                    };
                    let result = ctx.verifier.verify(&token).await;
                    let error_codes = result.as_ref().ok().map(|o| o.error_codes.clone());
                    let error = result.as_ref().err().map(ToString::to_string);
                    if let Err(code) = challenge::judge(result, &ctx.policy) {
                        match code {
                            ContactErrorCode::ChallengeUnavailable => tracing::error!(
                                error = error.as_deref().unwrap_or_default(),
                                "challenge verifier unavailable"
                            ),
                            _ => tracing::warn!(error_codes = ?error_codes, "challenge failed"),
                        }
                        return Err(challenge::rejection(code));
                    }
                }
            }
        }

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
// issue_form_token_fn
// ---------------------------------------------------------------------------

/// Issue a form token for a form that was not rendered with one.
///
/// Leptos server function compiled to `POST /api/form_token`.  `ContactForm`
/// calls it from the browser only when
/// [`ContactFormOptions::token_refresh_secs`](crate::config::ContactFormOptions::token_refresh_secs)
/// is set, and then in two cases: the form was created by client-side
/// navigation, so no server render put a token in it; or the token it has is
/// due for a refresh.
///
/// On the server it reads
/// [`FormTokenContext`](crate::form_token::FormTokenContext); without it the
/// call fails with `not_configured`, as `submit_contact` does.  With
/// [`Binding::Cookie`](crate::form_token::Binding::Cookie) the token reuses
/// the nonce the browser already holds, so fetching a token never
/// invalidates a form open in another tab.  If a
/// [`FormTokenIssuer`](crate::form_token::FormTokenIssuer) is in context it
/// is called with the token, which is how the Axum helper sets the cookie.
///
/// The endpoint is public, and grants nothing a page render does not: cover
/// it with the same rate limit as the rest of the site.
///
/// Declared outside the `form_token` module because the browser build, which
/// is the caller, never has the `form-token` feature.
#[cfg(any(feature = "form-token", not(feature = "ssr")))]
#[server(endpoint = "form_token")]
pub async fn issue_form_token_fn() -> Result<String, ServerFnError> {
    #[cfg(feature = "form-token")]
    {
        crate::form_token::issue_for_request().map(|t| t.0)
    }
    // Without `form-token` this function exists only in the browser build,
    // where `#[server]` replaces the body with a request.  This branch never
    // runs; it only has to type-check.
    #[cfg(not(feature = "form-token"))]
    {
        Err(ServerFnError::ServerError(
            crate::error::ContactErrorCode::NotConfigured.into_server_fn_message(),
        ))
    }
}

/// The issue time of a token, in Unix seconds — its first `|` segment.
///
/// The browser uses it to schedule a refresh.  It does not verify anything:
/// a malformed value yields `None` and simply schedules nothing.
#[cfg_attr(
    not(all(feature = "hydrate", not(feature = "ssr"))),
    allow(dead_code, reason = "used by the client refresh and by tests")
)]
pub(crate) fn token_issued_at(token: &str) -> Option<u64> {
    let (timestamp, rest) = token.split_once('|')?;
    // A bare number is not a token.
    rest.contains('|').then_some(())?;
    timestamp.parse().ok()
}

/// Seconds until the token a form *mounted* with is due for refresh.
///
/// `issued` is that token's server timestamp and `now` the browser's clock.
/// An overdue token yields `0`: refresh once, immediately.  Only the mounted
/// token is measured this way.  A token received from the endpoint is
/// refreshed `refresh` seconds after it arrives, on the browser clock alone,
/// so however wrong that clock is, this function costs at most one early
/// request per mount.
///
/// The delay never exceeds `refresh`.  A longer one can only come from a
/// browser clock running slow, because the token was issued no later than
/// the form mounted; its real refresh point is at most `refresh` seconds
/// away, so the cap is always correct.
#[cfg_attr(
    not(all(feature = "hydrate", not(feature = "ssr"))),
    allow(dead_code, reason = "used by the client refresh and by tests")
)]
pub(crate) fn mounted_refresh_delay(issued: u64, now: u64, refresh: u64) -> u64 {
    issued
        .saturating_add(refresh)
        .saturating_sub(now)
        .min(refresh)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "ssr"))]
mod tests;
