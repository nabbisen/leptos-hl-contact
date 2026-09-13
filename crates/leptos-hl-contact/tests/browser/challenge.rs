//! Explicit rendering of a challenge widget (RFC 005 handoff 02).

use leptos_hl_contact::{ChallengeProvider, ChallengeWidget};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

use crate::support::{Mounted, VendorStub, document, settle};

/// The script `ChallengeWidget` loads for Turnstile.
const TURNSTILE_SCRIPT: &str = "https://challenges.cloudflare.com/turnstile/v0/api.js";

fn turnstile() -> ChallengeWidget {
    ChallengeWidget::new(ChallengeProvider::Turnstile, "1x00000000000000000000AA")
        .expect("Cloudflare's always-pass test site key")
}

fn scripts(src: &str) -> u32 {
    document()
        .query_selector_all(&format!("script[src=\"{src}\"]"))
        .expect("selector")
        .length()
}

/// A `<script>` placed in `<head>` for one test, removed afterwards.
/// `type="text/plain"` keeps the browser from fetching it.
struct PlantedScript(web_sys::Element);

impl PlantedScript {
    fn new(src: &str) -> Self {
        let script = document().create_element("script").expect("script");
        script.set_attribute("type", "text/plain").expect("type");
        script.set_attribute("src", src).expect("src");
        document()
            .head()
            .expect("head")
            .append_child(&script)
            .expect("append");
        Self(script)
    }
}

impl Drop for PlantedScript {
    fn drop(&mut self) {
        self.0.remove();
    }
}

/// FR-ABUSE-10: a widget mounted after its vendor's global exists (a client
/// navigation to the form) is rendered explicitly, once, into its own
/// element; the script is not requested again.
#[wasm_bindgen_test]
async fn a_widget_mounted_after_its_vendor_global_renders_once() {
    let vendor = VendorStub::install("turnstile");
    let form = Mounted::with_challenge(turnstile());

    settle().await;

    let rendered = vendor.rendered();
    assert_eq!(rendered.len(), 1);
    let element = rendered[0]
        .dyn_ref::<web_sys::Node>()
        .expect("render is given an element");
    assert!(form.host.contains(Some(element)), "the form's own widget");
    assert_eq!(scripts(TURNSTILE_SCRIPT), 0);
}

/// FR-ABUSE-10: when the vendor script is already in `<head>` but its
/// global is not ready yet, mounting the widget does not insert it again.
#[wasm_bindgen_test]
async fn a_vendor_script_already_in_head_is_not_inserted_again() {
    let _planted = PlantedScript::new(TURNSTILE_SCRIPT);
    let _form = Mounted::with_challenge(turnstile());

    settle().await;

    assert_eq!(scripts(TURNSTILE_SCRIPT), 1);
}
