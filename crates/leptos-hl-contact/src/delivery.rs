// delivery.rs — ContactDelivery trait and shared delivery infrastructure.
//
// All delivery code is server-side only (`ssr` feature gate).  Import this
// module only from `#[server]` functions or other SSR-only code paths.

pub mod noop;

// The plain-text body every backend sends (RFC 017 D1).  Private: no type
// of it appears in the crate's API.
#[cfg(any(feature = "smtp-lettre", feature = "delivery-resend"))]
mod body;

#[cfg(feature = "smtp-lettre")]
pub mod smtp;

// Delivery through Resend's HTTP API (RFC 017); runs natively and on a
// Worker.
#[cfg(feature = "delivery-resend")]
pub mod resend;

#[cfg(feature = "delivery-timeout")]
pub mod timeout;

use std::{future::Future, pin::Pin, sync::Arc};

use crate::{error::ContactDeliveryError, model::ContactInput};

// ---------------------------------------------------------------------------
// ContactDelivery trait
// ---------------------------------------------------------------------------

/// The future [`ContactDelivery::deliver`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(not(all(target_arch = "wasm32", feature = "ssr")))]
pub type DeliveryFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + 'a>>;

/// The future [`ContactDelivery::deliver`] returns.
///
/// It is `Send` on every target except a wasm32 server build
/// (`all(target_arch = "wasm32", feature = "ssr")`, such as Cloudflare
/// Workers), where an implementation may await JavaScript futures, which are
/// not `Send`.  The implementing type itself must still be `Send + Sync` on
/// every target: Leptos context requires it.
#[cfg(all(target_arch = "wasm32", feature = "ssr"))]
pub type DeliveryFuture<'a> = Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + 'a>>;

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
/// # Errors
///
/// The text of a returned [`ContactDeliveryError`] is written to the server
/// log for operators, and never sent to the visitor.  Put the category and
/// the transport detail in it — status codes, the relay's reply.  Never put
/// the submission in it: no name, email address, subject, message, token or
/// credential.  This is the same rule the crate follows for its own log
/// events.
///
/// # Cancellation
///
/// A delivery may be cancelled at any `.await` when it is wrapped in a
/// timeout — `DeliveryTimeout`, or the SMTP backend's own deadline.  Do not
/// leave shared state half-updated across an `.await`.  An HTTP API call
/// cancelled mid-flight may still complete on the vendor's side.
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
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_>;
}

// ---------------------------------------------------------------------------
// ContactDeliveryContext
// ---------------------------------------------------------------------------

/// Type alias for the Leptos context used to inject a delivery backend.
///
/// Register this in your Axum router with
/// [`provide_context`](leptos::context::provide_context) inside the closure
/// passed to `leptos_routes_with_context`.
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
/// // Provide it in the closure passed to `leptos_routes_with_context`; the
/// // same closure serves page renders and server functions.
/// provide_context(delivery);
/// ```
pub type ContactDeliveryContext = Arc<dyn ContactDelivery>;
