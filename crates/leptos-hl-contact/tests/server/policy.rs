//! Server policy.

use leptos_hl_contact::{ContactServerPolicy, FieldError, FieldErrorCode};

use crate::support::{Harness, Setup};

fn policy(require_subject: bool, max_message_len: usize) -> Setup {
    Setup {
        policy: Some(ContactServerPolicy {
            require_subject,
            max_message_len,
        }),
        ..Setup::default()
    }
}

/// FR-SUB-07, FR-PE-04: `require_subject` is enforced on the server whatever
/// the form sent, in both request forms.
#[tokio::test]
async fn server_policy_requires_the_subject() {
    let h = Harness::new(policy(true, 4000));
    let without = h.fields().without("subject");

    let fetch = h.submit_fetch(&without).await;
    let errors = fetch.field_errors().expect("field errors");
    assert_eq!(
        errors.subject,
        Some(FieldError::Code(FieldErrorCode::Required))
    );

    let nojs = h.submit_nojs(&without).await;
    assert!(nojs.is_nojs_error());
    let page = h.follow(&nojs).await;
    assert!(page.html.contains("This field is required."));
    assert_eq!(page.aria_invalid_count(), 1);
    assert_eq!(h.deliveries(), 0);

    assert!(h.submit_fetch(&h.fields()).await.status.is_success());
    assert!(h.submit_nojs(&h.fields()).await.is_nojs_success());
    assert_eq!(h.deliveries(), 2);
}

/// FR-VAL-07, FR-PE-04: the policy's message limit counts characters, not
/// bytes — ten two-byte characters fit a limit of ten, eleven do not — in both
/// request forms.
#[tokio::test]
async fn server_policy_counts_the_message_in_characters() {
    let h = Harness::new(policy(false, 10));
    let at_limit = h.fields().set("message", &"é".repeat(10));
    let over = h.fields().set("message", &"é".repeat(11));

    assert!(h.submit_fetch(&at_limit).await.status.is_success());
    assert!(h.submit_nojs(&at_limit).await.is_nojs_success());
    assert_eq!(h.deliveries(), 2);

    let fetch = h.submit_fetch(&over).await;
    let errors = fetch.field_errors().expect("field errors");
    assert!(
        matches!(
            errors.message,
            Some(FieldError::Code(FieldErrorCode::Length { max: 10, .. }))
        ),
        "{:?}",
        errors.message
    );
    let nojs = h.submit_nojs(&over).await;
    assert!(nojs.is_nojs_error());
    let page = h.follow(&nojs).await;
    assert!(
        page.html.contains("and 10 characters."),
        "the length label names the limit"
    );
    assert_eq!(page.aria_invalid_count(), 1);
    assert_eq!(h.deliveries(), 2);
}
