//! The happy path.

use crate::support::{Harness, Setup};

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
