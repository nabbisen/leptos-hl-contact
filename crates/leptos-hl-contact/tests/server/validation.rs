//! Field validation and how errors reach the visitor.

use leptos_hl_contact::{FieldError, FieldErrorCode};

use crate::support::{Harness, Setup};

/// FR-SUB-05, FR-SUB-06, FR-VAL-02, FR-VAL-03, FR-PE-04: each rule rejects
/// with its own field code, in both request forms, and nothing is delivered.
#[tokio::test]
async fn each_rule_rejects_with_its_field_code() {
    let h = Harness::new(Setup::default());
    let code = |c| Some(FieldError::Code(c));

    let cases = [
        ("name", "", "name", code(FieldErrorCode::Required)),
        (
            "name",
            &"x".repeat(81),
            "name",
            code(FieldErrorCode::Length { min: 1, max: 80 }),
        ),
        (
            "email",
            "not-an-email",
            "email",
            code(FieldErrorCode::Format),
        ),
        (
            "subject",
            "line one\nline two",
            "subject",
            code(FieldErrorCode::LineBreaks),
        ),
        ("message", "", "message", code(FieldErrorCode::Required)),
        (
            "message",
            &"é".repeat(4001),
            "message",
            code(FieldErrorCode::Length { min: 1, max: 4000 }),
        ),
    ];
    for (field, value, reported, expected) in cases {
        let reply = h.submit_fetch(&h.fields().set(field, value)).await;
        let errors = reply
            .field_errors()
            .unwrap_or_else(|| panic!("{field}: no field errors in {}", reply.body));
        let actual = match reported {
            "name" => errors.name,
            "email" => errors.email,
            "subject" => errors.subject,
            _ => errors.message,
        };
        assert_eq!(actual, expected, "{field}={value:.20}, fetch");

        // Without JavaScript: the same field, and only it, is marked invalid.
        let nojs = h.submit_nojs(&h.fields().set(field, value)).await;
        assert!(nojs.is_nojs_error(), "{field}, no-JS");
        let page = h.follow(&nojs).await;
        assert_eq!(page.aria_invalid_count(), 1, "{field}, no-JS");
        assert_eq!(page.banner(), None, "{field}, no-JS");
    }
    assert_eq!(h.deliveries(), 0);
}

/// FR-PE-02, FR-UI-08: without JavaScript a field error comes back through
/// the redirect: `302`, and the page it lands on renders the label's text next
/// to the field with `aria-invalid="true"` — and no assertive banner, because
/// a field error never also raises the banner.
#[tokio::test]
async fn field_errors_round_trip_without_javascript() {
    let h = Harness::new(Setup::default());

    let reply = h
        .submit_nojs(&h.fields().set("email", "not-an-email"))
        .await;
    assert_eq!(reply.status.as_u16(), 302);
    assert!(
        reply.location().is_some_and(|l| l.contains("__err")),
        "{:?}",
        reply.location()
    );

    let page = h.follow(&reply).await;
    assert_eq!(page.status.as_u16(), 200);
    assert!(
        page.html.contains("Enter a valid email address."),
        "the email label's text is rendered"
    );
    assert_eq!(page.aria_invalid_count(), 1, "exactly the email field");
    assert_eq!(page.banner(), None, "no banner for a field error");
    assert_eq!(h.deliveries(), 0);
}

/// FR-UI-09: a whole-submission error raises the banner and marks no field
/// invalid.
#[tokio::test]
async fn a_banner_error_sets_no_field_error() {
    let h = Harness::new(Setup::default());

    let reply = h
        .submit_nojs(&h.fields().set("form_token", "not-a-token"))
        .await;
    assert_eq!(reply.status.as_u16(), 302);

    let page = h.follow(&reply).await;
    assert_eq!(
        page.banner().as_deref(),
        Some("Your session token expired. Please reload the page and try again.")
    );
    assert_eq!(page.aria_invalid_count(), 0);
    assert_eq!(h.deliveries(), 0);
}
