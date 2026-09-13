// tests.rs — unit tests for the parent module.
//
// Every test runs on a paused clock: time advances only when nothing else can
// run, so a deadline passes without waiting for it.

use std::future::pending;

use super::*;

#[derive(Debug)]
struct Never;

impl ContactDelivery for Never {
    fn deliver(
        &self,
        _input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        Box::pin(pending())
    }
}

/// Answers at once with the result its function returns.
struct Answers(fn() -> Result<(), ContactDeliveryError>);

impl ContactDelivery for Answers {
    fn deliver(
        &self,
        _input: ContactInput,
    ) -> Pin<Box<dyn Future<Output = Result<(), ContactDeliveryError>> + Send + '_>> {
        let result = (self.0)();
        Box::pin(async move { result })
    }
}

fn input() -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        None,
        "A message.".into(),
        String::new(),
    )
}

/// FR-DEL-08, NFR-PERF-03, T15 (RFC 009 D1): a delivery that never finishes
/// ends at the limit with `Timeout(limit)`.
#[tokio::test(start_paused = true)]
async fn a_delivery_that_never_finishes_times_out() {
    let limit = Duration::from_secs(30);
    let wrapped = DeliveryTimeout::new(Never, limit);
    assert_eq!(wrapped.limit(), limit);

    let started = tokio::time::Instant::now();
    let result = wrapped.deliver(input()).await;

    assert!(
        matches!(result, Err(ContactDeliveryError::Timeout(l)) if l == limit),
        "{result:?}"
    );
    assert!(started.elapsed() >= limit, "{:?}", started.elapsed());
}

/// FR-DEL-08 (RFC 009 D1): a delivery that finishes in time returns its own
/// result, success or error, unchanged.
#[tokio::test(start_paused = true)]
async fn a_delivery_that_finishes_in_time_returns_its_result() {
    let ok = DeliveryTimeout::new(Answers(|| Ok(())), Duration::from_secs(1));
    assert!(ok.deliver(input()).await.is_ok());

    let refused = DeliveryTimeout::new(
        Answers(|| Err(ContactDeliveryError::Transport("relay said no".into()))),
        Duration::from_secs(1),
    );
    let result = refused.deliver(input()).await;
    assert!(
        matches!(&result, Err(ContactDeliveryError::Transport(m)) if m == "relay said no"),
        "{result:?}"
    );
}

/// `Debug` shows the wrapped backend and the limit.
#[test]
fn debug_shows_the_inner_backend_and_the_limit() {
    let printed = format!("{:?}", DeliveryTimeout::new(Never, Duration::from_secs(30)));
    assert_eq!(printed, "DeliveryTimeout { inner: Never, limit: 30s }");
}
