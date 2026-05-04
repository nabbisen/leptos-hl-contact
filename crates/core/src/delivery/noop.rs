// delivery/noop.rs — No-op delivery backend for testing and local development.

use std::{future::Future, pin::Pin};

use crate::{delivery::ContactDelivery, error::ContactDeliveryError, model::ContactInput};

/// A delivery backend that silently discards every submission.
///
/// Use this during local development, in unit tests, or as a placeholder while
/// you configure a real backend.  Every call to
/// [`deliver`](ContactDelivery::deliver) logs the submission at `DEBUG` level
/// and returns `Ok(())`.
///
/// # Example
///
/// ```rust
/// use std::sync::Arc;
/// use leptos_hl_contact::delivery::{ContactDeliveryContext, noop::NoopDelivery};
///
/// let delivery: ContactDeliveryContext = Arc::new(NoopDelivery);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopDelivery;

impl ContactDelivery for NoopDelivery {
    fn deliver(
        &self,
        input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        Box::pin(async move {
            tracing::debug!(
                name = %input.name,
                email = %input.email,
                "NoopDelivery: discarding contact form submission"
            );
            Ok(())
        })
    }
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
