// delivery/smtp.rs — SMTP delivery backend via the `lettre` crate.
//
// Enabled by the `smtp-lettre` feature flag.

use std::{future::Future, pin::Pin};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};

use crate::{delivery::ContactDelivery, error::ContactDeliveryError, model::ContactInput};

// ---------------------------------------------------------------------------
// SmtpTlsMode
// ---------------------------------------------------------------------------

/// TLS mode for the SMTP connection.
///
/// Choose based on the requirements of your SMTP relay.
#[derive(Debug, Clone, Default)]
pub enum SmtpTlsMode {
    /// STARTTLS — connects on the plain port (usually 587) then upgrades.
    /// This is the recommended default for most providers.
    #[default]
    StartTls,

    /// Implicit TLS — connects on a dedicated TLS port (usually 465).
    Tls,

    /// No TLS — plaintext only.
    ///
    /// # Security Warning
    ///
    /// Use this mode **only** for local development or internal networks.
    /// Credentials and message content are transmitted unencrypted.
    None,
}

// ---------------------------------------------------------------------------
// SmtpConfig
// ---------------------------------------------------------------------------

/// SMTP relay configuration.
///
/// All fields are server-side only.  Never serialise this type or pass it to
/// client-side code.
///
/// # Security
///
/// Load `username` and `password` from environment variables or a secret
/// store — never hard-code them.
///
/// # Example
///
/// ```rust,no_run
/// use leptos_hl_contact::delivery::smtp::{SmtpConfig, SmtpTlsMode};
///
/// let config = SmtpConfig {
///     host:           std::env::var("SMTP_HOST").unwrap(),
///     port:           587,
///     username:       std::env::var("SMTP_USER").unwrap(),
///     password:       std::env::var("SMTP_PASS").unwrap(),
///     from_address:   std::env::var("SMTP_FROM").unwrap(),
///     to_address:     std::env::var("CONTACT_TO").unwrap(),
///     subject_prefix: "[Contact]".into(),
///     tls_mode:       SmtpTlsMode::StartTls,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// Hostname of the SMTP relay (e.g. `smtp.mailgun.org`).
    pub host: String,
    /// Port number (e.g. 587 for STARTTLS, 465 for implicit TLS).
    pub port: u16,
    /// SMTP authentication username.
    pub username: String,
    /// SMTP authentication password.
    ///
    /// # Security
    ///
    /// Load from an environment variable; do not embed in source code.
    pub password: String,
    /// The `From` header address (must be authorised to send from your relay).
    pub from_address: String,
    /// The `To` address where enquiries are delivered.
    pub to_address: String,
    /// Short prefix prepended to every email subject line (e.g. `"[Contact]"`).
    pub subject_prefix: String,
    /// TLS negotiation mode.
    pub tls_mode: SmtpTlsMode,
}

// ---------------------------------------------------------------------------
// LettreSmtpDelivery
// ---------------------------------------------------------------------------

/// SMTP delivery backend backed by [`lettre`].
///
/// Requires the `smtp-lettre` feature flag.
///
/// # Security
///
/// - `From` is always the server-configured address; user input is **never**
///   used for `From`.
/// - User-supplied `email` is placed in `Reply-To` only.
/// - `name` and `subject` are validated server-side to contain no newlines
///   before this method is called.
/// - SMTP credentials reside in [`SmtpConfig`] which lives only on the server.
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use leptos_hl_contact::delivery::{ContactDeliveryContext, smtp::{LettreSmtpDelivery, SmtpConfig, SmtpTlsMode}};
///
/// let delivery: ContactDeliveryContext = Arc::new(LettreSmtpDelivery {
///     config: SmtpConfig {
///         host:           "smtp.example.com".into(),
///         port:           587,
///         username:       std::env::var("SMTP_USER").unwrap(),
///         password:       std::env::var("SMTP_PASS").unwrap(),
///         from_address:   "noreply@example.com".into(),
///         to_address:     "admin@example.com".into(),
///         subject_prefix: "[Contact]".into(),
///         tls_mode:       SmtpTlsMode::StartTls,
///     },
/// });
/// ```
#[derive(Debug)]
pub struct LettreSmtpDelivery {
    /// SMTP relay configuration.
    pub config: SmtpConfig,
}

impl LettreSmtpDelivery {
    /// Build an [`AsyncSmtpTransport`] from the stored configuration.
    fn build_transport(&self) -> Result<AsyncSmtpTransport<Tokio1Executor>, ContactDeliveryError> {
        let creds = Credentials::new(self.config.username.clone(), self.config.password.clone());

        let transport = match self.config.tls_mode {
            SmtpTlsMode::StartTls => {
                AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.config.host)
                    .map_err(|e| ContactDeliveryError::Configuration(e.to_string()))?
                    .port(self.config.port)
                    .credentials(creds)
                    .build()
            }
            SmtpTlsMode::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&self.config.host)
                .map_err(|e| ContactDeliveryError::Configuration(e.to_string()))?
                .port(self.config.port)
                .credentials(creds)
                .build(),
            SmtpTlsMode::None => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.config.host)
                    .port(self.config.port)
                    .credentials(creds)
                    .build()
            }
        };

        Ok(transport)
    }

    /// Construct a [`Message`] from a validated [`ContactInput`].
    ///
    /// Headers:
    /// - `From`     — server-configured address
    /// - `To`       — server-configured recipient
    /// - `Reply-To` — user-supplied email
    /// - `Subject`  — `subject_prefix` + sanitised subject
    /// - `Body`     — plain text
    pub fn build_message(&self, input: &ContactInput) -> Result<Message, ContactDeliveryError> {
        let from =
            self.config
                .from_address
                .parse()
                .map_err(|e: lettre::address::AddressError| {
                    ContactDeliveryError::Configuration(format!("invalid from_address: {e}"))
                })?;

        let to = self
            .config
            .to_address
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                ContactDeliveryError::Configuration(format!("invalid to_address: {e}"))
            })?;

        let reply_to = format!("{} <{}>", input.name, input.email)
            .parse()
            .map_err(|e: lettre::address::AddressError| {
                ContactDeliveryError::MessageBuild(format!("invalid reply-to address: {e}"))
            })?;

        let effective_subject = input.effective_subject("(no subject)");
        let subject = format!("{} {}", self.config.subject_prefix, effective_subject);
        let body = build_plain_text_body(input);

        let message = Message::builder()
            .from(from)
            .to(to)
            .reply_to(reply_to)
            .subject(subject)
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_PLAIN)
                    .body(body),
            )
            .map_err(|e| ContactDeliveryError::MessageBuild(e.to_string()))?;

        Ok(message)
    }
}

impl ContactDelivery for LettreSmtpDelivery {
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        Box::pin(async move {
            let transport = self.build_transport()?;
            let message = self.build_message(&input)?;

            transport.send(message).await.map_err(|e| {
                tracing::error!(error = %e, "SMTP delivery failed");
                ContactDeliveryError::Transport(e.to_string())
            })?;

            tracing::info!(
                name = %input.name,
                "contact form submission delivered via SMTP"
            );
            Ok(())
        })
    }
}

// ---------------------------------------------------------------------------
// Plain text body builder
// ---------------------------------------------------------------------------

fn build_plain_text_body(input: &ContactInput) -> String {
    let subject = input.subject.as_deref().unwrap_or("(none)");
    format!(
        "New contact form submission\n\
         ===========================\n\
         \n\
         Name:\n\
         {}\n\
         \n\
         Email:\n\
         {}\n\
         \n\
         Subject:\n\
         {}\n\
         \n\
         Message:\n\
         {}\n",
        input.name, input.email, subject, input.message,
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_config() -> SmtpConfig {
        SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: "user".into(),
            password: "secret".into(),
            from_address: "noreply@example.com".into(),
            to_address: "admin@example.com".into(),
            subject_prefix: "[Contact]".into(),
            tls_mode: SmtpTlsMode::StartTls,
        }
    }

    fn sample_input() -> ContactInput {
        ContactInput::from_raw(
            "Alice".into(),
            "alice@example.com".into(),
            Some("Hello".into()),
            "This is a test message.".into(),
            String::new(),
        )
    }

    #[test]
    fn message_builder_creates_expected_headers() {
        let delivery = LettreSmtpDelivery {
            config: sample_config(),
        };
        let message = delivery.build_message(&sample_input()).unwrap();
        let raw = String::from_utf8(message.formatted()).unwrap();
        assert!(
            raw.contains("noreply@example.com")
                && (raw.contains("From: ") || raw.contains("From:"))
        );
        assert!(raw.contains("admin@example.com") && (raw.contains("To: ") || raw.contains("To:")));
        assert!(raw.contains("Reply-To:"));
        assert!(raw.contains("[Contact] Hello"));
    }

    #[test]
    fn from_uses_configured_address() {
        let delivery = LettreSmtpDelivery {
            config: sample_config(),
        };
        let message = delivery.build_message(&sample_input()).unwrap();
        let raw = String::from_utf8(message.formatted()).unwrap();
        assert!(raw.contains("noreply@example.com"));
    }

    #[test]
    fn reply_to_uses_user_email() {
        let delivery = LettreSmtpDelivery {
            config: sample_config(),
        };
        let message = delivery.build_message(&sample_input()).unwrap();
        let raw = String::from_utf8(message.formatted()).unwrap();
        assert!(raw.contains("alice@example.com"));
    }

    #[test]
    fn subject_contains_prefix_and_value() {
        let delivery = LettreSmtpDelivery {
            config: sample_config(),
        };
        let message = delivery.build_message(&sample_input()).unwrap();
        let raw = String::from_utf8(message.formatted()).unwrap();
        assert!(raw.contains("[Contact]"));
        assert!(raw.contains("Hello"));
    }

    #[test]
    fn body_includes_expected_fields() {
        let input = sample_input();
        let body = build_plain_text_body(&input);
        assert!(body.contains("Alice"));
        assert!(body.contains("alice@example.com"));
        assert!(body.contains("Hello"));
        assert!(body.contains("This is a test message."));
    }
}
