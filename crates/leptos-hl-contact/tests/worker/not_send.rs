//! A delivery, a verifier and a filter whose futures hold an `Rc` across an
//! `.await`, so none of them is `Send`.

use std::{rc::Rc, sync::Arc};

use leptos_hl_contact::{
    ChallengeOutcome, ChallengeVerifier, ContactDelivery, ContactDeliveryContext, ContactFilter,
    ContactFilterContext, ContactInput, DeliveryFuture, FilterDecision, FilterFuture, VerifyFuture,
};
use wasm_bindgen_test::wasm_bindgen_test;

/// Holds an `Rc` across an `.await`: the future that calls it is not `Send`.
async fn hold_rc_across_await() {
    let not_send = Rc::new(());
    std::future::ready(()).await;
    drop(not_send);
}

struct RcDelivery;

impl ContactDelivery for RcDelivery {
    fn deliver(&self, _input: ContactInput) -> DeliveryFuture<'_> {
        Box::pin(async {
            hold_rc_across_await().await;
            Ok(())
        })
    }
}

struct RcVerifier;

impl ChallengeVerifier for RcVerifier {
    fn verify(&self, _token: &str) -> VerifyFuture<'_> {
        Box::pin(async {
            hold_rc_across_await().await;
            Ok(ChallengeOutcome {
                passed: true,
                ..ChallengeOutcome::default()
            })
        })
    }
}

struct RcFilter;

impl ContactFilter for RcFilter {
    fn filter(&self, _input: &ContactInput) -> FilterFuture<'_> {
        Box::pin(async {
            hold_rc_across_await().await;
            FilterDecision::Accept
        })
    }
}

fn input() -> ContactInput {
    ContactInput::from_raw(
        "Ada".into(),
        "ada@example.com".into(),
        None,
        "Hello".into(),
        String::new(),
    )
}

/// NFR-PORT-02, FR-DEL-01, FR-DEL-07 (RFC 011 D2): on a wasm32 server a
/// delivery, a verifier and a filter may return futures that are not `Send`,
/// and each still goes into the `Send + Sync` value Leptos context holds.
#[wasm_bindgen_test]
async fn extension_futures_need_not_be_send() {
    let delivery: ContactDeliveryContext = Arc::new(RcDelivery);
    assert!(delivery.deliver(input()).await.is_ok());

    let verifier: Arc<dyn ChallengeVerifier> = Arc::new(RcVerifier);
    assert!(verifier.verify("token").await.expect("an outcome").passed);

    let filter: ContactFilterContext = Arc::new(RcFilter);
    assert_eq!(filter.filter(&input()).await, FilterDecision::Accept);
}
