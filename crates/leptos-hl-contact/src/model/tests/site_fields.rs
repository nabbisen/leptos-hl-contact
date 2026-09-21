//! Validating the site-defined fields of a submission (RFC 015 D2, A2, A3).
//!
//! Values in these tests are obviously fake: a real field value is personal
//! data and never belongs in a test's output.

use std::collections::BTreeMap;

use crate::{
    config::{SiteField, SiteFieldChoice, SiteFieldKind, SiteFields},
    error::{FieldError, FieldErrorCode},
    model::{SiteFieldValue, SiteFieldsOutcome, validate_site_fields},
};

/// Three fields whose definition order (`topic`, `organisation`, `timing`)
/// differs from their alphabetical order, which is the order a `BTreeMap`
/// yields.  `topic` and `organisation` are required.
fn definition() -> SiteFields {
    SiteFields::new(vec![
        SiteField {
            key: "topic".into(),
            label: "Topic".into(),
            kind: SiteFieldKind::Choice(vec![
                SiteFieldChoice {
                    key: "sales".into(),
                    label: "Sales enquiry".into(),
                },
                SiteFieldChoice {
                    key: "support".into(),
                    label: "Support request".into(),
                },
            ]),
            required: true,
            max_len: 0,
        },
        SiteField {
            key: "organisation".into(),
            label: "Organisation".into(),
            kind: SiteFieldKind::Line,
            required: true,
            max_len: 10,
        },
        SiteField {
            key: "timing".into(),
            label: "When".into(),
            kind: SiteFieldKind::Text,
            required: false,
            max_len: 20,
        },
    ])
    .expect("a valid definition")
}

fn raw(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn code(code: FieldErrorCode) -> FieldError {
    FieldError::Code(code)
}

fn invalid(outcome: SiteFieldsOutcome) -> BTreeMap<String, FieldError> {
    match outcome {
        SiteFieldsOutcome::Invalid(errors) => errors,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

fn valid(outcome: SiteFieldsOutcome) -> Vec<SiteFieldValue> {
    match outcome {
        SiteFieldsOutcome::Valid(values) => values,
        other => panic!("expected Valid, got {other:?}"),
    }
}

/// Both required fields answered, in the most ordinary way.
const ANSWERED: [(&str, &str); 2] = [("topic", "sales"), ("organisation", "Example Co")];

// ---- absent, and the outcomes that are not per-field ----------------------

/// `None` and an empty map both mean every field is absent.
#[test]
fn none_and_an_empty_map_are_every_field_absent() {
    for input in [None, Some(&raw(&[]))] {
        let errors = invalid(validate_site_fields(&definition(), input));
        assert_eq!(
            errors.keys().map(String::as_str).collect::<Vec<_>>(),
            ["organisation", "topic"],
            "both required fields are Required"
        );
        assert!(
            errors
                .values()
                .all(|e| *e == code(FieldErrorCode::Required))
        );
    }
}

#[test]
fn a_definition_with_no_fields_accepts_no_map_and_refuses_any_key() {
    let none = SiteFields::empty();
    assert_eq!(
        validate_site_fields(&none, None),
        SiteFieldsOutcome::Valid(Vec::new())
    );
    assert_eq!(
        validate_site_fields(&none, Some(&raw(&[]))),
        SiteFieldsOutcome::Valid(Vec::new())
    );
    assert_eq!(
        validate_site_fields(&none, Some(&raw(&[("topic", "sales")]))),
        SiteFieldsOutcome::Refused,
        "a page that defines no fields never sends one"
    );
}

/// A3.1: more keys than `SiteFields::MAX` is refused, whatever they are.
#[test]
fn five_keys_are_refused() {
    let five = raw(&[
        ("topic", "sales"),
        ("organisation", "Example Co"),
        ("timing", "soon"),
        ("extra_one", "x"),
        ("extra_two", "x"),
    ]);
    assert_eq!(five.len(), SiteFields::MAX + 1);
    assert_eq!(
        validate_site_fields(&definition(), Some(&five)),
        SiteFieldsOutcome::Refused
    );

    let many: BTreeMap<String, String> = (0..1000).map(|n| (format!("k{n}"), "x".into())).collect();
    assert_eq!(
        validate_site_fields(&definition(), Some(&many)),
        SiteFieldsOutcome::Refused,
        "spike case (i): 1,000 keys parse, so only this bound stops them"
    );
}

/// A3.2: a key the definition does not have, on its own or beside valid ones.
#[test]
fn an_unknown_key_is_refused() {
    let alone = raw(&[("nothing", "x")]);
    assert_eq!(
        validate_site_fields(&definition(), Some(&alone)),
        SiteFieldsOutcome::Refused
    );

    let beside_valid = raw(&[
        ("topic", "sales"),
        ("organisation", "Example Co"),
        ("nothing", "x"),
    ]);
    assert_eq!(
        validate_site_fields(&definition(), Some(&beside_valid)),
        SiteFieldsOutcome::Refused,
        "a valid submission plus one stray key is still refused"
    );
}

/// Spike case (h): a control character in a key.  The allow-list refuses it,
/// with no crash and no separate charset check.
#[test]
fn a_key_with_a_newline_is_refused_not_a_crash() {
    for key in ["ta\npic", "topic\n", "\ntopic", "to\rpic", "topic\0"] {
        let bad = raw(&[(key, "sales")]);
        assert_eq!(
            validate_site_fields(&definition(), Some(&bad)),
            SiteFieldsOutcome::Refused,
            "{key:?}"
        );
    }
}

// ---- per field ------------------------------------------------------------

#[test]
fn a_required_field_that_is_blank_or_missing_is_required() {
    for blank in ["", " ", "\t\r\n  "] {
        let errors = invalid(validate_site_fields(
            &definition(),
            Some(&raw(&[("topic", "sales"), ("organisation", blank)])),
        ));
        assert_eq!(
            errors.get("organisation"),
            Some(&code(FieldErrorCode::Required)),
            "{blank:?}"
        );
        assert!(!errors.contains_key("topic"));
    }
    let missing = invalid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales")])),
    ));
    assert_eq!(
        missing.get("organisation"),
        Some(&code(FieldErrorCode::Required))
    );
}

#[test]
fn a_value_is_trimmed() {
    let values = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", " sales "),
            ("organisation", "  Example Co \n"),
        ])),
    ));
    let organisation = values.iter().find(|v| v.key == "organisation").unwrap();
    assert_eq!(organisation.value, "Example Co");
}

/// A `Line` at `max_len` characters passes; one more is `Length`, whose `min`
/// is 1 when the field is required and 0 when it is not.
#[test]
fn a_line_at_its_limit_passes_and_one_over_is_length() {
    let at_limit = "x".repeat(10);
    valid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales"), ("organisation", &at_limit)])),
    ));

    let over = "x".repeat(11);
    let errors = invalid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales"), ("organisation", &over)])),
    ));
    assert_eq!(
        errors.get("organisation"),
        Some(&code(FieldErrorCode::Length { min: 1, max: 10 })),
        "required: min 1"
    );
}

/// The count is in characters, as for the built-in fields, so a multibyte
/// value at the limit passes.
#[test]
fn length_counts_characters_not_bytes() {
    let multibyte = "あ".repeat(10);
    assert!(
        multibyte.len() > 10,
        "precondition: more bytes than characters"
    );
    valid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales"), ("organisation", &multibyte)])),
    ));
}

#[test]
fn a_line_with_a_line_break_is_line_breaks() {
    for value in ["two\nlines", "two\rlines", "two\r\nlines"] {
        let errors = invalid(validate_site_fields(
            &definition(),
            Some(&raw(&[("topic", "sales"), ("organisation", value)])),
        ));
        assert_eq!(
            errors.get("organisation"),
            Some(&code(FieldErrorCode::LineBreaks)),
            "{value:?}"
        );
    }
    // A break at either end is trimmed away first, as for the built-in fields.
    valid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales"), ("organisation", "one line\n")])),
    ));
}

/// RFC 010 D2, as `validate_fields` does it: when a value is both too long
/// and has a line break, the visitor is told it is too long.
#[test]
fn length_is_reported_before_line_breaks() {
    let both = format!("{}\n{}", "x".repeat(8), "y".repeat(8));
    let errors = invalid(validate_site_fields(
        &definition(),
        Some(&raw(&[("topic", "sales"), ("organisation", &both)])),
    ));
    assert_eq!(
        errors.get("organisation"),
        Some(&code(FieldErrorCode::Length { min: 1, max: 10 }))
    );
}

/// A `Text` field takes line breaks, passes at exactly `max_len` characters,
/// and is `Length` over `max_len` with `min` 0, since it is optional here.
#[test]
fn a_text_field_takes_line_breaks_and_has_a_length() {
    let values = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "sales"),
            ("organisation", "Example Co"),
            ("timing", "line one\nline two"),
        ])),
    ));
    assert_eq!(values[2].value, "line one\nline two");

    // Exactly `max_len` (20) characters is accepted: "at most", not "under".
    let at_limit = "x".repeat(20);
    let values = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "sales"),
            ("organisation", "Example Co"),
            ("timing", &at_limit),
        ])),
    ));
    assert_eq!(values[2].value, at_limit);

    let over = "x".repeat(21);
    let errors = invalid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "sales"),
            ("organisation", "Example Co"),
            ("timing", &over),
        ])),
    ));
    assert_eq!(
        errors.get("timing"),
        Some(&code(FieldErrorCode::Length { min: 0, max: 20 }))
    );
}

/// A `Choice` value must be one of the listed keys, exactly: not the label,
/// not another case, not something else.
#[test]
fn an_unlisted_choice_is_format() {
    for value in [
        "billing",
        "SALES",
        "Sales",
        "Sales enquiry",
        "sales,support",
        "sal es",
    ] {
        let errors = invalid(validate_site_fields(
            &definition(),
            Some(&raw(&[("topic", value), ("organisation", "Example Co")])),
        ));
        assert_eq!(
            errors.get("topic"),
            Some(&code(FieldErrorCode::Format)),
            "{value:?}"
        );
    }
}

#[test]
fn a_listed_choice_carries_its_key_and_its_label() {
    let values = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "support"),
            ("organisation", "Example Co"),
        ])),
    ));
    let topic = values.iter().find(|v| v.key == "topic").unwrap();
    assert_eq!(topic.value, "support");
    assert_eq!(topic.value_label.as_deref(), Some("Support request"));
    let organisation = values.iter().find(|v| v.key == "organisation").unwrap();
    assert_eq!(organisation.value_label, None, "only a Choice has one");
}

/// An optional field left blank is not in the answered list.
#[test]
fn an_absent_optional_field_is_omitted() {
    let values = valid(validate_site_fields(&definition(), Some(&raw(&ANSWERED))));
    assert_eq!(
        values.iter().map(|v| v.key.as_str()).collect::<Vec<_>>(),
        ["topic", "organisation"],
        "`timing` is optional and was not answered"
    );

    let blank = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "sales"),
            ("organisation", "Example Co"),
            ("timing", "   "),
        ])),
    ));
    assert_eq!(blank.len(), 2, "a blank optional value is absent too");
}

// ---- shape of the result --------------------------------------------------

/// `Valid` follows the definition's order (`topic`, `organisation`,
/// `timing`), not the `BTreeMap`'s alphabetical one (`organisation`,
/// `timing`, `topic`).
#[test]
fn valid_values_follow_definition_order() {
    let all = raw(&[
        ("timing", "soon"),
        ("organisation", "Example Co"),
        ("topic", "sales"),
    ]);
    assert_eq!(
        all.keys().map(String::as_str).collect::<Vec<_>>(),
        ["organisation", "timing", "topic"],
        "precondition: the map iterates alphabetically"
    );
    let values = valid(validate_site_fields(&definition(), Some(&all)));
    assert_eq!(
        values.iter().map(|v| v.key.as_str()).collect::<Vec<_>>(),
        ["topic", "organisation", "timing"]
    );
}

/// The label in the result is the definition's, whatever the request says.
#[test]
fn labels_come_from_the_definition() {
    let values = valid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "sales"),
            ("organisation", "Fake Label"),
            ("timing", "Organisation"),
        ])),
    ));
    let labels: Vec<_> = values.iter().map(|v| v.label.as_str()).collect();
    assert_eq!(labels, ["Topic", "Organisation", "When"]);
}

/// Every failing field is reported, not the first; a valid field is not.
#[test]
fn several_errors_are_reported_together() {
    let errors = invalid(validate_site_fields(
        &definition(),
        Some(&raw(&[
            ("topic", "billing"),
            ("organisation", &"x".repeat(11)),
            ("timing", "fine"),
        ])),
    ));
    assert_eq!(errors.len(), 2);
    assert!(errors.contains_key("topic") && errors.contains_key("organisation"));
    assert!(!errors.contains_key("timing"));
}
