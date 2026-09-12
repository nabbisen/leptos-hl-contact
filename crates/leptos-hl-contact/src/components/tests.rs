// tests.rs — unit tests for the parent module.
//
// These render the component to a string on the server.  They cover the DOM
// contract (External Design §4.1.2), which is public API; the reactive
// behaviour under hydration is covered by the browser evidence in the
// handoff's review request.

use leptos::prelude::*;
use leptos::reactive::owner::Owner;

use crate::{components::ContactForm, config::ContactFormOptions};

/// Render a view to HTML inside a fresh reactive ownership scope.
///
/// Leptos 0.8 has no `leptos::ssr::render_to_string`; `RenderHtml::to_html`
/// is the equivalent, and the owner is what makes `provide_context` inside
/// the closure visible to the component.
fn render(f: impl FnOnce() -> AnyView) -> String {
    let owner = Owner::new();
    owner.set();
    let html = f().to_html();
    drop(owner);
    html
}

/// Count non-overlapping occurrences of `needle` in `haystack`.
fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn form_renders_all_ids_once() {
    let html = render(|| view! { <ContactForm /> }.into_any());

    for id in [
        "contact-name",
        "contact-email",
        "contact-subject",
        "contact-message",
        "contact-website",
    ] {
        assert_eq!(
            count(&html, &format!("id=\"{id}\"")),
            1,
            "expected exactly one id=\"{id}\" in:\n{html}"
        );
    }

    assert_eq!(
        count(&html, "name=\"form_token\""),
        1,
        "expected exactly one hidden token field"
    );
}

#[cfg(feature = "form-token")]
#[test]
fn hidden_token_is_rendered_from_context() {
    use crate::form_token::FormToken;

    let html = render(|| {
        leptos::context::provide_context(FormToken("test-token-value".into()));
        view! { <ContactForm /> }.into_any()
    });

    assert!(
        html.contains("value=\"test-token-value\""),
        "token from context must reach the hidden field:\n{html}"
    );
}

#[test]
fn subject_hidden_when_option_false() {
    let options = ContactFormOptions {
        show_subject: false,
        ..Default::default()
    };
    let html = render(move || view! { <ContactForm options=options /> }.into_any());

    assert!(
        !html.contains("contact-subject"),
        "subject field must not be rendered when show_subject is false:\n{html}"
    );
    // The other fields are still present.
    assert!(html.contains("id=\"contact-name\""));
    assert!(html.contains("id=\"contact-message\""));
}

/// The hidden field's value is now a reactive attribute.  On the server it
/// must still render the context token into that one field, since hydration
/// keeps whatever the server wrote.
#[cfg(feature = "form-token")]
#[test]
fn the_reactive_token_attribute_renders_the_ssr_value() {
    use crate::form_token::FormToken;

    let html = render(|| {
        leptos::context::provide_context(FormToken("1789232333|nonce|sig".into()));
        view! { <ContactForm /> }.into_any()
    });

    let field = html
        .split('<')
        .find(|tag| tag.contains("name=\"form_token\""))
        .expect("the hidden field is rendered");
    assert!(
        field.contains("value=\"1789232333|nonce|sig\""),
        "the SSR token must be on the hidden field itself: <{field}"
    );
}

/// With no token in context the field is still rendered, empty — which is
/// exactly what tells the browser to fetch one.
#[test]
fn the_token_field_is_empty_without_a_token() {
    let html = render(|| view! { <ContactForm /> }.into_any());

    let field = html
        .split('<')
        .find(|tag| tag.contains("name=\"form_token\""))
        .expect("the hidden field is rendered");
    assert!(
        !field.contains("value=\"") || field.contains("value=\"\""),
        "no token in context must leave the field empty: <{field}"
    );
}
