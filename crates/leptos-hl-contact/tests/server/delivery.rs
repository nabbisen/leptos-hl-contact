//! The happy path.

use crate::support::{DELIVERY_ERROR_DETAIL, Harness, Setup};

/// FR-PE-01, FR-PE-04: a valid submission is delivered exactly once and
/// answers success, in both request forms; without a success page the
/// no-JavaScript form returns to the page it came from.
#[tokio::test]
async fn a_valid_submission_is_delivered_once_in_both_forms() {
    let h = Harness::new(Setup::default());

    let fetch = h.submit_fetch(&h.fields()).await;
    assert!(fetch.status.is_success(), "{}", fetch.body);
    assert_eq!(fetch.contact_error(), None);
    assert_eq!(h.deliveries(), 1);

    let nojs = h.submit_nojs(&h.fields()).await;
    assert_eq!(nojs.status.as_u16(), 302);
    // No success page: back to the page the form came from.  The framework
    // appends an empty query, and there is no `__err` in it.
    assert_eq!(
        nojs.location().as_deref(),
        Some("http://localhost/contact?")
    );
    assert_eq!(h.deliveries(), 2);
    assert!(h.redirects.lock().unwrap().is_empty());
}

/// FR-UI-06, FR-PE-03: with a success page configured, both request forms
/// land on it — a `302` without JavaScript, the redirect header with it.
#[tokio::test]
async fn the_success_page_is_applied_when_configured() {
    let h = Harness::new(Setup {
        success_page: true,
        ..Setup::default()
    });

    let nojs = h.submit_nojs(&h.fields()).await;
    assert_eq!(nojs.status.as_u16(), 302);
    assert_eq!(nojs.location().as_deref(), Some("/thanks"));

    let fetch = h.submit_fetch(&h.fields()).await;
    assert!(fetch.status.is_success());
    assert_eq!(fetch.location().as_deref(), Some("/thanks"));
    assert!(fetch.redirect_header().is_some(), "serverfnredirect is set");

    assert_eq!(h.deliveries(), 2);
    assert_eq!(*h.redirects.lock().unwrap(), ["/thanks", "/thanks"]);
}

/// FR-SUB-09, NFR-TEST-03: a delivery error reaches the client only as the
/// generic `delivery_failed` message, in both request forms.  The transport
/// detail stays on the server.
#[tokio::test]
async fn a_delivery_error_reaches_the_client_only_as_delivery_failed() {
    let h = Harness::new(Setup {
        failing_delivery: true,
        ..Setup::default()
    });

    let fetch = h.submit_fetch(&h.fields()).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("delivery_failed"));
    assert!(!fetch.body.contains("relay said no"), "{}", fetch.body);

    let nojs = h.submit_nojs(&h.fields()).await;
    assert!(nojs.is_nojs_error(), "{:?}", nojs.location());
    let page = h.follow(&nojs).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("Failed to send message. Please try again later.")
    );
    assert!(!page.html.contains(DELIVERY_ERROR_DETAIL));
    assert!(!nojs.location().unwrap_or_default().contains("relay"));

    assert_eq!(h.failing.count(), 2);
    assert_eq!(h.deliveries(), 0);
}
