// challenge.rs — Challenge (CAPTCHA) verification: trait, policy, and the
// decision table that `submit_contact` follows.
//
// No HTTP here.  A verifier is anything that turns a vendor token into a
// `ChallengeOutcome`; the built-in HTTP verifiers arrive separately.  The two
// decision functions are pure so every row of the table is testable with a
// mock.

use std::{future::Future, net::IpAddr, pin::Pin, sync::Arc};

use leptos::server_fn::error::ServerFnError;

use crate::{config::NoJsPolicy, error::ContactErrorCode};

// ---------------------------------------------------------------------------
// Verifier
// ---------------------------------------------------------------------------

/// What a challenge vendor said about a token.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChallengeOutcome {
    /// The vendor accepted the token.
    pub passed: bool,
    /// reCAPTCHA v3's score, from 0.0 to 1.0.  `None` for other vendors.
    pub score: Option<f32>,
    /// reCAPTCHA v3's action name.  `None` for other vendors.
    pub action: Option<String>,
    /// The vendor's error codes, for logging.
    pub error_codes: Vec<String>,
}

/// Why a verifier could not produce an outcome.
///
/// Every variant rejects the submission with `challenge_unavailable`: the
/// check fails closed.
#[derive(Debug, thiserror::Error)]
pub enum ChallengeError {
    /// The vendor did not answer in time.
    #[error("timeout")]
    Timeout,
    /// The vendor could not be reached or answered with something unusable.
    #[error("unavailable: {0}")]
    Unavailable(String),
    /// The verifier itself is set up wrongly, such as a missing secret.
    #[error("misconfigured: {0}")]
    Misconfigured(String),
}

/// The future [`ChallengeVerifier::verify`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(not(all(target_arch = "wasm32", feature = "ssr")))]
pub type VerifyFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + Send + 'a>>;

/// The future [`ChallengeVerifier::verify`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(all(target_arch = "wasm32", feature = "ssr"))]
pub type VerifyFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ChallengeOutcome, ChallengeError>> + 'a>>;

/// Checks a challenge token with its vendor.
///
/// Implementations own whatever they need from `token` before returning the
/// future: the future may borrow only `self`.  Never log the token.
pub trait ChallengeVerifier: Send + Sync + 'static {
    /// Ask the vendor about `token`.
    fn verify(&self, token: &str) -> VerifyFuture<'_>;

    /// Ask the vendor about a request: the token and, when the site provides
    /// [`ChallengeClientIp`], the visitor's IP.  `submit_contact` calls this
    /// method.
    ///
    /// The default ignores everything but the token and calls
    /// [`verify`](Self::verify), so an existing verifier needs no change.
    /// Override it to use the IP.  As with `verify`, the future may borrow
    /// only `self`, and neither the token nor the IP may be logged.
    fn verify_request(&self, request: &ChallengeRequest<'_>) -> VerifyFuture<'_> {
        self.verify(request.token)
    }
}

/// What a verifier is asked about (RFC 011 D5).
///
/// Build it with [`ChallengeRequest::new`]: the struct is `#[non_exhaustive]`,
/// so fields can be added without breaking verifiers.
///
/// `Debug` redacts both the token and the IP; it shows only whether an IP is
/// present.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct ChallengeRequest<'a> {
    /// The vendor token from the form.
    pub token: &'a str,
    /// The visitor's IP, when the site provides [`ChallengeClientIp`].
    pub remote_ip: Option<IpAddr>,
}

impl<'a> ChallengeRequest<'a> {
    /// A request for `token`, without an IP.
    pub fn new(token: &'a str) -> Self {
        Self {
            token,
            remote_ip: None,
        }
    }

    /// The same request, with the visitor's IP.
    pub fn with_remote_ip(self, ip: IpAddr) -> Self {
        Self {
            remote_ip: Some(ip),
            ..self
        }
    }
}

impl std::fmt::Debug for ChallengeRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeRequest")
            .field("token", &"<redacted>")
            .field("remote_ip", &self.remote_ip.map(|_| "<redacted>"))
            .finish()
    }
}

/// The visitor's IP for this request, provided by the site (RFC 011 D5).
///
/// When it is in context, `submit_contact` passes it to
/// [`ChallengeVerifier::verify_request`], and `HttpChallengeVerifier` sends it
/// to the vendor as `remoteip`.
///
/// **The crate never reads a header for it.**  Which header can be trusted
/// depends on the proxy in front of the site, and a guess would accept a
/// spoofed `X-Forwarded-For`.  Take the IP from what your proxy guarantees:
/// `CF-Connecting-IP` behind Cloudflare, the peer address with no proxy.
///
/// **The IP is personal data.**  The crate never logs it, and a verifier must
/// not either.  `Debug` redacts it.
///
/// # Example
///
/// Behind Cloudflare, in the context closure passed to
/// `leptos_routes_with_context`, where the request's `Parts` are in context:
///
/// ```rust,ignore
/// use axum::http::request::Parts;
/// use leptos::prelude::*;
/// use leptos_hl_contact::ChallengeClientIp;
///
/// let context = move || {
///     // … the other context values …
///     let ip = use_context::<Parts>().and_then(|parts| {
///         parts.headers.get("CF-Connecting-IP")?.to_str().ok()?.parse().ok()
///     });
///     if let Some(ip) = ip {
///         provide_context(ChallengeClientIp(ip));
///     }
/// };
/// ```
#[derive(Clone, Copy)]
pub struct ChallengeClientIp(pub IpAddr);

impl std::fmt::Debug for ChallengeClientIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ChallengeClientIp")
            .field(&"<redacted>")
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Policy and context
// ---------------------------------------------------------------------------

/// How strict the server is about challenge results.
#[derive(Clone, Debug, PartialEq)]
pub struct ChallengePolicy {
    /// What to do when a submission carries no token.  Defaults to
    /// [`NoJsPolicy::Reject`].
    pub no_js: NoJsPolicy,
    /// The lowest reCAPTCHA v3 score that passes.  Defaults to 0.5.  Ignored
    /// when the outcome has no score.
    pub min_score: f32,
    /// The reCAPTCHA v3 action the token must carry.  `None` accepts any.
    pub expected_action: Option<String>,
}

impl Default for ChallengePolicy {
    fn default() -> Self {
        Self {
            no_js: NoJsPolicy::Reject,
            min_score: 0.5,
            expected_action: None,
        }
    }
}

/// Turns the challenge on for `submit_contact`.
///
/// Provide it in the context closure passed to `leptos_routes_with_context`.
/// Without it, a submission that carries a challenge token is rejected with
/// `not_configured`: a widget was rendered but nothing can verify it.
#[derive(Clone)]
pub struct ChallengeContext {
    /// The verifier to ask.
    pub verifier: Arc<dyn ChallengeVerifier>,
    /// How to judge its answer.
    pub policy: ChallengePolicy,
}

impl std::fmt::Debug for ChallengeContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChallengeContext")
            .field("verifier", &"<dyn ChallengeVerifier>")
            .field("policy", &self.policy)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Decision table (RFC 005 D3)
// ---------------------------------------------------------------------------

/// What `submit_contact` does before, or instead of, asking the verifier.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Gate {
    /// Carry on to delivery.
    Proceed,
    /// Stop with this code.
    Reject(ContactErrorCode),
    /// Ask the verifier, then [`judge`] its answer.
    Verify,
}

/// Rows 1–4: decide from configuration and token presence alone.
///
/// An empty or whitespace-only token counts as absent.
pub(crate) fn gate(ctx: Option<&ChallengeContext>, token: Option<&str>) -> Gate {
    match (ctx, token.filter(|t| !t.trim().is_empty())) {
        // 1. Feature not in use.
        (None, None) => Gate::Proceed,
        // 2. A widget was rendered but nothing can verify it.
        (None, Some(_)) => Gate::Reject(ContactErrorCode::NotConfigured),
        // 3 and 4. Configured, but no token arrived.
        (Some(ctx), None) => match ctx.policy.no_js {
            NoJsPolicy::Reject => Gate::Reject(ContactErrorCode::ChallengeRequired),
            NoJsPolicy::AcceptWithHoneypotOnly => Gate::Proceed,
        },
        // 5–7 are decided by `judge`.
        (Some(_), Some(_)) => Gate::Verify,
    }
}

/// Rows 5–7: decide from the verifier's answer.
pub(crate) fn judge(
    result: Result<ChallengeOutcome, ChallengeError>,
    policy: &ChallengePolicy,
) -> Result<(), ContactErrorCode> {
    match result {
        // 7. Fail closed.
        Err(_) => Err(ContactErrorCode::ChallengeUnavailable),
        // 6. The vendor said no.
        Ok(outcome) if !outcome.passed => Err(ContactErrorCode::ChallengeFailed),
        // 6. Score too low.  A NaN score fails too: `NaN < min` is false.
        Ok(ChallengeOutcome {
            score: Some(score), ..
        }) if score.is_nan() || score < policy.min_score => Err(ContactErrorCode::ChallengeFailed),
        // 6. Wrong action.
        Ok(ChallengeOutcome { action, .. })
            if policy
                .expected_action
                .as_ref()
                .is_some_and(|want| action.as_ref() != Some(want)) =>
        {
            Err(ContactErrorCode::ChallengeFailed)
        }
        // 5. Passed.
        Ok(_) => Ok(()),
    }
}

/// The error a rejection goes out as.
///
/// Configuration and vendor problems are the server's, so they are
/// `ServerError`; a missing or failed challenge is the submission's, so it is
/// `Args`, like a validation failure.
pub(crate) fn rejection(code: ContactErrorCode) -> ServerFnError {
    let message = code.into_server_fn_message();
    match code {
        ContactErrorCode::NotConfigured | ContactErrorCode::ChallengeUnavailable => {
            ServerFnError::ServerError(message)
        }
        _ => ServerFnError::Args(message),
    }
}

// ---------------------------------------------------------------------------
// Built-in HTTP verifiers
// ---------------------------------------------------------------------------

#[cfg(feature = "challenge-http")]
pub mod http;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
