// csrf.rs — Stateless HMAC-SHA256 CSRF token helper.
//
// Feature: `csrf`
//
// # Design
//
// Tokens are stateless (no session or database required).
// Each token embeds a Unix timestamp and a random nonce, both signed with
// HMAC-SHA256 using the application's secret key.
//
// Token format: `{timestamp_secs}|{nonce_hex}|{hmac_hex}`
//
// Verification checks:
//   1. The HMAC is valid (prevents forgery).
//   2. The token is not older than `token_ttl_secs`.
//
// # Usage in Axum
//
// 1. Build a `CsrfConfig` from an environment variable and wrap it in `Arc`.
// 2. Provide `Arc<CsrfConfig>` via `provide_context` in both the SSR renderer
//    closure and the server-function handler closure.
// 3. In the SSR renderer closure, also call `provide_context(generate_csrf_token(&config))`.
//    Leptos creates a fresh context per request, so each page render gets a
//    unique token.
// 4. `ContactForm` detects the `CsrfToken` context and embeds the token as a
//    hidden form field automatically.
// 5. `submit_contact` detects `Arc<CsrfConfig>` in context and verifies the
//    submitted token.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac, digest::KeyInit};
use rand::RngCore;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

// ---------------------------------------------------------------------------
// CsrfConfig
// ---------------------------------------------------------------------------

/// Configuration for the CSRF token helper.
///
/// Provide this as `Arc<CsrfConfig>` via Leptos context in **both**:
/// - the SSR renderer closure
/// - the server-function handler closure
///
/// # Security
///
/// Load `secret_key` from an environment variable.  It must be kept
/// server-side and never compiled into WASM.  Use at least 32 random bytes.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use leptos_hl_contact::csrf::CsrfConfig;
///
/// let csrf_config = Arc::new(CsrfConfig {
///     secret_key: std::env::var("CSRF_SECRET")
///         .expect("CSRF_SECRET must be set")
///         .into_bytes(),
///     token_ttl_secs: 3600,
/// });
/// ```
#[derive(Clone, Debug)]
pub struct CsrfConfig {
    /// HMAC signing key.  Must be kept server-side only.
    pub secret_key: Vec<u8>,

    /// Token validity window in seconds.  After this period, a token is
    /// considered expired.  Defaults to 3 600 (one hour).
    pub token_ttl_secs: u64,
}

impl CsrfConfig {
    /// Create a config with a one-hour TTL.
    pub fn new(secret_key: Vec<u8>) -> Self {
        Self {
            secret_key,
            token_ttl_secs: 3600,
        }
    }
}

// ---------------------------------------------------------------------------
// CsrfToken
// ---------------------------------------------------------------------------

/// A single-use CSRF token value, ready to embed in an HTML form.
///
/// Provide this via Leptos context in the **SSR renderer closure only** (not
/// in the server-function handler closure — each context is request-scoped).
/// `ContactForm` reads this context and inserts the value into a hidden
/// `<input name="csrf_token">` field automatically.
///
/// # Example
///
/// ```rust,ignore
/// use leptos_hl_contact::csrf::{CsrfConfig, CsrfToken, generate_csrf_token};
/// use std::sync::Arc;
///
/// // In your SSR renderer context closure:
/// let token: CsrfToken = generate_csrf_token(&csrf_config);
/// leptos::context::provide_context(token);
/// ```
#[derive(Clone, Debug)]
pub struct CsrfToken(pub String);

// ---------------------------------------------------------------------------
// generate_csrf_token
// ---------------------------------------------------------------------------

/// Generate a fresh, signed CSRF token.
///
/// The token encodes the current Unix timestamp and a 16-byte random nonce,
/// signed with HMAC-SHA256 using `config.secret_key`.
///
/// Call this once per SSR render and provide the result via Leptos context so
/// that `ContactForm` can embed it in the hidden form field.
///
/// # Panics
///
/// Panics if the system clock is before the Unix epoch (i.e. never in
/// practice).
pub fn generate_csrf_token(config: &CsrfConfig) -> CsrfToken {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_secs();

    let mut nonce_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce_hex = hex::encode(nonce_bytes);

    let signed_payload = format!("{timestamp}|{nonce_hex}");
    let signature = sign(&signed_payload, &config.secret_key);

    CsrfToken(format!("{signed_payload}|{signature}"))
}

// ---------------------------------------------------------------------------
// verify_csrf_token
// ---------------------------------------------------------------------------

/// Verify a CSRF token submitted with a form.
///
/// Returns `true` when:
/// 1. The token has the correct format.
/// 2. The HMAC signature is valid.
/// 3. The token is not older than `config.token_ttl_secs`.
///
/// Returns `false` otherwise.  Callers should treat `false` as an
/// `Unauthorized` or `Forbidden` response.
///
/// # Security
///
/// This function uses a constant-time comparison for the HMAC, so it is safe
/// against timing attacks.
pub fn verify_csrf_token(token: &str, config: &CsrfConfig) -> bool {
    // Format: "{timestamp}|{nonce_hex}|{hmac_hex}"
    let parts: Vec<&str> = token.splitn(3, '|').collect();
    if parts.len() != 3 {
        return false;
    }

    let timestamp_str = parts[0];
    let nonce_hex = parts[1];
    let submitted_sig = parts[2];

    // Parse timestamp
    let Ok(timestamp) = timestamp_str.parse::<u64>() else {
        return false;
    };

    // Verify nonce is valid hex
    if hex::decode(nonce_hex).is_err() {
        return false;
    }

    // Check TTL
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if now.saturating_sub(timestamp) > config.token_ttl_secs {
        tracing::debug!(timestamp, now, "CSRF token expired");
        return false;
    }

    // Verify HMAC (constant-time comparison via `verify_slice`)
    let payload = format!("{timestamp_str}|{nonce_hex}");
    let expected = sign(&payload, &config.secret_key);

    // Constant-time hex comparison
    constant_time_eq(submitted_sig, &expected)
}

// ---------------------------------------------------------------------------
// CsrfConfigContext
// ---------------------------------------------------------------------------

/// Type alias for the Leptos context used to inject CSRF configuration.
///
/// Provide this in **both** the SSR renderer and the server-function handler
/// closures so that token verification works for every `submit_contact` call.
pub type CsrfConfigContext = Arc<CsrfConfig>;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn sign(payload: &str, key: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Constant-time string comparison to prevent timing attacks.
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
