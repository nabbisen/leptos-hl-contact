//! Site-defined fields in the rendered markup (RFC 015 D5, A6, A7).
//!
//! Values in these tests are obviously fake, and labels are plain words: a
//! real field value is personal data and never belongs in a test's output.

use leptos::prelude::*;

use super::{count, render};
use crate::{
    components::ContactForm,
    config::{
        ContactFormClasses, ContactFormOptions, SiteField, SiteFieldChoice, SiteFieldKind,
        SiteFields,
    },
};

/// What a form with no site fields rendered in 0.7, byte for byte, as
/// `<ContactForm />` on the server with no form token in context.
const MARKUP_0_7: &str = r#"<div class=""><!><!><form action="/api/submit_contact" method="post"><div class=""><label for="contact-name" class="">Name</label><input id="contact-name" name="name" type="text" required maxlength="80" autocomplete="name" aria-required="true" class=""><!></div><div class=""><label for="contact-email" class="">Email</label><input id="contact-email" name="email" type="email" required maxlength="254" autocomplete="email" aria-required="true" class=""><!></div><div class=""><label for="contact-subject" class="">Subject</label><input id="contact-subject" name="subject" type="text" maxlength="120" aria-required="false" class=""><!></div><div class=""><label for="contact-message" class="">Message</label><textarea id="contact-message" name="message" required maxlength="4000" rows="6" aria-required="true" class=""></textarea><!></div><!><input type="hidden" name="form_token" value=""><div aria-hidden="true" style="position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden;"><label for="contact-website">Leave this field blank</label><input id="contact-website" name="website" type="text" tabindex="-1" autocomplete="off"></div><div class=""><button type="submit" aria-busy="false" class="">Send</button></div></form></div>"#;

/// The three-field example: one field of each kind.  Defined in an order
/// (`organisation`, `topic`, `timing`) that is not alphabetical.
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

fn form(fields: SiteFields) -> String {
    render(move || view! { <ContactForm site_fields=fields /> }.into_any())
}

/// The start tag of the element with this `id`.
fn tag<'a>(html: &'a str, id: &str) -> &'a str {
    let start = html
        .find(&format!("id=\"{id}\""))
        .unwrap_or_else(|| panic!("no element with id {id} in:\n{html}"));
    let open = html[..start].rfind('<').expect("a start tag");
    let close = html[start..].find('>').expect("the tag ends") + start + 1;
    &html[open..close]
}

/// The `<select>` with this `id`, from its start tag to its end tag.
fn select<'a>(html: &'a str, id: &str) -> &'a str {
    let open = html.find(tag(html, id)).expect("the tag is in the html");
    let end = html[open..].find("</select>").expect("an end tag") + open;
    &html[open..end]
}

/// FR-FIELD-01, RFC 015 "Compatibility": a site that defines no fields renders
/// exactly the markup 0.7 did, hydration markers included.  Both ways of
/// having none — the default and an explicit empty definition — are pinned.
#[test]
fn no_site_fields_render_the_markup_of_0_7_byte_for_byte() {
    assert_eq!(
        render(|| view! { <ContactForm /> }.into_any()),
        MARKUP_0_7,
        "the default"
    );
    assert_eq!(form(SiteFields::empty()), MARKUP_0_7, "an empty definition");
}

/// D5: the three kinds render as an `<input type="text">`, a `<select>` and a
/// `<textarea>`, with `fields[key]` names and `contact-field-{key}` ids
/// (A7), each with its own label.
#[test]
fn each_kind_renders_its_control_with_its_name_and_id() {
    let html = form(definition());

    let organisation = tag(&html, "contact-field-organisation");
    assert!(organisation.starts_with("<input "), "{organisation}");
    assert!(organisation.contains(r#"name="fields[organisation]""#));
    assert!(organisation.contains(r#"type="text""#));
    assert!(organisation.contains(r#"maxlength="120""#));

    let topic = tag(&html, "contact-field-topic");
    assert!(topic.starts_with("<select "), "{topic}");
    assert!(topic.contains(r#"name="fields[topic]""#));

    let timing = tag(&html, "contact-field-timing");
    assert!(timing.starts_with("<textarea "), "{timing}");
    assert!(timing.contains(r#"name="fields[timing]""#));
    assert!(timing.contains(r#"maxlength="500""#));

    for (key, label) in [
        ("organisation", "Organisation"),
        ("topic", "Topic"),
        ("timing", "Timing"),
    ] {
        assert!(
            html.contains(&format!(
                r#"<label for="contact-field-{key}" class="">{label}</label>"#
            )),
            "the {key} label: {html}"
        );
        assert_eq!(
            count(&html, &format!(r#"id="contact-field-{key}""#)),
            1,
            "{key}"
        );
    }
}

/// D5: a `Choice` starts with an empty option shown as "—", then its choices
/// in the order they were defined, each with its key as the value and its
/// label as the text.
#[test]
fn a_choice_starts_with_the_empty_option_then_its_choices_in_order() {
    let html = form(definition());
    let select = select(&html, "contact-field-topic");

    let options: Vec<&str> = select
        .split("<option ")
        .skip(1)
        .map(|o| o.split("</option>").next().unwrap())
        .collect();
    assert_eq!(
        options,
        [
            r#"value="">—"#,
            r#"value="sales">Sales"#,
            r#"value="support">Support"#
        ]
    );
}

/// D5, FR-A11Y: `required` and `aria-required` on a required field, and
/// neither on an optional one, for every kind of control.
#[test]
fn required_fields_carry_required_and_aria_required() {
    let html = form(definition());

    for id in ["contact-field-organisation", "contact-field-topic"] {
        let tag = tag(&html, id);
        assert!(tag.contains(" required"), "{id}: {tag}");
        assert!(tag.contains(r#"aria-required="true""#), "{id}: {tag}");
    }
    let timing = tag(&html, "contact-field-timing");
    assert!(!timing.contains("required"), "{timing}");
}

/// D5: with no error, no field is marked invalid and none points at an error
/// paragraph.
#[test]
fn a_field_without_an_error_is_not_marked_invalid() {
    let html = form(definition());
    assert_eq!(count(&html, "aria-invalid"), 0);
    assert_eq!(count(&html, "-error"), 0);
}

/// D5: the rows sit after the subject and before the message, in the order
/// they were defined, whatever their keys' alphabetical order.
#[test]
fn the_rows_sit_between_the_subject_and_the_message_in_definition_order() {
    let html = form(definition());
    let at = |id: &str| {
        html.find(&format!(r#"id="{id}""#))
            .unwrap_or_else(|| panic!("no {id}"))
    };

    let order = [
        "contact-name",
        "contact-email",
        "contact-subject",
        "contact-field-organisation",
        "contact-field-topic",
        "contact-field-timing",
        "contact-message",
    ];
    for pair in order.windows(2) {
        assert!(at(pair[0]) < at(pair[1]), "{} before {}", pair[0], pair[1]);
    }
}

/// D5: with the subject hidden, the rows still come before the message.
#[test]
fn the_rows_are_still_before_the_message_without_a_subject() {
    let options = ContactFormOptions {
        show_subject: false,
        ..Default::default()
    };
    let fields = definition();
    let html =
        render(move || view! { <ContactForm options=options site_fields=fields /> }.into_any());

    assert!(!html.contains("contact-subject"));
    assert!(html.find("contact-field-timing") < html.find(r#"id="contact-message""#));
    assert!(html.find(r#"id="contact-email""#) < html.find("contact-field-organisation"));
}

/// D5: the rows use the form's own classes and nothing new: the field wrapper,
/// the label, the input class for a line and a select, and the textarea class
/// for several lines.
#[test]
fn the_rows_use_the_existing_classes() {
    let classes = ContactFormClasses {
        field: "f-row".into(),
        label: "f-label".into(),
        input: "f-input".into(),
        textarea: "f-area".into(),
        ..Default::default()
    };
    let fields = definition();
    let html =
        render(move || view! { <ContactForm classes=classes site_fields=fields /> }.into_any());

    assert!(
        html.contains(r#"<label for="contact-field-topic" class="f-label">"#),
        "{html}"
    );
    assert!(tag(&html, "contact-field-organisation").contains(r#"class="f-input""#));
    assert!(tag(&html, "contact-field-topic").contains(r#"class="f-input""#));
    assert!(tag(&html, "contact-field-timing").contains(r#"class="f-area""#));
    // Name, email, subject, message and the submit row are five wrappers;
    // the three site rows make eight.
    assert_eq!(count(&html, r#"<div class="f-row">"#), 5 + 3);
}

/// A label and a choice label are text, escaped like any other.
#[test]
fn labels_are_escaped() {
    let fields = SiteFields::new(vec![SiteField {
        key: "topic".into(),
        label: "Topic <b>&</b>".into(),
        kind: SiteFieldKind::Choice(vec![
            SiteFieldChoice {
                key: "a".into(),
                label: "A <i>".into(),
            },
            SiteFieldChoice {
                key: "b".into(),
                label: "B".into(),
            },
        ]),
        required: false,
        max_len: 0,
    }])
    .unwrap();
    let html = form(fields);

    assert!(html.contains("Topic &lt;b&gt;&amp;&lt;/b&gt;"), "{html}");
    assert!(html.contains("A &lt;i&gt;"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
    assert!(!html.contains("<i>"), "{html}");
}
