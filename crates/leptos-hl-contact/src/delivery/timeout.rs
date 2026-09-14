// delivery/timeout.rs — a deadline for any delivery backend.
//
// Enabled by the `delivery-timeout` feature, which `smtp-lettre` turns on: the
// SMTP backend uses the same deadline.

use std::{future::Future, time::Duration};

use crate::{
    delivery::{ContactDelivery, DeliveryFuture},
    error::ContactDeliveryError,
    model::ContactInput,
};

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
    fn deliver(&self, input: ContactInput) -> DeliveryFuture<'_> {
        Box::pin(with_deadline(self.limit, self.inner.deliver(input)))
    }
}

/// Run `delivery`, or give up at `limit` with [`ContactDeliveryError::Timeout`].
///
/// The one implementation of the deadline, shared by [`DeliveryTimeout`] and
/// the SMTP backend.
#[cfg(not(all(target_arch = "wasm32", feature = "ssr")))]
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

/// On a wasm32 server there is no tokio timer, so the delivery races a
/// JavaScript timer (RFC 011 D7).
///
/// The delivery is polled first, so one that is ready when the timer fires
/// still returns its own result.  Whichever finishes, the other is dropped
/// when this function returns: an unfinished delivery is cancelled, and an
/// unfired timer is cleared by `Sleep`'s `Drop`.
#[cfg(all(target_arch = "wasm32", feature = "ssr"))]
pub(crate) async fn with_deadline<F>(
    limit: Duration,
    delivery: F,
) -> Result<(), ContactDeliveryError>
where
    F: Future<Output = Result<(), ContactDeliveryError>>,
{
    use std::task::Poll;

    let mut delivery = std::pin::pin!(delivery);
    let mut timer = crate::wasm_timer::sleep(limit);
    std::future::poll_fn(|cx| {
        if let Poll::Ready(result) = delivery.as_mut().poll(cx) {
            return Poll::Ready(result);
        }
        match std::pin::Pin::new(&mut timer).poll(cx) {
            Poll::Ready(()) => Poll::Ready(Err(ContactDeliveryError::Timeout(limit))),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
