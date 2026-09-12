// tests.rs — unit tests for the parent module.

use super::*;
#[test]
fn classes_default_is_all_empty() {
    let c = ContactFormClasses::default();
    assert!(c.root.is_empty());
    assert!(c.button.is_empty());
}

#[test]
fn labels_default_has_english_text() {
    let l = ContactFormLabels::default();
    assert!(!l.submit.is_empty());
    assert!(!l.success.is_empty());
}

#[test]
fn options_default_shows_subject() {
    let o = ContactFormOptions::default();
    assert!(o.show_subject);
    assert_eq!(o.max_message_len, MESSAGE_MAX_LEN);
}

#[test]
fn options_default_focuses_first_error() {
    assert!(ContactFormOptions::default().focus_first_error);
}

#[test]
fn options_effective_len_is_clamped() {
    let over = ContactFormOptions {
        max_message_len: 9_999,
        ..Default::default()
    };
    assert_eq!(over.effective_max_message_len(), MESSAGE_MAX_LEN);

    let under = ContactFormOptions {
        max_message_len: 1_000,
        ..Default::default()
    };
    assert_eq!(under.effective_max_message_len(), 1_000);
}

// ---------------------------------------------------------------------------
// ContactServerPolicy
// ---------------------------------------------------------------------------

fn input_with_message(message: &str) -> ContactInput {
    ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        Some("Hello".into()),
        message.into(),
        String::new(),
    )
}

#[test]
fn policy_default_matches_ceiling() {
    let p = ContactServerPolicy::default();
    assert!(!p.require_subject);
    assert_eq!(p.max_message_len, MESSAGE_MAX_LEN);
    assert_eq!(p.effective_max_message_len(), MESSAGE_MAX_LEN);
}

#[test]
fn policy_check_passes_valid_input() {
    let p = ContactServerPolicy::default();
    assert!(p.check(&input_with_message("Hello there")).is_empty());
}

#[test]
fn policy_check_requires_subject_when_set() {
    let p = ContactServerPolicy {
        require_subject: true,
        ..Default::default()
    };
    let input = ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        None,
        "Hello there".into(),
        String::new(),
    );
    let errs = p.check(&input);
    assert_eq!(errs.subject.as_deref(), Some("Subject is required."));
    assert!(errs.message.is_none());
}

/// A policy limit of 100 must accept 100 multibyte characters.  Counting
/// bytes would reject them at about 34.
#[test]
fn policy_check_counts_characters_not_bytes() {
    let p = ContactServerPolicy {
        max_message_len: 100,
        ..Default::default()
    };

    let exactly = "あ".repeat(100);
    assert!(
        exactly.len() > 100,
        "precondition: multibyte, {} bytes",
        exactly.len()
    );
    assert!(p.check(&input_with_message(&exactly)).is_empty());

    let one_over = "あ".repeat(101);
    assert!(p.check(&input_with_message(&one_over)).message.is_some());
}

#[test]
fn policy_check_clamps_to_ceiling() {
    let p = ContactServerPolicy {
        max_message_len: 10_000,
        ..Default::default()
    };
    assert_eq!(p.effective_max_message_len(), MESSAGE_MAX_LEN);

    let too_long = "x".repeat(MESSAGE_MAX_LEN + 1);
    let msg = p.check(&input_with_message(&too_long)).message.unwrap();
    assert!(
        msg.contains("4000"),
        "message must name the clamped ceiling, got {msg:?}"
    );
}

#[test]
fn policy_check_reports_both_errors_at_once() {
    let p = ContactServerPolicy {
        require_subject: true,
        max_message_len: 10,
    };
    let input = ContactInput::from_raw(
        "Alice".into(),
        "alice@example.com".into(),
        None,
        "far too long to pass".into(),
        String::new(),
    );
    let errs = p.check(&input);
    assert!(errs.subject.is_some());
    assert!(errs.message.is_some());
}

// ---------------------------------------------------------------------------
// ContactSuccessRedirect
// ---------------------------------------------------------------------------

#[test]
fn redirect_accepts_site_relative_paths() {
    for path in ["/thanks", "/a/b?x=1#top", "/", "/a/b/"] {
        assert!(
            ContactSuccessRedirect::new(path, |_| {}).is_ok(),
            "{path:?} should be accepted"
        );
    }
}

/// An open redirect would let an attacker send a visitor off-site through a
/// misconfigured form, so anything not site-relative is refused at startup.
#[test]
fn redirect_rejects_absolute_and_protocol_relative() {
    for path in [
        "https://evil.test",
        "//evil.test",
        "evil.test/x",
        "/a\\b",
        "",
        "/a b",
        "/a\n",
        "/a\tb",
        "javascript:alert(1)",
        "/\u{0000}",
    ] {
        assert!(
            ContactSuccessRedirect::new(path, |_| {}).is_err(),
            "{path:?} should be rejected"
        );
    }
}

#[test]
fn redirect_apply_calls_executor_with_path() {
    use std::sync::{Arc, Mutex};

    let seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&seen);

    let redirect = ContactSuccessRedirect::new("/thanks", move |p| {
        *sink.lock().unwrap() = Some(p.to_owned());
    })
    .expect("site-relative path");

    assert_eq!(redirect.path(), "/thanks");
    assert!(seen.lock().unwrap().is_none(), "not called before apply");

    redirect.apply();
    assert_eq!(seen.lock().unwrap().as_deref(), Some("/thanks"));
}

#[test]
fn redirect_debug_does_not_expose_the_executor() {
    let r = ContactSuccessRedirect::new("/thanks", |_| {}).unwrap();
    let s = format!("{r:?}");
    assert!(s.contains("/thanks"), "path should be visible: {s}");
}
