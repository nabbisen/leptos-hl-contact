// config.rs — Client-visible configuration types for ContactForm.
//
// All types in this module are serialisable so they can cross the SSR/hydrate
// boundary as component props.  They must **never** contain secrets.

use serde::{Deserialize, Serialize};

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
    /// Must not exceed the server-side hard limit of 4 000.  Defaults to
    /// `4000`.
    pub max_message_len: usize,
}

impl Default for ContactFormOptions {
    fn default() -> Self {
        Self {
            show_subject: true,
            require_subject: false,
            max_message_len: 4000,
        }
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

    /// Maximum allowed length of the `message` field in characters.
    /// Must not exceed the hard validation limit of 4 000.
    /// Defaults to `4000`.
    pub max_message_len: usize,
}

impl Default for ContactServerPolicy {
    fn default() -> Self {
        Self {
            require_subject: false,
            max_message_len: 4000,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
