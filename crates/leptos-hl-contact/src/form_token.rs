// form_token.rs — Stateless HMAC-SHA256 form token (feature `form-token`).
//
// # Design
//
// Tokens are stateless: no session store, no database.  Each token embeds a
// Unix timestamp and a random nonce, both signed with HMAC-SHA256 using the
// application's secret key.
//
// Token format: `{timestamp_secs}|{nonce_hex}|{hmac_hex}`
//
// # Usage in Axum
//
// 1. Build a `FormTokenConfig` from an environment variable and wrap it in
//    `Arc`.
// 2. Provide `Arc<FormTokenConfig>` via `provide_context` in the closure
//    passed to `leptos_routes_with_context`; that closure serves page renders
//    and server functions alike.
// 3. In the same closure call `provide_context(issue_form_token(&config))`.
//    Leptos creates a fresh context per request, so each page render gets a
//    unique token.
// 4. `ContactForm` detects the `FormToken` context and embeds the token as a
//    hidden form field automatically.
// 5. `submit_contact` detects `Arc<FormTokenConfig>` in context and verifies
//    the submitted token.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac, digest::KeyInit};
use rand::RngCore;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Tolerance for a token whose timestamp is slightly ahead of this server.
const ALLOWED_FUTURE_SKEW_SECS: u64 = 60;

// ---------------------------------------------------------------------------
// Binding
// ---------------------------------------------------------------------------

/// What, if anything, the token is tied to besides the secret.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Binding {
    /// The token proves only that the bearer fetched a page from this server
    /// within the TTL.  It is not tied to the visitor's browser.
    #[default]
    None,
    /// The token's nonce must also arrive in a cookie, which makes a
    /// cross-site submission fail.
    Cookie,
}

// ---------------------------------------------------------------------------
// FormTokenError
// ---------------------------------------------------------------------------

/// Why [`verify_form_token`] rejected a token.
///
/// Only [`TooYoung`](Self::TooYoung) is worth distinguishing to the visitor:
/// it is retryable, because the same token becomes valid moments later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormTokenError {
    /// Not three `|`-separated parts, or the nonce is not hex.
    Malformed,
    /// The HMAC does not match.
    BadSignature,
    /// Older than `ttl_secs`.
    Expired,
    /// Timestamped further ahead than the clock-skew tolerance.
    FromFuture,
    /// Submitted sooner than `min_age_secs` after it was issued.
    TooYoung,
    /// `Binding::Cookie` is configured but no bound value arrived.
    BindingMissing,
    /// The bound value does not match the token's nonce.
    BindingMismatch,
}

// ---------------------------------------------------------------------------
// FormTokenConfig
// ---------------------------------------------------------------------------

/// Configuration for the form token (feature `form-token`).
///
/// Provide this as `Arc<FormTokenConfig>` via Leptos context in the closure
/// passed to `leptos_routes_with_context`.
///
/// # Security
///
/// Load `secret_key` from an environment variable.  It must stay server-side
/// and never be compiled into WASM.  Use at least 32 random bytes.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use leptos_hl_contact::form_token::FormTokenConfig;
///
/// let config = Arc::new(
///     FormTokenConfig::new(
///         std::env::var("FORM_TOKEN_SECRET")
///             .expect("FORM_TOKEN_SECRET must be set")
///             .into_bytes(),
///     )
///     .with_ttl(3600)
///     .with_min_age(2),
/// );
/// ```
#[derive(Clone)]
pub struct FormTokenConfig {
    /// HMAC signing key.  Server-side only.
    pub secret_key: Vec<u8>,

    /// How long a token stays valid, in seconds.  Defaults to 3 600.
    pub ttl_secs: u64,

    /// How long after issue a token must be before it is accepted, in
    /// seconds.  Defaults to 2; `0` disables the check.
    ///
    /// A person needs a moment to read and fill a form; a script does not.
    /// The delay costs a fast human one retry and costs a script every
    /// submission.
    pub min_age_secs: u64,

    /// What the token is tied to besides the secret.  Defaults to
    /// [`Binding::None`].
    pub binding: Binding,
}

impl std::fmt::Debug for FormTokenConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormTokenConfig")
            .field("secret_key", &"<redacted>")
            .field("ttl_secs", &self.ttl_secs)
            .field("min_age_secs", &self.min_age_secs)
            .field("binding", &self.binding)
            .finish()
    }
}

impl FormTokenConfig {
    /// A config with a one-hour TTL, a two-second minimum age and no binding.
    pub fn new(secret_key: Vec<u8>) -> Self {
        Self {
            secret_key,
            ttl_secs: 3600,
            min_age_secs: 2,
            binding: Binding::None,
        }
    }

    /// Set how long a token stays valid.
    pub fn with_ttl(mut self, secs: u64) -> Self {
        self.ttl_secs = secs;
        self
    }

    /// Set the minimum age; `0` disables the check.
    pub fn with_min_age(mut self, secs: u64) -> Self {
        self.min_age_secs = secs;
        self
    }

    /// Set what the token is tied to.
    pub fn with_binding(mut self, binding: Binding) -> Self {
        self.binding = binding;
        self
    }
}

// ---------------------------------------------------------------------------
// FormToken
// ---------------------------------------------------------------------------

/// A signed, time-limited token value, ready to embed in an HTML form.
///
/// Provide this via Leptos context in the context closure.  It is read only
/// by page renders — `submit_contact` verifies the token the form submitted —
/// but it is harmless to generate on every request.  `ContactForm` reads this
/// context and inserts the value into a hidden `<input name="form_token">`
/// field automatically.
#[derive(Clone, Debug)]
pub struct FormToken(pub String);

// ---------------------------------------------------------------------------
// issue_form_token
// ---------------------------------------------------------------------------

/// Issue a fresh, signed token.
///
/// The token encodes the current Unix timestamp and a 16-byte random nonce,
/// signed with HMAC-SHA256 using `config.secret_key`.
///
/// Call this once per page render and provide the result via Leptos context
/// so that `ContactForm` can embed it in the hidden field.
///
/// # Panics
///
/// Panics if the system clock is before the Unix epoch.
pub fn issue_form_token(config: &FormTokenConfig) -> FormToken {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs();

    let mut nonce_bytes = [0u8; 16];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    let signed_payload = format!("{timestamp}|{nonce_hex}");
    let signature = sign(&signed_payload, &config.secret_key);

    FormToken(format!("{signed_payload}|{signature}"))
}

// ---------------------------------------------------------------------------
// verify_form_token
// ---------------------------------------------------------------------------

/// Verify a token submitted with a form.
///
/// `bound_value` is the value the binding arrived in — the cookie, for
/// [`Binding::Cookie`].  Pass `None` when the configured binding is
/// [`Binding::None`], where it is ignored.
///
/// Checks run in this order: format, timestamp, future skew, expiry, minimum
/// age, signature, binding.
///
/// # Security
///
/// The signature comparison is constant-time.
pub fn verify_form_token(
    token: &str,
    bound_value: Option<&str>,
    config: &FormTokenConfig,
) -> Result<(), FormTokenError> {
    // Format: "{timestamp}|{nonce_hex}|{hmac_hex}"
    let parts: Vec<&str> = token.splitn(3, '|').collect();
    if parts.len() != 3 {
        return Err(FormTokenError::Malformed);
    }
    let (timestamp_str, nonce_hex, submitted_sig) = (parts[0], parts[1], parts[2]);

    let Ok(timestamp) = timestamp_str.parse::<u64>() else {
        return Err(FormTokenError::Malformed);
    };
    if hex::decode(nonce_hex).is_err() {
        return Err(FormTokenError::Malformed);
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if timestamp > now.saturating_add(ALLOWED_FUTURE_SKEW_SECS) {
        return Err(FormTokenError::FromFuture);
    }

    let age = now.saturating_sub(timestamp);
    if age > config.ttl_secs {
        return Err(FormTokenError::Expired);
    }
    if age < config.min_age_secs {
        return Err(FormTokenError::TooYoung);
    }

    let payload = format!("{timestamp_str}|{nonce_hex}");
    let expected = sign(&payload, &config.secret_key);
    if !constant_time_eq(submitted_sig, &expected) {
        return Err(FormTokenError::BadSignature);
    }

    if config.binding == Binding::Cookie {
        let Some(bound) = bound_value else {
            return Err(FormTokenError::BindingMissing);
        };
        if !constant_time_eq(bound, nonce_hex) {
            return Err(FormTokenError::BindingMismatch);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// FormTokenContext
// ---------------------------------------------------------------------------

/// Type alias for the Leptos context used to inject the token configuration.
///
/// Provide this in the closure passed to `leptos_routes_with_context` so that
/// verification works for every `submit_contact` call.
pub type FormTokenContext = Arc<FormTokenConfig>;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn sign(payload: &str, key: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time string comparison, to keep signature checking free of a
/// timing side channel.
fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
