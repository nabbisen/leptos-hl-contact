//! Focus and field state after a failed submission.

use leptos_hl_contact::{ContactFieldErrors, ContactFormOptions, FieldError, FieldErrorCode};
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{FetchStub, Mounted, document, settle};

/// FR-UI-13, FR-A11Y-03, FR-UI-07: after a field-error payload, focus moves
/// to the first invalid field in form order, that field is marked invalid and
/// points at its error text, and what the visitor typed is still there.
#[wasm_bindgen_test]
async fn focus_moves_to_the_first_invalid_field() {
    let errors = ContactFieldErrors {
        email: Some(FieldError::Code(FieldErrorCode::Format)),
        message: Some(FieldError::Code(FieldErrorCode::Required)),
        ..ContactFieldErrors::default()
    };
    let body = format!("Args|{}", errors.into_server_fn_message());
    let fetch = FetchStub::install(move |url| {
        if url.ends_with("/api/submit_contact") {
            (500, body.clone())
        } else {
            (500, "ServerError|unexpected request in test".to_owned())
        }
    });
    let form = Mounted::new(ContactFormOptions::default());
    settle().await;

    form.fill_valid();
    form.submit();
    settle().await;

    assert_eq!(fetch.count("/api/submit_contact"), 1, "{:?}", fetch.urls());
    assert_eq!(
        document().active_element().map(|e| e.id()).as_deref(),
        Some("contact-email")
    );
    let email = form.control("email");
    assert_eq!(email.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        email.get_attribute("aria-describedby").as_deref(),
        Some("contact-email-error")
    );
    for (name, typed) in [
        ("name", "Ada Lovelace"),
        ("email", "ada@example.com"),
        ("message", "A message typed by a person."),
    ] {
        assert_eq!(form.value(name), typed, "{name} kept");
    }
}
