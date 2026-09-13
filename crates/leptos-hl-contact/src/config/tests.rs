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
    assert_eq!(l.code_text(C::ChallengeRequired), l.challenge_required);
    assert_eq!(l.code_text(C::ChallengeFailed), l.challenge_failed);
    assert_eq!(
        l.code_text(C::ChallengeUnavailable),
        l.challenge_unavailable
    );
    assert_eq!(l.code_text(C::Rejected), l.rejected);
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

/// RFC 005 D3's English defaults.
#[test]
fn challenge_labels_have_the_rfc_defaults() {
    let l = ContactErrorLabels::default();
    assert_eq!(l.challenge_required, "Please complete the security check.");
    assert_eq!(
        l.challenge_failed,
        "The security check did not pass. Please try again."
    );
    assert_eq!(
        l.challenge_unavailable,
        "The security check is unavailable right now. Please try again later."
    );
    assert_eq!(
        l.challenge_requires_js,
        "This form needs JavaScript to verify you are human."
    );
}

#[test]
fn no_js_policy_defaults_to_reject() {
    assert_eq!(NoJsPolicy::default(), NoJsPolicy::Reject);
}

// ---------------------------------------------------------------------------
// ChallengeWidget validation (RFC 005 handoff 02)
// ---------------------------------------------------------------------------

#[test]
fn widget_new_rejects_unsafe_or_empty_site_keys() {
    for bad in ["abc$", "", "a b", "key\"", "<script>", "k'ey"] {
        assert!(
            ChallengeWidget::new(ChallengeProvider::Turnstile, bad).is_err(),
            "{bad:?} must be rejected"
        );
    }
}

#[test]
fn widget_new_accepts_vendor_test_keys() {
    for key in [
        "1x00000000000000000000AA",
        "10000000-ffff-ffff-ffff-000000000001",
        "6LeIxAcTAAAAAJcZVRqyHh71UMIEGNQ_MXjiZKhI",
    ] {
        let w = ChallengeWidget::new(ChallengeProvider::Turnstile, key).expect(key);
        assert_eq!(w.site_key, key);
        assert!(w.load_script);
        assert_eq!(w.theme, ChallengeTheme::Auto);
        assert_eq!(w.no_js, NoJsPolicy::Reject);
    }
}

/// The action is embedded in the inline script too.
#[test]
fn widget_new_validates_the_recaptcha_v3_action() {
    let ok = ChallengeProvider::RecaptchaV3 {
        action: "contact_form".into(),
    };
    assert!(ChallengeWidget::new(ok, "SITE").is_ok());
    for bad in ["", "contact'});alert(1);//", "a b"] {
        let p = ChallengeProvider::RecaptchaV3 { action: bad.into() };
        assert!(ChallengeWidget::new(p, "SITE").is_err(), "{bad:?}");
    }
}

#[test]
fn widget_language_must_look_like_bcp47() {
    let w = || ChallengeWidget::new(ChallengeProvider::HCaptcha, "SITE").unwrap();
    for good in ["en", "fra", "pt-BR", "zh-Hant-TW", "es-419"] {
        assert!(w().with_language(good).is_ok(), "{good:?}");
    }
    for bad in [
        "",
        "e",
        "english",
        "en_US",
        "en-",
        "-en",
        "en-B",
        "en-toolongsub",
        "en\"",
        "en&x=1",
    ] {
        assert!(w().with_language(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn widget_script_nonce_must_be_base64() {
    let w = || ChallengeWidget::new(ChallengeProvider::Turnstile, "SITE").unwrap();
    for good in ["abc123", "rAnd0m+/nonce==", "ZXhhbXBsZQ"] {
        assert!(w().with_script_nonce(good).is_ok(), "{good:?}");
    }
    for bad in ["", "abc\"", "a b", "abc-def", "<x>"] {
        assert!(w().with_script_nonce(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn widget_builders_set_their_fields() {
    let w = ChallengeWidget::new(ChallengeProvider::Turnstile, "SITE")
        .unwrap()
        .with_theme(ChallengeTheme::Dark)
        .with_language("de")
        .unwrap()
        .without_script()
        .with_script_nonce("bm9uY2U=")
        .unwrap()
        .with_no_js(NoJsPolicy::AcceptWithHoneypotOnly);
    assert_eq!(w.theme, ChallengeTheme::Dark);
    assert_eq!(w.language.as_deref(), Some("de"));
    assert!(!w.load_script);
    assert_eq!(w.script_nonce.as_deref(), Some("bm9uY2U="));
    assert_eq!(w.no_js, NoJsPolicy::AcceptWithHoneypotOnly);
}

#[test]
fn widget_script_urls_carry_the_language_where_the_vendor_reads_it() {
    let src = |p: ChallengeProvider, lang: Option<&str>| {
        let w = ChallengeWidget::new(p, "SITE").unwrap();
        match lang {
            Some(l) => w.with_language(l).unwrap(),
            None => w,
        }
        .script_src()
    };
    assert_eq!(
        src(ChallengeProvider::Turnstile, Some("fr")),
        "https://challenges.cloudflare.com/turnstile/v0/api.js"
    );
    assert_eq!(
        src(ChallengeProvider::HCaptcha, Some("fr")),
        "https://js.hcaptcha.com/1/api.js?hl=fr"
    );
    assert_eq!(
        src(ChallengeProvider::RecaptchaV2, None),
        "https://www.google.com/recaptcha/api.js"
    );
    assert_eq!(
        src(
            ChallengeProvider::RecaptchaV3 {
                action: "contact".into()
            },
            Some("fr")
        ),
        "https://www.google.com/recaptcha/api.js?render=SITE&hl=fr"
    );
}

/// RFC 006 D1's default, distinct from every token and challenge label so a
/// reader can tell which layer acted.
#[test]
fn rejected_label_has_the_rfc_default_and_is_distinct() {
    let l = ContactErrorLabels::default();
    assert_eq!(l.rejected, "Your message could not be accepted.");
    for other in [
        &l.token_invalid,
        &l.too_fast,
        &l.not_configured,
        &l.delivery_failed,
        &l.challenge_required,
        &l.challenge_failed,
        &l.challenge_unavailable,
        &l.challenge_requires_js,
    ] {
        assert_ne!(&l.rejected, other);
    }
}
