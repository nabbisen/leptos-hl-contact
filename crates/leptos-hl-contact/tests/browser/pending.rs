//! The submit button while a submission is in flight.

use leptos_hl_contact::ContactFormLabels;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{FetchStub, Mounted, settle};

/// The button's `disabled` property, `aria-busy` attribute and text.
fn button_state(form: &Mounted) -> (bool, Option<String>, String) {
    let button = form.submit_button();
    let disabled = js_sys::Reflect::get(&button, &JsValue::from_str("disabled"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    (
        disabled,
        button.get_attribute("aria-busy"),
        button.text_content().unwrap_or_default(),
    )
}

/// FR-UI-05, FR-A11Y-05, FR-UI-02: while a submission is in flight the
/// submit button is disabled, announces `aria-busy="true"` and shows the
/// `sending` label; before it, the button is enabled and shows `submit`.
/// Both texts are custom labels, so they come from `ContactFormLabels`.
#[wasm_bindgen_test]
async fn the_submit_button_is_busy_while_sending() {
    let fetch = FetchStub::never_answering("/api/submit_contact");
    let form = Mounted::with_labels(ContactFormLabels {
        submit: "Send it".to_owned(),
        sending: "Sending it now".to_owned(),
        ..ContactFormLabels::default()
    });
    settle().await;

    assert_eq!(
        button_state(&form),
        (false, Some("false".to_owned()), "Send it".to_owned()),
        "before the submit"
    );

    form.fill_valid();
    form.submit();
    settle().await;

    assert_eq!(fetch.count("/api/submit_contact"), 1, "{:?}", fetch.urls());
    assert_eq!(
        button_state(&form),
        (true, Some("true".to_owned()), "Sending it now".to_owned()),
        "while sending"
    );
}
