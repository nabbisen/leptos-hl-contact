// delivery/mod.rs — ContactDelivery trait and shared delivery infrastructure.
//
// All delivery code is server-side only (`ssr` feature gate).  Import this
// module only from `#[server]` functions or other SSR-only code paths.

pub mod noop;

#[cfg(feature = "smtp-lettre")]
pub mod smtp;

use std::{future::Future, pin::Pin, sync::Arc};

use crate::{error::ContactDeliveryError, model::ContactInput};

// ---------------------------------------------------------------------------
// ContactDelivery trait
// ---------------------------------------------------------------------------

/// Abstraction over message delivery backends.
///
/// Implement this trait to add a custom delivery backend (SendGrid, AWS SES,
/// database persistence, Slack webhook, etc.).  The crate ships two built-in
/// implementations:
///
/// - [`NoopDelivery`](noop::NoopDelivery) — discards messages; useful for
///   local development and tests.
/// - [`LettreSmtpDelivery`](smtp::LettreSmtpDelivery) — sends via SMTP using
///   [`lettre`].  Requires the `smtp-lettre` feature.
///
/// # Dyn compatibility
///
/// The trait uses `Pin<Box<dyn Future>>` rather than `async fn` so it can be
/// used as `Arc<dyn ContactDelivery>` (object-safe / dyn-compatible).
///
/// # Security
///
/// Implementations **must not** expose credentials, SMTP passwords, or API
/// keys to the client side.  Keep all secrets in server-side environment
/// variables or a secret store.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use leptos_hl_contact::delivery::{ContactDelivery, ContactDeliveryContext};
/// use leptos_hl_contact::delivery::noop::NoopDelivery;
///
/// let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
/// ```
pub trait ContactDelivery: Send + Sync + 'static {
    /// Deliver a validated contact form submission.
    ///
    /// Implementations should avoid leaking internal error details; callers
    /// will log errors and return a generic message to the client.
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>>;
}

// ---------------------------------------------------------------------------
// ContactDeliveryContext
// ---------------------------------------------------------------------------

/// Type alias for the Leptos context used to inject a delivery backend.
///
/// Register this in your Axum router via
/// [`provide_context`](leptos::context::provide_context) or
/// [`handle_server_fns_with_context`](leptos_axum::handle_server_fns_with_context).
///
/// # Example (Axum)
///
/// ```rust,ignore
/// use std::sync::Arc;
/// use leptos::context::provide_context;
/// use leptos_hl_contact::delivery::ContactDeliveryContext;
/// use leptos_hl_contact::delivery::noop::NoopDelivery;
///
/// let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
/// // Provide to both the server-function handler and the SSR renderer.
/// provide_context(delivery);
/// ```
pub type ContactDeliveryContext = Arc<dyn ContactDelivery>;
