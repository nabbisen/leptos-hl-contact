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
    /// The honeypot's wrapper `<div>`.
    ///
    /// Needed when [`ContactFormOptions::honeypot_inline_style`] is `false`:
    /// the site's CSS must then hide the wrapper through this class.  Optional
    /// otherwise.  When empty, no `class` attribute is rendered.
    #[serde(default)]
    pub honeypot: String,
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
    /// Delivery did not finish within its deadline.  The message may have been
    /// sent, so the text should not say it failed.
    pub delivery_timeout: String,
    /// A challenge is configured but the submission carried no token.
    pub challenge_required: String,
    /// The challenge did not pass.
    pub challenge_failed: String,
    /// The challenge vendor could not be reached.
    pub challenge_unavailable: String,
    /// Shown inside `<noscript>` when the challenge needs JavaScript.
    pub challenge_requires_js: String,
    /// A `ContactFilter` refused the submission.  Deliberately generic, and
    /// distinct from every token and challenge label.
    pub rejected: String,
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
            delivery_timeout: "Sending took too long. Your message may have been sent — please wait a few minutes before trying again.".into(),
            challenge_required: "Please complete the security check.".into(),
            challenge_failed: "The security check did not pass. Please try again.".into(),
            challenge_unavailable:
                "The security check is unavailable right now. Please try again later.".into(),
            challenge_requires_js: "This form needs JavaScript to verify you are human.".into(),
            rejected: "Your message could not be accepted.".into(),
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
            C::DeliveryTimeout => self.delivery_timeout.clone(),
            C::ChallengeRequired => self.challenge_required.clone(),
            C::ChallengeFailed => self.challenge_failed.clone(),
            C::ChallengeUnavailable => self.challenge_unavailable.clone(),
            C::Rejected => self.rejected.clone(),
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
// NoJsPolicy
// ---------------------------------------------------------------------------

/// What the server does with a submission that carries no challenge token.
///
/// Shared by the component, which renders a `<noscript>` explanation under
/// [`Reject`](Self::Reject), and by the server's challenge policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoJsPolicy {
    /// Reject with `challenge_required`.  Recommended.
    #[default]
    Reject,
    /// Accept, relying on the honeypot alone.
    ///
    /// A server cannot distinguish a no-JS browser from a bot that omits the
    /// token, so this policy makes the challenge advisory.  Use [`Reject`](Self::Reject)
    /// unless no-JS visitors matter more than bots.
    AcceptWithHoneypotOnly,
}

// ---------------------------------------------------------------------------
// Challenge widget
// ---------------------------------------------------------------------------

/// Which challenge vendor renders the widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChallengeProvider {
    /// Cloudflare Turnstile.
    Turnstile,
    /// hCaptcha.
    HCaptcha,
    /// Google reCAPTCHA v2, checkbox or invisible.
    RecaptchaV2,
    /// Google reCAPTCHA v3, which has no visible widget.  A token is fetched
    /// on every submit, because v3 tokens are single-use.
    RecaptchaV3 {
        /// The action name sent with each token, `[A-Za-z0-9_-]+`.  The server
        /// can require it through `ChallengePolicy::expected_action`.
        action: String,
    },
}

/// The widget's colour scheme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChallengeTheme {
    /// Follow the visitor's preference where the vendor supports it.
    #[default]
    Auto,
    /// Light.
    Light,
    /// Dark.
    Dark,
}

impl ChallengeTheme {
    /// Turnstile's `data-theme`, which has an `auto` value.
    pub(crate) fn turnstile_value(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// hCaptcha's and reCAPTCHA's `data-theme`.  They have no `auto`, so it
    /// is omitted and the vendor default applies.
    pub(crate) fn explicit_value(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Light => Some("light"),
            Self::Dark => Some("dark"),
        }
    }
}

/// A [`ChallengeWidget`] value that would not be safe to put in the page.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid challenge configuration: {0}")]
pub struct InvalidChallengeConfig(pub &'static str);

/// A CAPTCHA widget rendered inside [`ContactForm`](crate::components::ContactForm).
///
/// Build it with [`new`](Self::new) and the `with_*` methods.  Every value
/// that reaches the page is validated there, which is why the fields are not
/// public: the site key and the reCAPTCHA v3 action are embedded in an inline
/// script, and only a validated value is safe to embed.
///
/// The widget only collects a token.  The server verifies it through
/// `ChallengeContext`; without one, a submission carrying a token is rejected
/// with `not_configured`.
///
/// # Example
///
/// ```rust
/// use leptos_hl_contact::config::{ChallengeProvider, ChallengeTheme, ChallengeWidget};
///
/// let widget = ChallengeWidget::new(ChallengeProvider::Turnstile, "1x00000000000000000000AA")
///     .unwrap()
///     .with_theme(ChallengeTheme::Dark);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengeWidget {
    pub(crate) provider: ChallengeProvider,
    pub(crate) site_key: String,
    pub(crate) theme: ChallengeTheme,
    pub(crate) language: Option<String>,
    pub(crate) load_script: bool,
    pub(crate) script_nonce: Option<String>,
    pub(crate) no_js: NoJsPolicy,
}

impl ChallengeWidget {
    /// A widget with the vendor's script loaded by the component, the `Auto`
    /// theme, the vendor's own language detection, and `NoJsPolicy::Reject`.
    ///
    /// Validates `site_key`, and the reCAPTCHA v3 `action`, against
    /// `[A-Za-z0-9_-]+`.
    pub fn new(
        provider: ChallengeProvider,
        site_key: impl Into<String>,
    ) -> Result<Self, InvalidChallengeConfig> {
        let site_key = site_key.into();
        if !is_key_charset(&site_key) {
            return Err(InvalidChallengeConfig(
                "site_key must be non-empty and use only [A-Za-z0-9_-]",
            ));
        }
        if let ChallengeProvider::RecaptchaV3 { action } = &provider
            && !is_key_charset(action)
        {
            return Err(InvalidChallengeConfig(
                "the reCAPTCHA v3 action must be non-empty and use only [A-Za-z0-9_-]",
            ));
        }
        Ok(Self {
            provider,
            site_key,
            theme: ChallengeTheme::Auto,
            language: None,
            load_script: true,
            script_nonce: None,
            no_js: NoJsPolicy::Reject,
        })
    }

    /// Set the colour scheme.
    pub fn with_theme(mut self, theme: ChallengeTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the widget language, a BCP 47 tag such as `en` or `pt-BR`.
    pub fn with_language(
        mut self,
        language: impl Into<String>,
    ) -> Result<Self, InvalidChallengeConfig> {
        let language = language.into();
        if !is_language_tag(&language) {
            return Err(InvalidChallengeConfig(
                "language must look like a BCP 47 tag, such as en or pt-BR",
            ));
        }
        self.language = Some(language);
        Ok(self)
    }

    /// Do not emit the vendor's `<script>`: the page loads it itself.
    ///
    /// reCAPTCHA v3 still gets its inline submit script, which it cannot work
    /// without.
    pub fn without_script(mut self) -> Self {
        self.load_script = false;
        self
    }

    /// Put a Content-Security-Policy nonce on the script tags.
    ///
    /// The nonce follows CSP Level 3's grammar: base64 or base64url
    /// characters (`A–Z a–z 0–9 + / - _`), then at most two `=` of padding.
    /// The values `leptos::nonce::use_nonce()` returns, which are base64url,
    /// are accepted.
    ///
    /// # Errors
    ///
    /// [`InvalidChallengeConfig`] for any other value: an empty string, one
    /// with a quote, a space or another character outside that set, or one
    /// with `=` anywhere but the end.
    pub fn with_script_nonce(
        mut self,
        nonce: impl Into<String>,
    ) -> Result<Self, InvalidChallengeConfig> {
        let nonce = nonce.into();
        if !is_csp_nonce(&nonce) {
            return Err(InvalidChallengeConfig(
                "script_nonce must be a CSP nonce (base64 or base64url: [A-Za-z0-9+/_-], up to two trailing '=')",
            ));
        }
        self.script_nonce = Some(nonce);
        Ok(self)
    }

    /// Set what the component shows without JavaScript.  Use the same value
    /// as the server's `ChallengePolicy::no_js`.
    pub fn with_no_js(mut self, no_js: NoJsPolicy) -> Self {
        self.no_js = no_js;
        self
    }

    /// The vendor script URL, with the language where the vendor takes it
    /// from the URL.
    pub(crate) fn script_src(&self) -> String {
        let hl = |sep: char| {
            self.language
                .as_ref()
                .map(|l| format!("{sep}hl={l}"))
                .unwrap_or_default()
        };
        match &self.provider {
            ChallengeProvider::Turnstile => {
                "https://challenges.cloudflare.com/turnstile/v0/api.js".to_owned()
            }
            ChallengeProvider::HCaptcha => format!("https://js.hcaptcha.com/1/api.js{}", hl('?')),
            ChallengeProvider::RecaptchaV2 => {
                format!("https://www.google.com/recaptcha/api.js{}", hl('?'))
            }
            ChallengeProvider::RecaptchaV3 { .. } => format!(
                "https://www.google.com/recaptcha/api.js?render={}{}",
                self.site_key,
                hl('&')
            ),
        }
    }
}

/// `[A-Za-z0-9_-]+`
fn is_key_charset(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// `[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8})*`
fn is_language_tag(s: &str) -> bool {
    let mut parts = s.split('-');
    let primary_ok = parts
        .next()
        .is_some_and(|p| (2..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphabetic()));
    primary_ok
        && parts.all(|p| (2..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// A CSP Level 3 nonce, `base64-value` in the `nonce-source` grammar:
/// `[A-Za-z0-9+/_-]+` followed by at most two `=`.  Base64url nonces, such as
/// the ones Leptos generates, use `-` and `_`.
fn is_csp_nonce(s: &str) -> bool {
    let value = s.trim_end_matches('=');
    let padding = s.len() - value.len();
    !value.is_empty()
        && padding <= 2
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'-' | b'_'))
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

    /// Set this when the server issues form tokens.  It lets the browser
    /// fetch a token for a form reached by client-side navigation, and fetch
    /// a replacement before a token left open expires.  Client-side only.
    ///
    /// `ttl_secs - 60` is the right value — `Some(3540)` for the default
    /// one-hour TTL — because the browser does not know the server's TTL.
    /// Values of 60 or less still fetch a missing token but never refresh:
    /// they mean a TTL of two minutes or less, where a refresh would race the
    /// expiry.
    ///
    /// `None`, the default, means the browser never calls the token endpoint.
    /// A server-rendered token is still submitted as rendered, but a form
    /// reached by client-side navigation submits an empty token and shows the
    /// token-invalid message.
    pub token_refresh_secs: Option<u64>,

    /// Whether the honeypot's wrapper carries the inline style that moves it
    /// off screen.  Defaults to `true`.
    ///
    /// A Content Security Policy without `'unsafe-inline'` blocks that
    /// attribute.  Set this to `false` under such a policy: the crate then
    /// renders **no** `style` attribute, and the site must hide the wrapper
    /// through [`ContactFormClasses::honeypot`] with a rule such as:
    ///
    /// ```css
    /// .your-honeypot { position: absolute; left: -9999px; width: 1px; height: 1px; overflow: hidden; }
    /// ```
    ///
    /// With `false` and no such rule the field is visible, and a visitor who
    /// fills it in is silently discarded as a bot.  `aria-hidden`,
    /// `tabindex="-1"` and `autocomplete="off"` are rendered in both modes.
    #[serde(default = "default_true")]
    pub honeypot_inline_style: bool,
}

/// Serde default for fields added after 0.6 whose default is `true`.
fn default_true() -> bool {
    true
}

impl Default for ContactFormOptions {
    fn default() -> Self {
        Self {
            show_subject: true,
            require_subject: false,
            max_message_len: MESSAGE_MAX_LEN,
            focus_first_error: true,
            token_refresh_secs: None,
            honeypot_inline_style: true,
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
