// delivery/resend.rs — Delivery via Resend's HTTP API (RFC 017).
//
// Enabled by the `delivery-resend` feature flag.  Runs natively and on a
// wasm32 server (Cloudflare Workers), through the shared transport in
// `crate::http`.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    delivery::{
        ContactDelivery, DeliveryFuture,
        body::{build_plain_text_body, compose_subject},
    },
    error::ContactDeliveryError,
    http::{HttpBody, HttpClient, HttpError, HttpRequest},
    model::ContactInput,
};

/// Resend's endpoint.  Kept in one place, in this module (RFC 017 D1a).
const RESEND_URL: &str = "https://api.resend.com/emails";

// ---------------------------------------------------------------------------
// ResendConfig
// ---------------------------------------------------------------------------

/// Resend delivery configuration.
///
/// Only `new`'s three values are required; everything else defaults and is
/// changed with a builder, so a field added later never breaks an existing
/// literal or call ([`#[non_exhaustive]`](Self), no public fields — the
/// lesson 0.8.0 wrote down for `ContactServerPolicy`, `ContactInput` and
/// `ContactFieldErrors`, and `SmtpConfig`'s own shape before that).
///
/// # Security
///
/// Load `api_key` from an environment variable or a secret store — never
/// hard-code it.  `from_address` must be a sender address at a domain
/// verified with Resend; an unverified domain is rejected by Resend itself
/// (`Configuration`, via a 401 or 403), not checked here.
///
/// # Example
///
/// ```rust,no_run
/// use leptos_hl_contact::delivery::resend::ResendConfig;
///
/// let config = ResendConfig::new(
///     std::env::var("RESEND_API_KEY").unwrap(),
///     "noreply@example.com",
///     "admin@example.com",
/// )
/// .with_subject_prefix("[Contact]");
/// ```
#[non_exhaustive]
pub struct ResendConfig {
    api_key: String,
    from_address: String,
    to_address: String,
    subject_prefix: String,
    timeout: Duration,
}

impl ResendConfig {
    /// Ten seconds: shorter than SMTP's default, because one JSON `POST` has
    /// no connect-then-dialogue phases to add up.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    /// A config with no subject prefix and [`Self::DEFAULT_TIMEOUT`].
    ///
    /// An empty `api_key`, `from_address` or `to_address` is accepted here
    /// and reported as [`ContactDeliveryError::Configuration`] on the first
    /// delivery, so a missing environment variable fails closed rather than
    /// at startup, which a Worker does not have.
    pub fn new(
        api_key: impl Into<String>,
        from_address: impl Into<String>,
        to_address: impl Into<String>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            from_address: from_address.into(),
            to_address: to_address.into(),
            subject_prefix: String::new(),
            timeout: Self::DEFAULT_TIMEOUT,
        }
    }

    /// Prepended to every email subject line (e.g. `"[Contact]"`), with a
    /// space before the enquiry's own subject, exactly as `SmtpConfig`'s
    /// prefix is.
    pub fn with_subject_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.subject_prefix = prefix.into();
        self
    }

    /// Change the per-delivery time limit.
    ///
    /// Applies to the one HTTP request this backend makes; there is no
    /// separate connect phase to bound.  When it passes, the visitor sees
    /// `delivery_timeout`, which says the message may have been sent.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl std::fmt::Debug for ResendConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResendConfig")
            .field("api_key", &"<redacted>")
            .field("from_address", &self.from_address)
            .field("to_address", &self.to_address)
            .field("subject_prefix", &self.subject_prefix)
            .field("timeout", &self.timeout)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ResendDelivery
// ---------------------------------------------------------------------------

/// Delivery backend that posts to [Resend](https://resend.com)'s HTTP API.
///
/// Requires the `delivery-resend` feature flag.  Runs natively and on a
/// wasm32 server (Cloudflare Workers): unlike `LettreSmtpDelivery`
/// (`smtp-lettre`), it needs no `tokio` runtime.
///
/// # Security
///
/// - `from` and `to` are always the configured addresses; the submission is
///   never used for either.
/// - The visitor's `email` is placed in `reply_to` only.
/// - The API key is sent only to Resend, in the `Authorization` header, and
///   `Debug` redacts it.
/// - The body sent is the same plain text `LettreSmtpDelivery` sends
///   (`delivery/body.rs`), including the site's own fields.
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use leptos_hl_contact::delivery::{
///     ContactDeliveryContext,
///     resend::{ResendConfig, ResendDelivery},
/// };
///
/// let delivery: ContactDeliveryContext = Arc::new(ResendDelivery::new(ResendConfig::new(
///     std::env::var("RESEND_API_KEY").unwrap(),
///     std::env::var("RESEND_FROM").unwrap(),
///     std::env::var("CONTACT_TO").unwrap(),
/// )));
/// ```
pub struct ResendDelivery {
    config: ResendConfig,
    client: HttpClient,
    url: String,
}

impl ResendDelivery {
    /// A delivery backend for `config`, posting to Resend's endpoint.
    pub fn new(config: ResendConfig) -> Self {
        Self {
            config,
            client: HttpClient::new(),
            url: RESEND_URL.to_owned(),
        }
    }

    /// Send to `url` instead of Resend's endpoint — for a forwarding proxy
    /// that outbound traffic must pass through, or a test server.  The same
    /// escape hatch `HttpChallengeVerifier::with_verify_url` gives the
    /// challenge verifiers (`challenge-http`).
    ///
    /// A `url` that does not start with `https://` logs one `warn!`, here,
    /// naming neither the URL nor the key: the key would be sent in the
    /// clear.  It is not refused — a local responder over `http` is exactly
    /// what this crate's own tests, and a site's staging setup, do.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        let url = url.into();
        if !url.starts_with("https://") {
            tracing::warn!("delivery URL is not https: the key will be sent in the clear");
        }
        self.url = url;
        self
    }
}

impl std::fmt::Debug for ResendDelivery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResendDelivery")
            .field("config", &self.config)
            .finish()
    }
}

/// The JSON body Resend's `/emails` endpoint takes (RFC 017 D3).  Field
/// order is the order Resend's own docs give.
#[derive(Serialize)]
struct ResendRequestBody<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text: &'a str,
    reply_to: &'a str,
}

/// The one field this backend reads from a 2xx answer: the provider's
/// message id, for operators to trace an enquiry the recipient says never
/// arrived.  Absent or unparsable is not an error — the delivery already
/// succeeded.
#[derive(Deserialize)]
struct ResendSuccessBody {
    #[serde(default)]
    id: Option<String>,
}

fn parse_id(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<ResendSuccessBody>(body)
        .ok()
        .and_then(|b| b.id)
}

/// `HttpError::Timeout` is the crate's own timeout error, so the visitor
/// sees `delivery_timeout`; everything else the transport could not judge —
/// `Transport` and `Unusable` alike — is `Transport`, with the module's own
/// text (never a URL, never the vendor's).
fn map_http_error(error: HttpError, timeout: Duration) -> ContactDeliveryError {
    match error {
        HttpError::Timeout => ContactDeliveryError::Timeout(timeout),
        HttpError::Transport(text) => ContactDeliveryError::Transport(text),
        HttpError::Unusable(reason) => ContactDeliveryError::Transport(reason.to_owned()),
    }
}

impl ContactDelivery for ResendDelivery {
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_> {
        Box::pin(async move {
            for (name, value) in [
                ("api_key", &self.config.api_key),
                ("from_address", &self.config.from_address),
                ("to_address", &self.config.to_address),
            ] {
                if value.is_empty() {
                    return Err(ContactDeliveryError::Configuration(format!(
                        "{name} is empty"
                    )));
                }
            }

            let subject = compose_subject(
                &self.config.subject_prefix,
                &input.effective_subject("(no subject)"),
            );
            let text = build_plain_text_body(&input);

            let payload = ResendRequestBody {
                from: &self.config.from_address,
                to: &self.config.to_address,
                subject: &subject,
                text: &text,
                reply_to: &input.email,
            };
            let json = serde_json::to_string(&payload)
                .map_err(|e| ContactDeliveryError::MessageBuild(e.to_string()))?;

            let authorization = format!("Bearer {}", self.config.api_key);
            let headers = [("authorization", authorization)];
            let request = HttpRequest {
                url: &self.url,
                headers: &headers,
                body: HttpBody::Json(&json),
                limit: self.config.timeout,
            };

            let response = match self.client.post(request).await {
                Ok(response) => response,
                Err(e) => return Err(log_failure(map_http_error(e, self.config.timeout))),
            };

            if (200..=299).contains(&response.status) {
                match parse_id(&response.body) {
                    Some(id) => {
                        tracing::info!(id = %id, "contact form submission delivered via Resend")
                    }
                    None => tracing::info!("contact form submission delivered via Resend"),
                }
                return Ok(());
            }

            let error = match response.status {
                401 | 403 => {
                    ContactDeliveryError::Configuration(format!("HTTP {}", response.status))
                }
                422 => ContactDeliveryError::MessageBuild(format!("HTTP {}", response.status)),
                // 429, 5xx and anything else not handled above (RFC 017 D4).
                _ => ContactDeliveryError::Transport(format!("HTTP {}", response.status)),
            };
            Err(log_failure(error))
        })
    }
}

/// Logs every delivery failure this backend produces itself, except a
/// timeout: `submit_contact` already logs that one with the limit that
/// passed (`delivery/timeout.rs`'s pattern).  Mirrors `LettreSmtpDelivery`'s
/// own `"SMTP delivery failed"` line.
fn log_failure(error: ContactDeliveryError) -> ContactDeliveryError {
    if !matches!(error, ContactDeliveryError::Timeout(_)) {
        tracing::error!(error = %error, "Resend delivery failed");
    }
    error
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
