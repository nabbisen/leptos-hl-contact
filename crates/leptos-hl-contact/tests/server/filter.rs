//! The pre-delivery filter.

use std::sync::Arc;

use leptos_hl_contact::{ContactFilter, FilterChain, FilterDecision};

use crate::support::{FixedFilter, Harness, Setup};

/// FR-ABUSE-14, RFC 006 D1: `Reject` is the generic `rejected` code in both
/// request forms, and nothing is delivered.
#[tokio::test]
async fn a_filter_reject_returns_the_rejected_code() {
    let h = Harness::new(Setup {
        filter: Some(Arc::new(FixedFilter::new(
            FilterDecision::Reject,
            "Rejects",
        ))),
        ..Setup::default()
    });

    let fetch = h.submit_fetch(&h.fields()).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("rejected"));
    assert!(!fetch.is_server_error());

    let page = h.follow(&h.submit_nojs(&h.fields()).await).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("Your message could not be accepted.")
    );
    assert_eq!(h.deliveries(), 0);
}

/// FR-ABUSE-14 (RFC 006 D1): a chain stops at the first decision other
/// than `Accept`; the filters after it are never called.
#[tokio::test]
async fn a_filter_chain_short_circuits() {
    let first = Arc::new(FixedFilter::new(FilterDecision::Accept, "First"));
    let second = Arc::new(FixedFilter::new(FilterDecision::Reject, "Second"));
    let third = Arc::new(FixedFilter::new(FilterDecision::SilentDrop, "Third"));
    let filters: Vec<Arc<dyn ContactFilter>> = vec![first.clone(), second.clone(), third.clone()];
    let h = Harness::new(Setup {
        filter: Some(Arc::new(FilterChain::new(filters))),
        ..Setup::default()
    });

    let reply = h.submit_fetch(&h.fields()).await;
    assert_eq!(reply.contact_error().as_deref(), Some("rejected"));
    let page = h.follow(&h.submit_nojs(&h.fields()).await).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("Your message could not be accepted.")
    );

    assert_eq!([first.calls(), second.calls(), third.calls()], [2, 2, 0]);
    assert_eq!(h.deliveries(), 0);
}
