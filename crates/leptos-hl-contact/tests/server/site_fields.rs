//! What refusing a site-defined field logs (RFC 015 A3, A5): a **count**, and
//! never a key or a value from the request.  Keys are attacker text and values
//! are personal data.

use std::collections::BTreeMap;

use leptos_hl_contact::{
    SiteField, SiteFieldKind, SiteFields,
    model::{SiteFieldsOutcome, validate_site_fields},
};

use crate::support::capture_logs;

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
