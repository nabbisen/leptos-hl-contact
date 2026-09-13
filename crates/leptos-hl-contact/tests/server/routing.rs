//! Configuration and routing.

use leptos_hl_contact::{ContactServerPolicy, FieldError, FieldErrorCode};

use crate::support::{Harness, Setup};

/// RFC 007, FR-CFG-02, T5: context provided only in the one closure passed to
/// `leptos_routes_with_context` reaches `submit_contact`, with no
/// hand-written server-function route.  The delivery context carries a valid
/// submission to delivery, and the server policy from the same closure is
/// enforced on an invalid one.
#[tokio::test]
async fn context_in_the_one_closure_reaches_submit_contact() {
    let h = Harness::new(Setup {
        policy: Some(ContactServerPolicy {
            require_subject: true,
            max_message_len: 4000,
        }),
        ..Setup::default()
    });

    let delivered = h.submit_fetch(&h.fields()).await;
    assert!(delivered.status.is_success(), "{}", delivered.body);
    assert!(h.submit_nojs(&h.fields()).await.is_nojs_success());
    assert_eq!(h.deliveries(), 2);
    assert_eq!(
        h.delivery.last().expect("delivered").email,
        "ada@example.com"
    );

    let refused = h.submit_fetch(&h.fields().set("subject", "")).await;
    let errors = refused.field_errors().expect("field errors");
    assert_eq!(
        errors.subject,
        Some(FieldError::Code(FieldErrorCode::Required)),
        "the policy provided in the closure was applied"
    );
    let page = h
        .follow(&h.submit_nojs(&h.fields().set("subject", "")).await)
        .await;
    assert!(
        page.html.contains("This field is required."),
        "and without JavaScript"
    );
    assert_eq!(h.deliveries(), 2);
}

/// FR-SUB-08, FR-CFG-03: no delivery context fails closed with the generic
/// configuration message, in both request forms.
#[tokio::test]
async fn a_missing_delivery_context_is_not_configured() {
    let h = Harness::new(Setup {
        delivery: false,
        ..Setup::default()
    });

    let fetch = h.submit_fetch(&h.fields()).await;
    assert_eq!(fetch.contact_error().as_deref(), Some("not_configured"));
    assert!(fetch.is_server_error(), "{}", fetch.body);

    let nojs = h.submit_nojs(&h.fields()).await;
    assert_eq!(nojs.status.as_u16(), 302);
    let page = h.follow(&nojs).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("This form is not available right now.")
    );
}
