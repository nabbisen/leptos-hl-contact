//! A `ContactForm` mounted into the test page, and letting the page settle.

use leptos::{
    mount::{UnmountHandle, mount_to},
    prelude::*,
    tachys::view::any_view::AnyViewState,
};
use leptos_hl_contact::{ChallengeWidget, ContactForm, ContactFormOptions};
use wasm_bindgen::{JsCast, JsValue};

pub fn document() -> web_sys::Document {
    web_sys::window()
        .and_then(|w| w.document())
        .expect("document")
}

/// A `ContactForm` in its own element of the test page; unmounted and
/// removed when dropped.
///
/// `mount_to` gives the form an owner with no shared context, so it is not
/// hydrating: the effects run as they do in a client after navigation.
pub struct Mounted {
    pub host: web_sys::HtmlElement,
    _handle: UnmountHandle<AnyViewState>,
}

impl Mounted {
    pub fn new(options: ContactFormOptions) -> Self {
        Self::mount(move || view! { <ContactForm options=options /> }.into_any())
    }

    pub fn with_challenge(widget: ChallengeWidget) -> Self {
        Self::mount(move || view! { <ContactForm challenge=Some(widget) /> }.into_any())
    }

    fn mount(view: impl FnOnce() -> AnyView + 'static) -> Self {
        let host: web_sys::HtmlElement = document()
            .create_element("div")
            .expect("div")
            .unchecked_into();
        document()
            .body()
            .expect("body")
            .append_child(&host)
            .expect("append");
        let handle = mount_to(host.clone(), view);
        Self {
            host,
            _handle: handle,
        }
    }

    /// The form control named `name`.
    pub fn control(&self, name: &str) -> web_sys::Element {
        self.host
            .query_selector(&format!("[name=\"{name}\"]"))
            .expect("selector")
            .unwrap_or_else(|| panic!("no control named {name}"))
    }

    pub fn value(&self, name: &str) -> String {
        js_sys::Reflect::get(&self.control(name), &JsValue::from_str("value"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    }

    pub fn set_value(&self, name: &str, value: &str) {
        js_sys::Reflect::set(
            &self.control(name),
            &JsValue::from_str("value"),
            &JsValue::from_str(value),
        )
        .expect("set value");
    }

    pub fn token(&self) -> String {
        self.value("form_token")
    }

    /// Values the browser's own constraint validation accepts, so a submit
    /// reaches the server function.
    pub fn fill_valid(&self) {
        self.set_value("name", "Ada Lovelace");
        self.set_value("email", "ada@example.com");
        self.set_value("message", "A message typed by a person.");
    }

    /// Submits as a visitor does: `requestSubmit` runs validation and fires
    /// `submit`, which `ActionForm` handles.
    pub fn submit(&self) {
        self.host
            .query_selector("form")
            .expect("selector")
            .expect("a form")
            .unchecked_into::<web_sys::HtmlFormElement>()
            .request_submit()
            .expect("requestSubmit");
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        self.host.remove();
    }
}

/// Lets the page finish what it has started: pending promises, and browser
/// tasks, which is when a `Response` body becomes readable.  A task is
/// awaited with a `MessageChannel` message, not a timer, so this never waits
/// on time and keeps working while `Clock` replaces `setTimeout`.
pub async fn settle() {
    for _ in 0..3 {
        microtasks().await;
        next_task().await;
    }
    microtasks().await;
}

async fn microtasks() {
    for _ in 0..20 {
        wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL))
            .await
            .expect("resolved");
    }
}

async fn next_task() {
    let channel = web_sys::MessageChannel::new().expect("MessageChannel");
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        channel.port1().set_onmessage(Some(&resolve));
        channel
            .port2()
            .post_message(&JsValue::NULL)
            .expect("postMessage");
    });
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("message");
    channel.port1().set_onmessage(None);
}
