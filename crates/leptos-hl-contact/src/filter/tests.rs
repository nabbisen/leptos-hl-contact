// tests.rs — filter decisions and chain behaviour.

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use super::*;

/// A filter with a fixed decision that counts its calls.
struct Fixed {
    decision: FilterDecision,
    calls: Arc<AtomicUsize>,
}

impl Fixed {
    fn new(decision: FilterDecision) -> (Arc<Self>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Arc::new(Self {
                decision,
                calls: Arc::clone(&calls),
            }),
            calls,
        )
    }
}

impl ContactFilter for Fixed {
    fn filter(&self, _input: &ContactInput) -> FilterFuture<'_> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let decision = self.decision;
        Box::pin(async move { decision })
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

#[tokio::test]
async fn a_filter_can_return_each_decision() {
    for decision in [
        FilterDecision::Accept,
        FilterDecision::Reject,
        FilterDecision::SilentDrop,
    ] {
        let (filter, calls) = Fixed::new(decision);
        assert_eq!(filter.filter(&input()).await, decision);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn a_chain_returns_the_first_non_accept_and_stops() {
    for decisive in [FilterDecision::Reject, FilterDecision::SilentDrop] {
        let (first, first_calls) = Fixed::new(FilterDecision::Accept);
        let (second, second_calls) = Fixed::new(decisive);
        let (third, third_calls) = Fixed::new(FilterDecision::Reject);
        let chain = FilterChain::new(vec![first, second, third]);

        assert_eq!(chain.filter(&input()).await, decisive);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            third_calls.load(Ordering::SeqCst),
            0,
            "a filter after the deciding one must not be called"
        );
    }
}

#[tokio::test]
async fn a_chain_of_accepts_accepts_and_calls_every_filter() {
    let (a, a_calls) = Fixed::new(FilterDecision::Accept);
    let (b, b_calls) = Fixed::new(FilterDecision::Accept);
    let chain = FilterChain::new(vec![a, b]);
    assert_eq!(chain.filter(&input()).await, FilterDecision::Accept);
    assert_eq!(a_calls.load(Ordering::SeqCst), 1);
    assert_eq!(b_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn an_empty_chain_accepts() {
    assert_eq!(
        FilterChain::new(Vec::new()).filter(&input()).await,
        FilterDecision::Accept
    );
}

#[test]
fn the_default_name_is_the_type_name() {
    let (filter, _) = Fixed::new(FilterDecision::Accept);
    assert_eq!(filter.name(), std::any::type_name::<Fixed>());
    assert!(filter.name().ends_with("Fixed"), "{}", filter.name());
}

/// A chain in context is still one `ContactFilterContext`.
#[tokio::test]
async fn a_chain_is_usable_as_the_context() {
    let (reject, _) = Fixed::new(FilterDecision::Reject);
    let context: ContactFilterContext = Arc::new(FilterChain::new(vec![reject]));
    assert_eq!(context.filter(&input()).await, FilterDecision::Reject);
}
