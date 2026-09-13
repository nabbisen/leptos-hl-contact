//! The rendered markup's accessibility and limit attributes: what assistive
//! technology and the browser read before any script runs (External Design
//! §4.1.2, the DOM contract).

use leptos::prelude::*;

use super::{count, render};
use crate::{components::ContactForm, config::ContactFormOptions, model::MESSAGE_MAX_LEN};

const VISIBLE_CONTROLS: [&str; 4] = [
    "contact-name",
    "contact-email",
    "contact-subject",
    "contact-message",
];

fn form(options: ContactFormOptions) -> String {
    render(move || view! { <ContactForm options=options /> }.into_any())
}

/// The start tag of the element whose `id` is exactly `id`.
fn tag<'a>(html: &'a str, id: &str) -> &'a str {
    let at = html
        .find(&format!("id=\"{id}\""))
        .unwrap_or_else(|| panic!("no id=\"{id}\" in:\n{html}"));
    let start = html[..at].rfind('<').expect("tag start");
    let end = at + html[at..].find('>').expect("tag end");
    &html[start..=end]
}

/// Whether `tag` carries the attribute `name`, with or without a value.
fn has_attribute(tag: &str, name: &str) -> bool {
    tag.trim_start_matches('<')
        .trim_end_matches('>')
        .split_whitespace()
        .any(|token| token == name || token.starts_with(&format!("{name}=")))
}

/// The quoted value of the attribute `name` on `tag`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let at = tag.find(&key)? + key.len();
    let end = tag[at..].find('"')?;
    Some(&tag[at..at + end])
}

/// FR-A11Y-01: every visible control has exactly one `<label for>` naming
/// its `id`, and no control relies on a placeholder instead.
#[test]
fn every_input_has_a_label_for_it() {
    let html = form(ContactFormOptions::default());
    for id in VISIBLE_CONTROLS {
        tag(&html, id);
        assert_eq!(
            count(&html, &format!(" for=\"{id}\"")),
            1,
            "one label for {id} in:\n{html}"
        );
    }
    assert!(!html.contains("placeholder"), "a placeholder in:\n{html}");
}

/// FR-A11Y-02, FR-UI-01, FR-UI-11: `name`, `email` and `message` carry both
/// `required` and `aria-required="true"`; `subject` carries neither by
/// default, and both with `require_subject`.
#[test]
fn required_fields_carry_both_required_attributes() {
    let html = form(ContactFormOptions::default());
    for id in ["contact-name", "contact-email", "contact-message"] {
        let control = tag(&html, id);
        assert!(has_attribute(control, "required"), "{control}");
        assert_eq!(
            attribute(control, "aria-required"),
            Some("true"),
            "{control}"
        );
    }
    let subject = tag(&html, "contact-subject");
    assert!(!has_attribute(subject, "required"), "{subject}");
    assert_ne!(
        attribute(subject, "aria-required"),
        Some("true"),
        "{subject}"
    );

    let html = form(ContactFormOptions {
        require_subject: true,
        ..ContactFormOptions::default()
    });
    let subject = tag(&html, "contact-subject");
    assert!(has_attribute(subject, "required"), "{subject}");
    assert_eq!(
        attribute(subject, "aria-required"),
        Some("true"),
        "{subject}"
    );
}

/// FR-UI-10, FR-A11Y-06: the honeypot input is out of the tab order and
/// autofill, and sits inside an `aria-hidden` wrapper placed off screen.
#[test]
fn the_honeypot_is_hidden_from_everyone() {
    let html = form(ContactFormOptions::default());
    let input = tag(&html, "contact-website");
    assert_eq!(attribute(input, "name"), Some("website"), "{input}");
    assert_eq!(attribute(input, "tabindex"), Some("-1"), "{input}");
    assert_eq!(attribute(input, "autocomplete"), Some("off"), "{input}");

    let at = html.find("id=\"contact-website\"").expect("the honeypot");
    let start = html[..at].rfind("<div").expect("a wrapper");
    let wrapper = &html[start..=start + html[start..].find('>').expect("tag end")];
    assert_eq!(attribute(wrapper, "aria-hidden"), Some("true"), "{wrapper}");
    let style = attribute(wrapper, "style").unwrap_or_default();
    assert!(
        style.contains("position:absolute") && style.contains("left:-9999px"),
        "{wrapper}"
    );
    assert!(
        !html[start..at].contains("</div>"),
        "the input is inside the wrapper"
    );
}

/// FR-VAL-07, FR-VAL-08: `maxlength` agrees with the validator — 80 on
/// `name`, 120 on `subject` — and on `message` is the effective length, with
/// a UI option above the ceiling clamped to it.
#[test]
fn maxlength_matches_the_validator() {
    let html = form(ContactFormOptions::default());
    assert_eq!(
        attribute(tag(&html, "contact-name"), "maxlength"),
        Some("80")
    );
    assert_eq!(
        attribute(tag(&html, "contact-subject"), "maxlength"),
        Some("120")
    );
    let default_len = ContactFormOptions::default()
        .effective_max_message_len()
        .to_string();
    assert_eq!(
        attribute(tag(&html, "contact-message"), "maxlength"),
        Some(default_len.as_str())
    );

    for (asked, rendered) in [(500, 500), (MESSAGE_MAX_LEN + 1000, MESSAGE_MAX_LEN)] {
        let html = form(ContactFormOptions {
            max_message_len: asked,
            ..ContactFormOptions::default()
        });
        assert_eq!(
            attribute(tag(&html, "contact-message"), "maxlength"),
            Some(rendered.to_string().as_str()),
            "max_message_len = {asked}"
        );
    }
}
