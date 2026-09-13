//! Field validation and how errors reach the visitor.

use leptos_hl_contact::{FieldError, FieldErrorCode};

use crate::support::{Harness, Setup};

/// FR-SUB-05, FR-SUB-06, FR-VAL-01, FR-VAL-02, FR-VAL-03, FR-VAL-04,
/// FR-VAL-07, FR-PE-04, NFR-SEC-02: each rule rejects with its own field
/// code, in both request forms, and nothing is delivered.
#[tokio::test]
async fn each_rule_rejects_with_its_field_code() {
    let h = Harness::new(Setup::default());
    let code = |c| Some(FieldError::Code(c));
    let long_email = format!("{}@example.com", "a".repeat(243));

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
        // A single-label domain, and one character over the SMTP path limit.
        ("email", "abc@bar", "email", code(FieldErrorCode::Format)),
        (
            "email",
            &long_email,
            "email",
            code(FieldErrorCode::Length { min: 0, max: 254 }),
        ),
        // Header injection through the name.
        (
            "name",
            "a\r\nBcc: x@example.com",
            "name",
            code(FieldErrorCode::LineBreaks),
        ),
        (
            "subject",
            "line one\nline two",
            "subject",
            code(FieldErrorCode::LineBreaks),
        ),
        (
            "subject",
            &"s".repeat(121),
            "subject",
            code(FieldErrorCode::Length { min: 0, max: 120 }),
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

/// FR-VAL-03: a blank subject is treated as absent, not rejected.
#[tokio::test]
async fn a_blank_subject_is_delivered_as_absent() {
    let h = Harness::new(Setup::default());

    let reply = h.submit_fetch(&h.fields().set("subject", "   ")).await;
    assert!(reply.status.is_success(), "{}", reply.body);
    assert_eq!(h.deliveries(), 1);
    assert_eq!(h.delivery.last().expect("delivered").subject, None);
}

/// FR-PE-02, FR-UI-08, FR-VAL-02: without JavaScript a field error comes back
/// through the redirect: `302`, and the page it lands on renders the label's
/// text next to the field with `aria-invalid="true"` — and no assertive
/// banner, because a field error never also raises the banner.  Shown for a
/// malformed address and for a single-label domain, which the browser's own
/// `type="email"` check lets through.
#[tokio::test]
async fn field_errors_round_trip_without_javascript() {
    let h = Harness::new(Setup::default());

    for email in ["not-an-email", "abc@bar"] {
        let reply = h.submit_nojs(&h.fields().set("email", email)).await;
        assert_eq!(reply.status.as_u16(), 302, "{email}");
        assert!(
            reply.location().is_some_and(|l| l.contains("__err")),
            "{email}: {:?}",
            reply.location()
        );

        let page = h.follow(&reply).await;
        assert_eq!(page.status.as_u16(), 200, "{email}");
        assert!(
            page.html.contains("Enter a valid email address."),
            "{email}: the email label's text is rendered"
        );
        assert_eq!(
            page.aria_invalid_count(),
            1,
            "{email}: exactly the email field"
        );
        assert_eq!(page.banner(), None, "{email}: no banner for a field error");
    }
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
