// delivery/timeout.rs — a deadline for any delivery backend.
//
// Enabled by the `delivery-timeout` feature, which `smtp-lettre` turns on: the
// SMTP backend uses the same deadline.

use std::{future::Future, pin::Pin, time::Duration};

use crate::{delivery::ContactDelivery, error::ContactDeliveryError, model::ContactInput};

/// Bounds any backend's [`deliver`](ContactDelivery::deliver) by a deadline.
///
/// When the deadline passes, the inner delivery is dropped — cancelled at its
/// current `.await` — and the result is [`ContactDeliveryError::Timeout`].  The
/// visitor sees `delivery_timeout`, which says the message *may* have been
/// sent: a deadline can pass after the relay accepted the message but before
/// its reply arrived.
///
/// Needs a Tokio runtime with its time driver enabled, as every Tokio server
/// runtime (Axum's included) has.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, time::Duration};
/// use leptos_hl_contact::delivery::{
///     ContactDeliveryContext, noop::NoopDelivery, timeout::DeliveryTimeout,
/// };
///
/// let delivery: ContactDeliveryContext =
///     Arc::new(DeliveryTimeout::new(NoopDelivery, Duration::from_secs(30)));
/// ```
pub struct DeliveryTimeout<D> {
    inner: D,
    limit: Duration,
}

impl<D: ContactDelivery> DeliveryTimeout<D> {
    /// Wrap `inner` so that one delivery takes at most `limit`.
    pub fn new(inner: D, limit: Duration) -> Self {
        Self { inner, limit }
    }

    /// The deadline for one delivery.
    pub fn limit(&self) -> Duration {
        self.limit
    }
}

impl<D: std::fmt::Debug> std::fmt::Debug for DeliveryTimeout<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeliveryTimeout")
            .field("inner", &self.inner)
            .field("limit", &self.limit)
            .finish()
    }
}

impl<D: ContactDelivery> ContactDelivery for DeliveryTimeout<D> {
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        Box::pin(with_deadline(self.limit, self.inner.deliver(input)))
    }
}

/// Run `delivery`, or give up at `limit` with [`ContactDeliveryError::Timeout`].
///
/// The one implementation of the deadline, shared by [`DeliveryTimeout`] and
/// the SMTP backend.
pub(crate) async fn with_deadline<F>(
    limit: Duration,
    delivery: F,
) -> Result<(), ContactDeliveryError>
where
    F: Future<Output = Result<(), ContactDeliveryError>>,
{
    match tokio::time::timeout(limit, delivery).await {
        Ok(result) => result,
        Err(_elapsed) => Err(ContactDeliveryError::Timeout(limit)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
