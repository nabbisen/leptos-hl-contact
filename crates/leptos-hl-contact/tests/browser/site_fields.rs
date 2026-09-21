//! Site-defined fields in the browser (RFC 015 D5, A6): rendering, what is
//! submitted, where an error lands, where focus goes, and what a failed
//! submission leaves in place.  Values are obviously fake.

use leptos_hl_contact::{
    ContactFieldErrors, FieldError, FieldErrorCode, SiteField, SiteFieldChoice, SiteFieldKind,
    SiteFields,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{FetchStub, Mounted, document, settle};

/// One field of each kind, defined as `organisation`, `topic`, `timing`.
fn definition() -> SiteFields {
    SiteFields::new(vec![
        SiteField {
            key: "organisation".into(),
            label: "Organisation".into(),
            kind: SiteFieldKind::Line,
            required: true,
            max_len: 120,
        },
        SiteField {
            key: "topic".into(),
            label: "Topic".into(),
            kind: SiteFieldKind::Choice(vec![
                SiteFieldChoice {
                    key: "sales".into(),
                    label: "Sales".into(),
                },
                SiteFieldChoice {
                    key: "support".into(),
                    label: "Support".into(),
                },
            ]),
            required: true,
            max_len: 0,
        },
        SiteField {
            key: "timing".into(),
            label: "Timing".into(),
            kind: SiteFieldKind::Text,
            required: false,
            max_len: 500,
        },
    ])
    .expect("a valid definition")
}

/// Answers the submission with `errors` and everything else as unexpected.
fn failing_with(errors: ContactFieldErrors) -> FetchStub {
    let body = format!("Args|{}", errors.into_server_fn_message());
    FetchStub::install(move |url| {
        if url.ends_with("/api/submit_contact") {
            (500, body.clone())
        } else {
            (500, "ServerError|unexpected request in test".to_owned())
        }
    })
}

fn error(code: FieldErrorCode) -> FieldError {
    FieldError::Code(code)
}

fn errors_for(keys: &[&str], code: FieldErrorCode) -> ContactFieldErrors {
    ContactFieldErrors {
        site_fields: keys
            .iter()
            .map(|key| ((*key).to_owned(), error(code.clone())))
            .collect(),
        ..ContactFieldErrors::default()
    }
}

fn fill_site_fields(form: &Mounted) {
    form.set_value("fields[organisation]", "Example Co");
    form.set_value("fields[topic]", "support");
    form.set_value("fields[timing]", "Next month");
}

fn element(form: &Mounted, id: &str) -> web_sys::Element {
    form.host
        .query_selector(&format!("#{id}"))
        .expect("selector")
        .unwrap_or_else(|| panic!("no element #{id}"))
}

fn focused_id() -> Option<String> {
    document().active_element().map(|e| e.id())
}

/// FR-UI-01, FR-FIELD-08, RFC 015 D5, A7: the three kinds render as an input,
/// a select and a textarea, named `fields[key]` with `contact-field-{key}`
/// ids, required as defined; the select starts with an empty option shown as
/// "—".
#[wasm_bindgen_test]
async fn the_rows_render_with_their_names_ids_and_required() {
    let form = Mounted::with_site_fields(definition());
    settle().await;

    for (id, name, tag, required) in [
        (
            "contact-field-organisation",
            "fields[organisation]",
            "INPUT",
            true,
        ),
        ("contact-field-topic", "fields[topic]", "SELECT", true),
        ("contact-field-timing", "fields[timing]", "TEXTAREA", false),
    ] {
        let el = element(&form, id);
        assert_eq!(el.tag_name(), tag, "{id}");
        assert_eq!(el.get_attribute("name").as_deref(), Some(name), "{id}");
        assert_eq!(el.has_attribute("required"), required, "{id}: required");
        assert_eq!(
            el.get_attribute("aria-required").as_deref(),
            required.then_some("true"),
            "{id}: aria-required"
        );
        assert!(
            form.host
                .query_selector(&format!(r#"label[for="{id}"]"#))
                .unwrap()
                .is_some(),
            "{id}: its label"
        );
    }

    let options = form
        .host
        .query_selector_all("#contact-field-topic option")
        .expect("selector");
    assert_eq!(options.length(), 3, "the empty option and two choices");
    let first: web_sys::HtmlOptionElement = options.item(0).unwrap().unchecked_into();
    assert_eq!(
        (first.value(), first.text()),
        (String::new(), "—".to_owned())
    );
}

/// FR-FIELD-08, RFC 015 D5, A1: the submitted body carries `fields[key]=value` for each
/// site field the visitor filled, and the built-in fields as before.
#[wasm_bindgen_test]
async fn the_submitted_body_carries_the_site_fields() {
    let fetch = FetchStub::install(|_| (200, "null".to_owned()));
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    let bodies = fetch.bodies();
    assert_eq!(bodies.len(), 1, "one submission: {:?}", fetch.urls());
    let body = &bodies[0];
    for pair in [
        "fields[organisation]=Example+Co",
        "fields[topic]=support",
        "fields[timing]=Next+month",
        "name=Ada+Lovelace",
    ] {
        assert!(body.contains(pair), "{pair} in {body}");
    }
}

/// FR-FIELD-08, FR-UI-08: a field error for `topic` shows under it, in its
/// own paragraph, and marks it invalid and described; no other field is
/// marked, and there is no banner.
#[wasm_bindgen_test]
async fn a_field_error_shows_under_its_field() {
    let _fetch = failing_with(errors_for(&["topic"], FieldErrorCode::Required));
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    let topic = element(&form, "contact-field-topic");
    assert_eq!(topic.get_attribute("aria-invalid").as_deref(), Some("true"));
    assert_eq!(
        topic.get_attribute("aria-describedby").as_deref(),
        Some("contact-field-topic-error")
    );
    let paragraph = element(&form, "contact-field-topic-error");
    assert_eq!(
        paragraph.text_content().as_deref(),
        Some("This field is required.")
    );
    assert_eq!(paragraph.get_attribute("role").as_deref(), Some("alert"));

    for id in ["contact-field-organisation", "contact-field-timing"] {
        assert_eq!(
            element(&form, id).get_attribute("aria-invalid"),
            None,
            "{id}"
        );
    }
    assert!(
        form.host
            .query_selector(r#"[aria-live="assertive"]"#)
            .unwrap()
            .is_none(),
        "no banner for a field error"
    );
}

/// FR-UI-13, FR-A11Y-03: focus goes to the first invalid field in document
/// order, and a site field is before the message and after the subject.
#[wasm_bindgen_test]
async fn focus_goes_to_the_first_invalid_site_field_before_the_message() {
    let errors = ContactFieldErrors {
        message: Some(error(FieldErrorCode::Required)),
        site_fields: [
            ("topic".to_owned(), error(FieldErrorCode::Format)),
            ("organisation".to_owned(), error(FieldErrorCode::Required)),
        ]
        .into(),
        ..ContactFieldErrors::default()
    };
    let _fetch = failing_with(errors);
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    // `organisation` is defined before `topic`, so it is first in the page.
    assert_eq!(focused_id().as_deref(), Some("contact-field-organisation"));
}

/// FR-UI-13: a built-in field before the site fields still wins, and one
/// after them (the message) does not.
#[wasm_bindgen_test]
async fn the_built_in_fields_keep_their_place_around_the_site_fields() {
    let errors = ContactFieldErrors {
        subject: Some(error(FieldErrorCode::Length { min: 0, max: 120 })),
        site_fields: [("organisation".to_owned(), error(FieldErrorCode::Required))].into(),
        ..ContactFieldErrors::default()
    };
    let _fetch = failing_with(errors);
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    assert_eq!(focused_id().as_deref(), Some("contact-subject"));
}

/// FR-UI-07, RFC 015 A6: after a failed submission the typed `organisation`
/// and the selected `topic` are still there, as the built-in fields' values
/// are.
#[wasm_bindgen_test]
async fn a_failed_submission_keeps_what_was_typed_and_selected() {
    let _fetch = failing_with(errors_for(&["timing"], FieldErrorCode::Format));
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    assert_eq!(form.value("fields[organisation]"), "Example Co");
    assert_eq!(form.value("fields[topic]"), "support");
    assert_eq!(form.value("fields[timing]"), "Next month");
    assert_eq!(form.value("name"), "Ada Lovelace");
}

/// FR-FIELD-02, RFC 015 D3: an error for a key this form did not render has no row to
/// show it in, so it shows the generic message: a mismatch between the form
/// and the server is loud, never a dropped error.
#[wasm_bindgen_test]
async fn an_error_for_an_unrendered_key_shows_the_generic_message() {
    let _fetch = failing_with(errors_for(&["stray"], FieldErrorCode::Required));
    let form = Mounted::with_site_fields(definition());
    settle().await;

    form.fill_valid();
    fill_site_fields(&form);
    form.submit();
    settle().await;

    let banner = form
        .host
        .query_selector(r#"[aria-live="assertive"]"#)
        .unwrap()
        .expect("the generic banner");
    assert_eq!(
        banner.text_content().as_deref(),
        Some("Failed to send message. Please try again later.")
    );
    assert!(
        form.host
            .query_selector(r#"[aria-invalid="true"]"#)
            .unwrap()
            .is_none(),
        "no field is marked"
    );
}

/// FR-UI-01: a form with no site fields renders no site control.
#[wasm_bindgen_test]
async fn a_form_without_site_fields_renders_none() {
    let form = Mounted::new(leptos_hl_contact::ContactFormOptions::default());
    settle().await;

    assert!(
        form.host
            .query_selector(r#"[name^="fields["]"#)
            .unwrap()
            .is_none()
    );
}
