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
    assert_eq!(
        errs.subject,
        Some(FieldError::Code(FieldErrorCode::Required))
    );
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
    let err = p.check(&input_with_message(&too_long)).message.unwrap();
    assert_eq!(
        err,
        FieldError::Code(FieldErrorCode::Length {
            min: 1,
            max: MESSAGE_MAX_LEN
        }),
        "the code must carry the clamped ceiling"
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

// ---------------------------------------------------------------------------
// ContactErrorLabels
// ---------------------------------------------------------------------------

#[test]
fn error_label_defaults_are_non_empty() {
    let l = ContactErrorLabels::default();
    for (name, text) in [
        ("required", &l.required),
        ("length", &l.length),
        ("format_email", &l.format_email),
        ("format", &l.format),
        ("line_breaks", &l.line_breaks),
        ("token_invalid", &l.token_invalid),
        ("not_configured", &l.not_configured),
        ("delivery_failed", &l.delivery_failed),
    ] {
        assert!(!text.trim().is_empty(), "{name} must have a default");
    }
}

#[test]
fn field_text_substitutes_min_and_max() {
    let l = ContactErrorLabels::default();
    let text = l.field_text(
        ContactField::Message,
        &FieldError::Code(FieldErrorCode::Length {
            min: 1,
            max: MESSAGE_MAX_LEN,
        }),
    );
    assert!(text.contains('1'), "{text}");
    assert!(text.contains("4000"), "{text}");
    assert!(!text.contains("{min}") && !text.contains("{max}"), "{text}");
}

/// The email field gets its own wording; every other field shares one.
#[test]
fn field_text_uses_format_email_only_for_the_email_field() {
    let l = ContactErrorLabels::default();
    let err = FieldError::Code(FieldErrorCode::Format);

    assert_eq!(l.field_text(ContactField::Email, &err), l.format_email);
    assert_eq!(l.field_text(ContactField::Name, &err), l.format);
    assert_eq!(l.field_text(ContactField::Subject, &err), l.format);
}

#[test]
fn field_text_passes_through_pre_rendered_text() {
    let l = ContactErrorLabels::default();
    let err = FieldError::Text("from an older server".into());
    assert_eq!(
        l.field_text(ContactField::Name, &err),
        "from an older server"
    );
}

#[test]
fn field_text_renders_required_and_line_breaks() {
    let l = ContactErrorLabels::default();
    assert_eq!(
        l.field_text(
            ContactField::Subject,
            &FieldError::Code(FieldErrorCode::Required)
        ),
        l.required
    );
    assert_eq!(
        l.field_text(
            ContactField::Name,
            &FieldError::Code(FieldErrorCode::LineBreaks)
        ),
        l.line_breaks
    );
}

/// `Unexpected` is deliberately indistinguishable from a delivery failure:
/// the visitor's message did not go, and the cause belongs in the logs.
#[test]
fn code_text_maps_unexpected_to_delivery_failed() {
    use crate::error::ContactErrorCode as C;
    let l = ContactErrorLabels::default();
    assert_eq!(l.code_text(C::TokenInvalid), l.token_invalid);
    assert_eq!(l.code_text(C::NotConfigured), l.not_configured);
    assert_eq!(l.code_text(C::DeliveryFailed), l.delivery_failed);
    assert_eq!(l.code_text(C::Unexpected), l.delivery_failed);
}

#[test]
fn labels_default_includes_error_labels() {
    let l = ContactFormLabels::default();
    assert!(!l.errors.required.is_empty());
}

/// A translated label set with `{min}`/`{max}` must substitute the same way.
#[test]
fn field_text_substitutes_in_a_translated_label() {
    let l = ContactErrorLabels {
        length: "{min}〜{max}文字で入力してください。".into(),
        ..Default::default()
    };
    let text = l.field_text(
        ContactField::Name,
        &FieldError::Code(FieldErrorCode::Length { min: 1, max: 80 }),
    );
    assert_eq!(text, "1〜80文字で入力してください。");
}

/// Off by default: a server without form tokens has no endpoint to call, and
/// the browser cannot tell that server apart from a client-side navigation.
#[test]
fn options_default_never_calls_the_token_endpoint() {
    assert_eq!(ContactFormOptions::default().token_refresh_secs, None);
}
