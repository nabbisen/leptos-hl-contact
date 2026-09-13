// http.rs — Built-in challenge verifiers that call the vendors' siteverify
// endpoints (feature `challenge-http`).

use std::{future::Future, pin::Pin, time::Duration};

use serde::Deserialize;

use super::{ChallengeError, ChallengeOutcome, ChallengeVerifier};
use crate::config::ChallengeProvider;

/// How long a verification may take before it fails with
/// [`ChallengeError::Timeout`].
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const TURNSTILE_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
const HCAPTCHA_URL: &str = "https://api.hcaptcha.com/siteverify";
const RECAPTCHA_URL: &str = "https://www.google.com/recaptcha/api/siteverify";

/// Verifies a challenge token with the vendor's `siteverify` endpoint.
///
/// One `reqwest` client per verifier, reused for every call.  Each call is
/// capped at five seconds by default and never retried: a failure rejects
/// the submission with `challenge_unavailable`, which the visitor can retry.
///
/// Redirects are not followed.  A followed redirect would resend the request
/// body, secret included, to wherever the `Location` header points; no
/// vendor endpoint redirects, so a 3xx is `Unavailable` like any other
/// non-2xx answer.
///
/// # Security
///
/// The secret is sent only to the vendor, and `Debug` redacts it.  Neither the
/// secret nor the token appears in an error.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use leptos_hl_contact::{
///     ChallengeContext, ChallengePolicy, HttpChallengeVerifier, config::ChallengeProvider,
/// };
///
/// let verifier = HttpChallengeVerifier::new(
///     ChallengeProvider::Turnstile,
///     std::env::var("CHALLENGE_SECRET").expect("CHALLENGE_SECRET"),
/// );
/// let context = ChallengeContext {
///     verifier: Arc::new(verifier),
///     policy: ChallengePolicy::default(),
/// };
/// ```
pub struct HttpChallengeVerifier {
    provider: ChallengeProvider,
    secret: String,
    timeout: Duration,
    verify_url: Option<String>,
    client: reqwest::Client,
}

impl HttpChallengeVerifier {
    /// A verifier for `provider`, with a five-second timeout and the vendor's
    /// own endpoint.
    ///
    /// An empty `secret` is accepted here and reported as
    /// [`ChallengeError::Misconfigured`] on the first verification, so a
    /// missing environment variable fails closed rather than at startup.
    ///
    /// # Panics
    ///
    /// If the TLS backend cannot be initialised, as `reqwest::Client::new`
    /// does.  With rustls there is no system library that could be missing.
    pub fn new(provider: ChallengeProvider, secret: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            // Never resend the secret to a `Location` of someone else's choosing.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("the rustls TLS backend initialises");
        Self {
            provider,
            secret: secret.into(),
            timeout: DEFAULT_TIMEOUT,
            verify_url: None,
            client,
        }
    }

    /// Change the per-call time limit.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Send verifications to `url` instead of the vendor's endpoint — for a
    /// forwarding proxy that outbound traffic must pass through, or a test
    /// server.
    pub fn with_verify_url(mut self, url: impl Into<String>) -> Self {
        self.verify_url = Some(url.into());
        self
    }

    fn endpoint(&self) -> &str {
        self.verify_url.as_deref().unwrap_or(match self.provider {
            ChallengeProvider::Turnstile => TURNSTILE_URL,
            ChallengeProvider::HCaptcha => HCAPTCHA_URL,
            ChallengeProvider::RecaptchaV2 | ChallengeProvider::RecaptchaV3 { .. } => RECAPTCHA_URL,
        })
    }
}

impl std::fmt::Debug for HttpChallengeVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpChallengeVerifier")
            .field("provider", &self.provider)
            .field("secret", &"<redacted>")
            .field("timeout", &self.timeout)
            .field("verify_url", &self.verify_url)
            .finish()
    }
}

impl ChallengeVerifier for HttpChallengeVerifier {
    fn verify(
        &self,
        token: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + '_>> {
        let token = token.to_owned();
        Box::pin(async move {
            if self.secret.is_empty() {
                return Err(ChallengeError::Misconfigured("empty secret".into()));
            }
            let response = self
                .client
                .post(self.endpoint())
                .timeout(self.timeout)
                .form(&[
                    ("secret", self.secret.as_str()),
                    ("response", token.as_str()),
                ])
                .send()
                .await
                .map_err(transport_error)?;
            let status = response.status();
            if !status.is_success() {
                return Err(ChallengeError::Unavailable(format!("HTTP {status}")));
            }
            let body = response.bytes().await.map_err(transport_error)?;
            parse_response(&body)
        })
    }
}

/// A timeout is `Timeout`; anything else on the wire is `Unavailable`.  The
/// URL is stripped from the message: it could be a proxy address.
fn transport_error(error: reqwest::Error) -> ChallengeError {
    if error.is_timeout() {
        ChallengeError::Timeout
    } else {
        ChallengeError::Unavailable(error.without_url().to_string())
    }
}

/// The fields every vendor shares.  Turnstile and hCaptcha omit `score`;
/// reCAPTCHA v2 omits `score` and `action`; any vendor may omit
/// `error-codes`.
#[derive(Deserialize)]
struct SiteverifyResponse {
    success: Option<bool>,
    score: Option<f32>,
    action: Option<String>,
    #[serde(rename = "error-codes", default)]
    error_codes: Vec<String>,
}

/// Parse a `siteverify` body.  Anything that is not JSON with a boolean
/// `success` is `Unavailable`, never a failed challenge: the vendor did not
/// give an answer, so the submission must not be judged on it.
pub(crate) fn parse_response(body: &[u8]) -> Result<ChallengeOutcome, ChallengeError> {
    let malformed = || ChallengeError::Unavailable("malformed response".into());
    let parsed: SiteverifyResponse = serde_json::from_slice(body).map_err(|_| malformed())?;
    let passed = parsed.success.ok_or_else(malformed)?;
    Ok(ChallengeOutcome {
        passed,
        score: parsed.score,
        action: parsed.action,
        error_codes: parsed.error_codes,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
