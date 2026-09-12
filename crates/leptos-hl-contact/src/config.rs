// config.rs — Configuration types for ContactForm.
//
// The prop types are serialisable so they can cross the SSR/hydrate boundary.
// They must **never** contain secrets.  `ContactSuccessRedirect` is
// server-side configuration and is deliberately not serialisable.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{error::ContactFieldErrors, model::ContactInput, model::MESSAGE_MAX_LEN};

// ---------------------------------------------------------------------------
// ContactFormClasses
// ---------------------------------------------------------------------------

/// CSS class overrides for every structural element of the contact form.
///
/// All fields default to empty strings so callers only need to specify the
/// classes they care about.  Works with Tailwind CSS, UnoCSS, vanilla CSS, or
/// any other class-based styling system.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::config::ContactFormClasses;
///
/// let classes = ContactFormClasses {
///     root: "max-w-lg mx-auto".into(),
///     button: "btn btn-primary".into(),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContactFormClasses {
    /// Outermost wrapper element of the form.
    pub root: String,
    /// Wrapper around each label + input pair.
    pub field: String,
    /// `<label>` elements.
    pub label: String,
    /// Single-line `<input>` elements.
    pub input: String,
    /// Multi-line `<textarea>` element.
    pub textarea: String,
    /// Submit `<button>`.
    pub button: String,
    /// Inline validation-error messages.
    pub error: String,
    /// Success message shown after a successful submission.
    pub success: String,
}

// ---------------------------------------------------------------------------
// ContactFormLabels
// ---------------------------------------------------------------------------

/// User-visible text strings for every label, button, and status message.
///
/// Provides English defaults.  Override any subset to localise the form or to
/// adapt copy to your product's voice.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::config::ContactFormLabels;
///
/// let labels = ContactFormLabels {
///     submit: "Send enquiry".into(),
///     success: "Thank you — we will be in touch shortly.".into(),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactFormLabels {
    /// Label for the name field.
    pub name: String,
    /// Label for the email field.
    pub email: String,
    /// Label for the subject field.
    pub subject: String,
    /// Label for the message field.
    pub message: String,
    /// Submit button text (idle state).
    pub submit: String,
    /// Submit button text while the request is in flight.
    pub sending: String,
    /// Message displayed after a successful submission.
    pub success: String,
    /// Generic error message displayed when delivery fails.
    pub error: String,
    /// Accessible description for the honeypot field (read by screen readers
    /// that discover the hidden element; should instruct users to leave it
    /// blank).
    pub honeypot_label: String,
}

impl Default for ContactFormLabels {
    fn default() -> Self {
        Self {
            name: "Name".into(),
            email: "Email".into(),
            subject: "Subject".into(),
            message: "Message".into(),
            submit: "Send".into(),
            sending: "Sending…".into(),
            success: "Your message has been sent. We will get back to you soon.".into(),
            error: "Failed to send message. Please try again later.".into(),
            honeypot_label: "Leave this field blank".into(),
        }
    }
}

// ---------------------------------------------------------------------------
// ContactFormOptions
// ---------------------------------------------------------------------------

/// Behavioural options for the contact form.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::config::ContactFormOptions;
///
/// let options = ContactFormOptions {
///     show_subject: false,
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactFormOptions {
    /// Whether the subject field is displayed.  Defaults to `true`.
    pub show_subject: bool,

    /// Whether the subject field is required when displayed.
    /// Has no effect when [`show_subject`](Self::show_subject) is `false`.
    /// Defaults to `false`.
    pub require_subject: bool,

    /// Maximum number of characters allowed in the message body.
    ///
    /// Values above [`MESSAGE_MAX_LEN`] are clamped to it; read the effective
    /// value with [`effective_max_message_len`](Self::effective_max_message_len).
    /// Defaults to [`MESSAGE_MAX_LEN`].
    pub max_message_len: usize,

    /// After a failed submission, move keyboard focus to the first invalid
    /// input.  Client-side only.  Defaults to `true`.
    pub focus_first_error: bool,
}

impl Default for ContactFormOptions {
    fn default() -> Self {
        Self {
            show_subject: true,
            require_subject: false,
            max_message_len: MESSAGE_MAX_LEN,
            focus_first_error: true,
        }
    }
}

impl ContactFormOptions {
    /// The `maxlength` the component renders: [`max_message_len`](Self::max_message_len)
    /// clamped to [`MESSAGE_MAX_LEN`].
    ///
    /// This is a UI hint only.  The server enforces its own limits regardless
    /// of what the client was told.
    pub fn effective_max_message_len(&self) -> usize {
        self.max_message_len.min(MESSAGE_MAX_LEN)
    }
}

// ---------------------------------------------------------------------------
// ContactServerPolicy
// ---------------------------------------------------------------------------

/// Server-side enforcement policy for contact form submissions.
///
/// Provide this via Leptos context in both the SSR renderer and the
/// server-function handler closures to enforce constraints server-side,
/// independent of whatever the client-side [`ContactFormOptions`] states.
///
/// # Why this is separate from `ContactFormOptions`
///
/// [`ContactFormOptions`] controls the UI (whether fields are shown, required,
/// or length-capped).  It is client-visible and cannot be trusted as a security
/// boundary.  `ContactServerPolicy` is the server-authoritative source of truth.
///
/// # Example
///
/// ```rust,ignore
/// use leptos::context::provide_context;
/// use leptos_hl_contact::config::ContactServerPolicy;
///
/// // In both SSR renderer and server-fn handler closures:
/// provide_context(ContactServerPolicy {
///     require_subject: true,
///     max_message_len: 2000,
/// });
/// ```
#[derive(Clone, Debug)]
pub struct ContactServerPolicy {
    /// Reject submissions where `subject` is absent or blank.
    /// Defaults to `false`.
    pub require_subject: bool,

    /// Maximum allowed length of the `message` field, in characters (Unicode
    /// scalar values).
    ///
    /// Values above [`MESSAGE_MAX_LEN`] are clamped to it: the policy can only
    /// tighten the validator's limit, never raise it.  Defaults to
    /// [`MESSAGE_MAX_LEN`].
    pub max_message_len: usize,
}

impl Default for ContactServerPolicy {
    fn default() -> Self {
        Self {
            require_subject: false,
            max_message_len: MESSAGE_MAX_LEN,
        }
    }
}

impl ContactServerPolicy {
    /// Effective message limit: [`max_message_len`](Self::max_message_len)
    /// clamped to [`MESSAGE_MAX_LEN`].
    pub fn effective_max_message_len(&self) -> usize {
        self.max_message_len.min(MESSAGE_MAX_LEN)
    }

    /// Apply the policy to a normalised input.
    ///
    /// An empty result means the input passes.  Both errors may be set at
    /// once.  Lengths are counted in characters, matching the validator and
    /// the component's `maxlength`.
    pub fn check(&self, input: &ContactInput) -> ContactFieldErrors {
        let mut errs = ContactFieldErrors::default();

        if self.require_subject && input.subject.is_none() {
            errs.subject = Some("Subject is required.".into());
        }

        let limit = self.effective_max_message_len();
        if input.message.chars().count() > limit {
            errs.message = Some(format!("Message must be at most {limit} characters."));
        }

        errs
    }
}

// ---------------------------------------------------------------------------
// ContactSuccessRedirect
// ---------------------------------------------------------------------------

/// The configured redirect path is not a safe site-relative path.
#[derive(Debug, thiserror::Error)]
#[error(
    "invalid redirect path: must be site-relative (start with '/', not '//', \
     no scheme, no control characters)"
)]
pub struct InvalidRedirectPath;

/// Where to send the visitor after a successful submission.
///
/// Provide this via Leptos context in the server-function handler closure to
/// send every successful submission to a page of your own, with or without
/// JavaScript.  When it is absent, behaviour is unchanged: a JavaScript
/// client shows the inline success message and a no-JavaScript client
/// reloads the form page.
///
/// The crate never calls a framework's redirect itself.  It holds a validated
/// path and an opaque executor supplied by the integrator; with Axum,
/// [`success_redirect`](crate::axum_helpers::success_redirect) builds both.
///
/// # Security
///
/// [`new`](Self::new) rejects anything that is not site-relative, so a
/// misconfiguration cannot turn the form into an open redirect.  The path is
/// fixed at startup and is never read from form input or a query parameter.
#[derive(Clone)]
pub struct ContactSuccessRedirect {
    path: String,
    redirect: Arc<dyn Fn(&str) + Send + Sync>,
}

impl std::fmt::Debug for ContactSuccessRedirect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContactSuccessRedirect")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl ContactSuccessRedirect {
    /// Build a redirect for `path`, executed by `redirect`.
    ///
    /// `path` must be site-relative: non-empty, starting with a single `/`,
    /// with no scheme, backslash, whitespace or ASCII control character.
    /// Query strings and fragments are allowed.
    ///
    /// # Errors
    ///
    /// [`InvalidRedirectPath`] when `path` fails those rules.
    pub fn new(
        path: impl Into<String>,
        redirect: impl Fn(&str) + Send + Sync + 'static,
    ) -> Result<Self, InvalidRedirectPath> {
        let path = path.into();

        let valid = path.starts_with('/')
            && !path.starts_with("//")
            && !path.contains('\\')
            && !path.contains("://")
            && !path.chars().any(|c| c.is_control() || c.is_whitespace());

        if !valid {
            return Err(InvalidRedirectPath);
        }

        Ok(Self {
            path,
            redirect: Arc::new(redirect),
        })
    }

    /// The validated destination.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Run the redirect.  Called by
    /// [`submit_contact`](crate::server::submit_contact) after a successful
    /// delivery.
    pub fn apply(&self) {
        (self.redirect)(&self.path);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
