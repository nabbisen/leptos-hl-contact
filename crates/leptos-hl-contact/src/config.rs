// config.rs — Configuration types for ContactForm.
//
// The prop types are serialisable so they can cross the SSR/hydrate boundary.
// They must **never** contain secrets.  `ContactSuccessRedirect` is
// server-side configuration and is deliberately not serialisable.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    error::{ContactField, ContactFieldErrors, FieldError, FieldErrorCode},
    model::ContactInput,
    model::MESSAGE_MAX_LEN,
};

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
// ContactErrorLabels
// ---------------------------------------------------------------------------

/// Text for every error the server can report.
///
/// The server sends codes, never sentences, so translating these translates
/// the whole form.  `length` may contain the placeholders `{min}` and
/// `{max}`, which are replaced with decimal numbers; there is no format
/// syntax beyond those two.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactErrorLabels {
    /// A required field was absent or blank.
    pub required: String,
    /// Length outside the permitted range.  May use `{min}` and `{max}`.
    pub length: String,
    /// The email field's value is not a valid address.
    pub format_email: String,
    /// Any other syntactically invalid value.
    pub format: String,
    /// CR or LF in a field that forbids them.
    pub line_breaks: String,
    /// The form token was missing, malformed, or expired.
    pub token_invalid: String,
    /// The form was submitted too soon after loading.  Retryable.
    pub too_fast: String,
    /// A required context value is absent on the server.
    pub not_configured: String,
    /// The delivery backend refused or failed.
    pub delivery_failed: String,
}

impl Default for ContactErrorLabels {
    fn default() -> Self {
        Self {
            required: "This field is required.".into(),
            length: "Must be between {min} and {max} characters.".into(),
            format_email: "Enter a valid email address.".into(),
            format: "Invalid value.".into(),
            line_breaks: "Line breaks are not allowed here.".into(),
            token_invalid: "Your session token expired. Please reload the page and try again."
                .into(),
            too_fast: "Please wait a moment and try again.".into(),
            not_configured: "This form is not available right now.".into(),
            delivery_failed: "Failed to send message. Please try again later.".into(),
        }
    }
}

impl ContactErrorLabels {
    /// Render one field's error.
    ///
    /// `field` selects between [`format_email`](Self::format_email) and
    /// [`format`](Self::format); every other code ignores it.
    pub fn field_text(&self, field: ContactField, err: &FieldError) -> String {
        match err {
            FieldError::Text(s) => s.clone(),
            FieldError::Code(code) => match code {
                FieldErrorCode::Required => self.required.clone(),
                FieldErrorCode::Length { min, max } => self
                    .length
                    .replace("{min}", &min.to_string())
                    .replace("{max}", &max.to_string()),
                FieldErrorCode::Format => {
                    if field == ContactField::Email {
                        self.format_email.clone()
                    } else {
                        self.format.clone()
                    }
                }
                FieldErrorCode::LineBreaks => self.line_breaks.clone(),
            },
        }
    }

    /// Render a whole-submission error.
    ///
    /// [`Unexpected`](crate::error::ContactErrorCode::Unexpected) renders as
    /// [`delivery_failed`](Self::delivery_failed): from the visitor's side
    /// the message did not go, and the cause belongs in the logs.
    pub fn code_text(&self, code: crate::error::ContactErrorCode) -> String {
        use crate::error::ContactErrorCode as C;
        match code {
            C::TokenInvalid => self.token_invalid.clone(),
            C::TooFast => self.too_fast.clone(),
            C::NotConfigured => self.not_configured.clone(),
            C::DeliveryFailed | C::Unexpected => self.delivery_failed.clone(),
        }
    }
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

    /// Text for every error the server can report.
    pub errors: ContactErrorLabels,
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
            errors: ContactErrorLabels::default(),
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

    /// Seconds after a form token's issue time at which the browser fetches
    /// a replacement, so a form left open never submits an expired token.
    /// Client-side only.  Defaults to `Some(3540)`.
    ///
    /// The browser does not know the server's TTL, so this **should be
    /// `ttl_secs - 60`**.  Values of 60 or less schedule nothing — they
    /// correspond to a TTL of two minutes or less, where a refresh would race
    /// the expiry.  `None` disables the refresh.  Has no effect without the
    /// form token.
    pub token_refresh_secs: Option<u64>,
}

impl Default for ContactFormOptions {
    fn default() -> Self {
        Self {
            show_subject: true,
            require_subject: false,
            max_message_len: MESSAGE_MAX_LEN,
            focus_first_error: true,
            token_refresh_secs: Some(3540),
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
/// Provide this via Leptos context in the closure passed to
/// `leptos_routes_with_context` to enforce constraints server-side,
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
/// // In the context closure:
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
            errs.subject = Some(FieldError::Code(FieldErrorCode::Required));
        }

        let limit = self.effective_max_message_len();
        if input.message.chars().count() > limit {
            errs.message = Some(FieldError::Code(FieldErrorCode::Length {
                min: 1,
                max: limit,
            }));
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
/// Provide this via Leptos context in the closure passed to
/// `leptos_routes_with_context` to send every successful submission to a page
/// of your own, with or without JavaScript.  When it is absent, behaviour is unchanged: a JavaScript
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
