//! `ContactFormOptions::native_validation` (RFC 019 D1-D4).

use leptos_hl_contact::{ContactFieldErrors, ContactFormOptions, FieldError, FieldErrorCode};
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{FetchStub, Mounted, document, settle};

/// FR-UI-02, FR-I18N-02, FR-PE-01/02: with `native_validation: false`, the
/// browser's own prompting never runs, so submitting the form with its
/// required fields left empty still reaches the server function — and the
/// site's own error text appears under the first invalid field, with
/// `aria-invalid` and focus there, exactly as a server-rejected submission
/// does with the default (`focus::focus_moves_to_the_first_invalid_field`
/// covers that the default's behaviour is unchanged; this test is the
/// `false` side the default cannot reach, since the browser blocks an empty
/// required field before `submit` ever fires).
#[wasm_bindgen_test]
async fn an_empty_required_field_reaches_the_server_with_novalidate() {
    let errors = ContactFieldErrors {
        name: Some(FieldError::Code(FieldErrorCode::Required)),
        email: Some(FieldError::Code(FieldErrorCode::Required)),
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
    let form = Mounted::new(ContactFormOptions {
        native_validation: false,
        ..ContactFormOptions::default()
    });
    settle().await;

    // Every required field left empty: the browser's own validation, were
    // it running, would refuse to submit and `fetch` would never be called.
    form.submit();
    settle().await;

    assert_eq!(
        fetch.count("/api/submit_contact"),
        1,
        "novalidate must let an empty required field reach the server: {:?}",
        fetch.urls()
    );
    assert_eq!(
        document().active_element().map(|e| e.id()).as_deref(),
        Some("contact-name"),
        "focus on the first invalid field in form order"
    );
    let name = form.control("name");
    assert_eq!(name.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        name.get_attribute("aria-describedby").as_deref(),
        Some("contact-name-error")
    );
}
