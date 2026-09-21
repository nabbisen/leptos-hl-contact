//! Site-defined fields through the documented router, in both request forms
//! (RFC 015 D3, A3, A4, A5): what is accepted, what is refused, the errors
//! and where they land, and what is logged: a **count**, and never a key or
//! a value from the request.  Keys are attacker text and values are personal
//! data.

use std::collections::BTreeMap;

use leptos_hl_contact::{
    ContactFieldErrors, ContactServerPolicy, FieldError, FieldErrorCode, SiteField,
    SiteFieldChoice, SiteFieldKind, SiteFieldValue, SiteFields,
    model::{SiteFieldsOutcome, validate_site_fields},
};

use crate::support::{Fields, Harness, Reply, Setup, capture_logs};

/// A distinctive fake key and value, so their absence from the captured
/// output cannot be a coincidence.
const PROBE_KEY: &str = "zz_probe_key";
const PROBE_VALUE: &str = "zz_probe_value";

fn definition() -> SiteFields {
    SiteFields::new(vec![SiteField {
        key: "topic".into(),
        label: "Topic".into(),
        kind: SiteFieldKind::Line,
        required: false,
        max_len: 10,
    }])
    .expect("a valid definition")
}

fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// FR-OBS-02, RFC 015 A3.1, A5: more keys than the bound is refused, and the
/// log says how many, not which.
#[test]
fn too_many_keys_log_the_count_and_never_a_key() {
    let (logs, _guard) = capture_logs();
    let five = map(&[
        (PROBE_KEY, PROBE_VALUE),
        ("k1", "v"),
        ("k2", "v"),
        ("k3", "v"),
        ("k4", "v"),
    ]);

    assert_eq!(
        validate_site_fields(&definition(), Some(&five)),
        SiteFieldsOutcome::Refused
    );

    assert!(
        logs.any_contains("site fields refused: too many keys"),
        "the refusal is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
    assert!(logs.any_contains("count=5"), "{:?}", logs.lines());
    for line in logs.lines() {
        assert!(
            !line.contains(PROBE_KEY),
            "a request key was logged: {line}"
        );
        assert!(!line.contains(PROBE_VALUE), "a request value was logged");
    }
}

/// FR-OBS-02, RFC 015 A3.2, A5: an unknown key is refused, and the log
/// carries the count of unknown keys, not the key.
#[test]
fn an_unknown_key_logs_its_count_and_never_the_key() {
    let (logs, _guard) = capture_logs();
    let stray = map(&[(PROBE_KEY, PROBE_VALUE), ("topic", "sales")]);

    assert_eq!(
        validate_site_fields(&definition(), Some(&stray)),
        SiteFieldsOutcome::Refused
    );

    assert!(
        logs.any_contains("site fields refused: unknown keys"),
        "the refusal is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
    assert!(logs.any_contains("unknown_keys=1"), "{:?}", logs.lines());
    for line in logs.lines() {
        assert!(
            !line.contains(PROBE_KEY),
            "a request key was logged: {line}"
        );
        assert!(!line.contains(PROBE_VALUE), "a request value was logged");
    }
}

/// Two unknown keys are counted as two.
#[test]
fn several_unknown_keys_are_counted() {
    let (logs, _guard) = capture_logs();
    let stray = map(&[(PROBE_KEY, PROBE_VALUE), ("zz_probe_other", PROBE_VALUE)]);

    assert_eq!(
        validate_site_fields(&definition(), Some(&stray)),
        SiteFieldsOutcome::Refused
    );

    assert!(logs.any_contains("unknown_keys=2"), "{:?}", logs.lines());
    assert!(!logs.any_contains("zz_probe"), "{:?}", logs.lines());
}

// ---------------------------------------------------------------------------
// Through the router
// ---------------------------------------------------------------------------

/// The three-field example, defined in an order (`topic`, `organisation`,
/// `timing`) that is not alphabetical.  Values are obviously fake.
fn example() -> SiteFields {
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
            max_len: 20,
        },
        SiteField {
            key: "timing".into(),
            label: "Timing".into(),
            kind: SiteFieldKind::Text,
            required: false,
            max_len: 100,
        },
    ])
    .expect("a valid definition")
}

fn setup() -> Setup {
    Setup {
        policy: Some(ContactServerPolicy {
            site_fields: example(),
            ..Default::default()
        }),
        ..Setup::default()
    }
}

fn harness() -> Harness {
    Harness::new(setup())
}

/// A valid submission, with all three site fields answered.
fn valid(h: &Harness) -> Fields {
    h.fields()
        .set("fields[topic]", "sales")
        .set("fields[organisation]", "Example Co")
        .set("fields[timing]", "Next month")
}

const REJECTED_BANNER: &str = "Your message could not be accepted.";
const GENERIC_BANNER: &str = "Failed to send message. Please try again later.";

fn code(code: FieldErrorCode) -> FieldError {
    FieldError::Code(code)
}

fn errors_of(reply: &Reply, context: &str) -> ContactFieldErrors {
    reply
        .field_errors()
        .unwrap_or_else(|| panic!("{context}: no field errors in {}", reply.body))
}

/// FR-FIELD-06, FR-SUB-01: a valid submission is delivered in both request
/// forms, with the values in **definition order**, their labels from the
/// server's definition, and a choice carrying its key and its label.
#[tokio::test]
async fn a_valid_submission_is_delivered_with_the_values_in_definition_order() {
    let h = harness();
    let fields = valid(&h);

    let fetch = h.submit_fetch(&fields).await;
    assert!(fetch.status.is_success(), "{}", fetch.body);
    let delivered = h.delivery.last().expect("delivered");
    let nojs = h.submit_nojs(&fields).await;
    assert!(nojs.is_nojs_success(), "{:?}", nojs.location());

    let value = |key: &str, label: &str, value: &str, value_label: Option<&str>| SiteFieldValue {
        key: key.into(),
        label: label.into(),
        value: value.into(),
        value_label: value_label.map(Into::into),
    };
    let expected = vec![
        value("topic", "Topic", "sales", Some("Sales enquiry")),
        value("organisation", "Organisation", "Example Co", None),
        value("timing", "Timing", "Next month", None),
    ];
    assert_eq!(delivered.site_fields, expected, "fetch");
    assert_eq!(
        h.delivery.last().expect("delivered").site_fields,
        expected,
        "no-JS"
    );
    assert_eq!(h.deliveries(), 2);
}

/// FR-FIELD-06: only answered fields are delivered, and a blank optional one
/// is left out.
#[tokio::test]
async fn a_blank_optional_field_is_left_out_of_delivery() {
    let h = harness();

    let reply = h
        .submit_fetch(&valid(&h).set("fields[timing]", "   "))
        .await;
    assert!(reply.status.is_success(), "{}", reply.body);
    let keys: Vec<String> = h
        .delivery
        .last()
        .expect("delivered")
        .site_fields
        .into_iter()
        .map(|v| v.key)
        .collect();
    assert_eq!(keys, ["topic", "organisation"]);

    let reply = h.submit_fetch(&valid(&h).without("fields[timing]")).await;
    assert!(reply.status.is_success(), "{}", reply.body);
    assert_eq!(h.delivery.last().unwrap().site_fields.len(), 2);
}

/// FR-FIELD-03, RFC 015 A3: too many keys, a key the definition lacks, and a
/// key with a control character each refuse the **whole submission** as
/// `rejected`, in both request forms, and deliver nothing.  The refusal comes
/// before any field error, so a submission that is also invalid learns
/// nothing about the definition.
#[tokio::test]
async fn an_unknown_key_or_too_many_keys_reject_the_whole_submission() {
    let h = harness();
    let cases = [
        (
            "five keys",
            valid(&h).set("fields[k1]", "v").set("fields[k2]", "v"),
        ),
        ("an unknown key", valid(&h).set("fields[stray]", "v")),
        (
            "an unknown key beside a missing required one",
            h.fields().set("fields[stray]", "v"),
        ),
        (
            "a key with a line feed",
            valid(&h).set("fields[to\npic]", "v"),
        ),
        ("a key with a NUL", valid(&h).set("fields[topic\0]", "v")),
        (
            "an unknown key on an otherwise invalid submission",
            valid(&h).set("fields[stray]", "v").set("name", ""),
        ),
    ];

    for (row, fields) in cases {
        let fetch = h.submit_fetch(&fields).await;
        assert_eq!(
            fetch.contact_error().as_deref(),
            Some("rejected"),
            "{row}, fetch"
        );
        assert!(
            fetch.field_errors().is_none(),
            "{row}: no field error, fetch"
        );

        let nojs = h.submit_nojs(&fields).await;
        assert!(nojs.is_nojs_error(), "{row}, no-JS: {:?}", nojs.location());
        let page = h.follow(&nojs).await;
        assert_eq!(
            page.banner().as_deref(),
            Some(REJECTED_BANNER),
            "{row}, no-JS"
        );
        assert_eq!(page.aria_invalid_count(), 0, "{row}, no-JS");
    }
    assert_eq!(h.deliveries(), 0);
}

/// FR-FIELD-03: a site that defines no fields refuses any `fields[…]` key,
/// with or without a policy in context.
#[tokio::test]
async fn a_site_without_site_fields_refuses_any_field_key() {
    for h in [
        Harness::new(Setup::default()),
        Harness::new(Setup {
            policy: Some(ContactServerPolicy::default()),
            ..Setup::default()
        }),
    ] {
        let fields = h.fields().set("fields[topic]", "sales");
        let fetch = h.submit_fetch(&fields).await;
        assert_eq!(fetch.contact_error().as_deref(), Some("rejected"));
        assert!(h.submit_nojs(&fields).await.is_nojs_error());
        assert_eq!(h.deliveries(), 0);
    }
}

/// Today's behaviour, with no site fields: a submission with no `fields[…]`
/// at all is delivered, and carries no site values.  (The rest of the suite,
/// which defines none, is the wider evidence; this names the row.)
#[tokio::test]
async fn no_fields_at_all_with_an_empty_definition_is_todays_behaviour() {
    let h = Harness::new(Setup::default());
    let fields = h.fields();

    assert!(h.submit_fetch(&fields).await.status.is_success());
    assert!(h.submit_nojs(&fields).await.is_nojs_success());
    assert_eq!(h.deliveries(), 2);
    assert!(h.delivery.last().unwrap().site_fields.is_empty());
}

/// FR-FIELD-05, D2: each rule reports its code under the definition's key, in
/// both request forms.  Over the wire the fetch response carries it in the
/// field errors; without JavaScript the page it lands on marks exactly that
/// field invalid, points it at its error paragraph, and raises no banner.
#[tokio::test]
async fn each_rule_reports_its_code_under_the_key() {
    let h = harness();
    let cases = [
        (
            "a blank required choice",
            valid(&h).set("fields[topic]", ""),
            "topic",
            code(FieldErrorCode::Required),
        ),
        (
            "a required field absent from the request",
            valid(&h).without("fields[organisation]"),
            "organisation",
            code(FieldErrorCode::Required),
        ),
        (
            "an over-length line",
            valid(&h).set("fields[organisation]", &"o".repeat(21)),
            "organisation",
            code(FieldErrorCode::Length { min: 1, max: 20 }),
        ),
        (
            "an over-length text",
            valid(&h).set("fields[timing]", &"t".repeat(101)),
            "timing",
            code(FieldErrorCode::Length { min: 0, max: 100 }),
        ),
        (
            "a line feed in a line",
            valid(&h).set("fields[organisation]", "Example\nCo"),
            "organisation",
            code(FieldErrorCode::LineBreaks),
        ),
        (
            "a carriage return in a line",
            valid(&h).set("fields[organisation]", "Example\rCo"),
            "organisation",
            code(FieldErrorCode::LineBreaks),
        ),
        (
            "an unlisted choice",
            valid(&h).set("fields[topic]", "billing"),
            "topic",
            code(FieldErrorCode::Format),
        ),
    ];

    for (row, fields, key, expected) in cases {
        let fetch = h.submit_fetch(&fields).await;
        let errors = errors_of(&fetch, row);
        assert_eq!(errors.site_fields.get(key), Some(&expected), "{row}, fetch");
        assert_eq!(errors.site_fields.len(), 1, "{row}: only that field, fetch");
        assert!(errors.name.is_none() && errors.message.is_none(), "{row}");

        let nojs = h.submit_nojs(&fields).await;
        assert!(nojs.is_nojs_error(), "{row}, no-JS: {:?}", nojs.location());
        let page = h.follow(&nojs).await;
        assert_eq!(page.aria_invalid_count(), 1, "{row}, no-JS");
        assert_eq!(page.banner(), None, "{row}, no-JS");
        assert!(
            page.html
                .contains(&format!(r#"aria-describedby="contact-field-{key}-error""#)),
            "{row}: the field points at its error"
        );
        assert!(
            page.html
                .contains(&format!(r#"id="contact-field-{key}-error""#)),
            "{row}: the error paragraph"
        );
    }
    assert_eq!(h.deliveries(), 0);
}

/// FR-FIELD-05: the required label reads as it does for a built-in field, and
/// a length error names the limit.
#[tokio::test]
async fn the_error_text_is_the_existing_label_text() {
    let h = harness();

    let required = h.submit_nojs(&valid(&h).set("fields[topic]", "")).await;
    assert!(
        h.follow(&required)
            .await
            .html
            .contains("This field is required.")
    );

    let long = valid(&h).set("fields[organisation]", &"o".repeat(21));
    let page = h.follow(&h.submit_nojs(&long).await).await;
    assert!(
        page.html.contains("and 20 characters."),
        "the length label names the limit"
    );
}

/// D2: site-field errors and the built-in fields' errors arrive in one
/// response, so the visitor fixes everything at once.
#[tokio::test]
async fn site_field_errors_and_built_in_errors_arrive_together() {
    let h = harness();
    let fields = valid(&h)
        .set("name", "")
        .set("fields[topic]", "billing")
        .set("fields[organisation]", "");

    let fetch = h.submit_fetch(&fields).await;
    let errors = errors_of(&fetch, "several");
    assert_eq!(errors.name, Some(code(FieldErrorCode::Required)));
    assert_eq!(
        errors.site_fields.get("topic"),
        Some(&code(FieldErrorCode::Format))
    );
    assert_eq!(
        errors.site_fields.get("organisation"),
        Some(&code(FieldErrorCode::Required))
    );

    let nojs = h.submit_nojs(&fields).await;
    assert_eq!(h.follow(&nojs).await.aria_invalid_count(), 3);
    assert_eq!(h.deliveries(), 0);
}

/// FR-ABUSE-09, RFC 015 D3: a honeypot hit still ends as a silent success
/// before any site-field check, so a bot learns nothing from what it sends —
/// not from invalid values, and not from keys the definition lacks.  Its
/// response is the delivered one's, in both request forms, and nothing is
/// delivered.
#[tokio::test]
async fn a_honeypot_hit_is_silent_whatever_the_site_fields_hold() {
    let delivered = harness();
    let honeypot = harness();
    let genuine = valid(&delivered);
    let bot = valid(&honeypot)
        .set("website", "http://bot.example")
        .set("fields[topic]", "billing")
        .set("fields[organisation]", &"o".repeat(50))
        .set("fields[stray]", "v");

    let (a, b) = (
        delivered.submit_fetch(&genuine).await,
        honeypot.submit_fetch(&bot).await,
    );
    assert_eq!(a.fingerprint(), b.fingerprint(), "fetch");
    let (a, b) = (
        delivered.submit_nojs(&genuine).await,
        honeypot.submit_nojs(&bot).await,
    );
    assert_eq!(a.fingerprint(), b.fingerprint(), "no-JS");

    assert_eq!([delivered.deliveries(), honeypot.deliveries()], [2, 0]);
}

/// RFC 015 A4: a malformed map fails while the request is decoded, before
/// `submit_contact` runs.  The response is neither a `contact_error:` nor a
/// `field_errors:` payload, so the form shows its generic message, and
/// nothing is delivered.  A repeated key, a nested one, a sequence and a bare
/// `fields=` are the four shapes.
#[tokio::test]
async fn a_malformed_map_shows_the_generic_message() {
    let h = harness();
    let cases = [
        ("a repeated key", valid(&h).push("fields[topic]", "support")),
        ("a nested key", valid(&h).push("fields[topic][x]", "v")),
        ("a sequence", valid(&h).push("fields[]", "v")),
        ("a bare fields=", h.fields().set("fields", "x")),
    ];

    for (row, fields) in cases {
        let fetch = h.submit_fetch(&fields).await;
        assert!(!fetch.status.is_success(), "{row}, fetch: {}", fetch.body);
        assert_eq!(fetch.contact_error(), None, "{row}, fetch: {}", fetch.body);
        assert!(
            fetch.field_errors().is_none(),
            "{row}, fetch: {}",
            fetch.body
        );

        let nojs = h.submit_nojs(&fields).await;
        assert!(nojs.is_nojs_error(), "{row}, no-JS: {:?}", nojs.location());
        let page = h.follow(&nojs).await;
        assert_eq!(
            page.banner().as_deref(),
            Some(GENERIC_BANNER),
            "{row}, no-JS"
        );
        assert_eq!(page.aria_invalid_count(), 0, "{row}, no-JS");
    }
    assert_eq!(h.deliveries(), 0);
}

/// RFC 015 D1, D3: a form and a server that disagree **fail closed**, and
/// loudly.  The server requires `topic`; the page did not render it.  The
/// server reports `required` under `topic`, and the page, which has no such
/// row, shows the generic message instead of dropping the error.
#[tokio::test]
async fn a_server_field_the_form_did_not_render_shows_the_generic_message() {
    let h = Harness::new(Setup {
        form_site_fields: Some(SiteFields::empty()),
        ..setup()
    });
    let fields = h.fields();

    let fetch = h.submit_fetch(&fields).await;
    let errors = errors_of(&fetch, "mismatch");
    assert_eq!(
        errors.site_fields.get("topic"),
        Some(&code(FieldErrorCode::Required))
    );

    let nojs = h.submit_nojs(&fields).await;
    assert!(nojs.is_nojs_error());
    let page = h.follow(&nojs).await;
    assert!(
        !page.html.contains("contact-field-topic"),
        "the page has no such row"
    );
    assert_eq!(page.banner().as_deref(), Some(GENERIC_BANNER));
    assert_eq!(page.aria_invalid_count(), 0);
    assert_eq!(h.deliveries(), 0);
}

/// FR-FIELD-02: the page a visitor gets renders the server's own definition,
/// when the site builds one and passes it to both.
#[tokio::test]
async fn the_page_renders_the_shared_definition() {
    let h = harness();
    let page = h.render("/contact", None).await;

    assert_eq!(page.status.as_u16(), 200);
    for name in ["fields[topic]", "fields[organisation]", "fields[timing]"] {
        assert!(page.html.contains(&format!(r#"name="{name}""#)), "{name}");
    }
}

/// FR-FIELD-07, FR-OBS-02: through the router, a value of a site field, and a
/// key the request invented, appear in no log event, whether the submission
/// is delivered, invalid, refused or a honeypot hit.  The capture is checked
/// first: the refusal's own event is present.
#[tokio::test]
async fn site_field_values_and_request_keys_are_never_logged() {
    let (logs, _guard) = capture_logs();
    let h = harness();
    const VALUE: &str = "ORGANISATION-VALUE-MARKER-4f8";
    const OTHER: &str = "TIMING-VALUE-MARKER-2b6";

    let delivered = valid(&h)
        .set("fields[organisation]", VALUE)
        .set("fields[timing]", OTHER);
    let invalid = delivered.clone().set("fields[topic]", "billing");
    let too_long = delivered
        .clone()
        .set("fields[organisation]", &VALUE.repeat(3));
    let refused = delivered.clone().set("fields[zz_probe_key]", OTHER);
    let honeypot = delivered.clone().set("website", "http://bot.example");
    for fields in [&delivered, &invalid, &too_long, &refused, &honeypot] {
        h.submit_fetch(fields).await;
        h.submit_nojs(fields).await;
    }

    assert!(
        logs.any_contains("site fields refused: unknown keys"),
        "the refusal is logged; the capture saw:\n{}",
        logs.lines().join("\n")
    );
    for marker in [VALUE, OTHER, "zz_probe_key", "billing"] {
        assert!(
            !logs.any_contains(marker),
            "a request value or key was logged: {marker}"
        );
    }
}
