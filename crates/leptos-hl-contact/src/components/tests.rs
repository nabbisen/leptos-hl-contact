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

mod attributes;

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

// ---------------------------------------------------------------------------
// Challenge widget (RFC 005 handoff 02)
// ---------------------------------------------------------------------------

use crate::config::{ChallengeProvider, ChallengeTheme, ChallengeWidget, NoJsPolicy};

const TURNSTILE_TEST_KEY: &str = "1x00000000000000000000AA";

fn render_with(widget: ChallengeWidget) -> String {
    render(move || view! { <ContactForm challenge=Some(widget) /> }.into_any())
}

fn turnstile() -> ChallengeWidget {
    ChallengeWidget::new(ChallengeProvider::Turnstile, TURNSTILE_TEST_KEY).unwrap()
}

#[test]
fn no_challenge_prop_renders_no_widget_and_no_script() {
    let html = render(|| view! { <ContactForm /> }.into_any());
    for needle in [
        "cf-turnstile",
        "h-captcha",
        "g-recaptcha",
        "<script",
        "<noscript",
    ] {
        assert!(!html.contains(needle), "{needle} without the prop:\n{html}");
    }
}

#[test]
fn turnstile_renders_its_element_and_script() {
    let html = render_with(turnstile());
    assert!(html.contains("class=\"cf-turnstile\""), "{html}");
    assert!(html.contains(&format!("data-sitekey=\"{TURNSTILE_TEST_KEY}\"")));
    assert!(
        html.contains("data-theme=\"auto\""),
        "Turnstile has an auto theme"
    );
    assert!(html.contains("src=\"https://challenges.cloudflare.com/turnstile/v0/api.js\""));
    assert_eq!(count(&html, "<script"), 1, "{html}");
    assert!(html.contains(" async") && html.contains(" defer"), "{html}");
}

/// Between the message field and the hidden token, as the handoff places it.
#[test]
fn the_widget_sits_after_the_message_and_before_the_token() {
    let html = render_with(turnstile());
    let message = html.find("id=\"contact-message\"").unwrap();
    let widget = html.find("cf-turnstile").unwrap();
    let token = html.find("name=\"form_token\"").unwrap();
    assert!(message < widget && widget < token, "{html}");
}

#[test]
fn turnstile_language_is_a_data_attribute() {
    let html = render_with(turnstile().with_language("pt-BR").unwrap());
    assert!(html.contains("data-language=\"pt-BR\""), "{html}");
}

#[test]
fn hcaptcha_renders_its_element_and_script_and_omits_auto_theme() {
    let w = ChallengeWidget::new(
        ChallengeProvider::HCaptcha,
        "10000000-ffff-ffff-ffff-000000000001",
    )
    .unwrap();
    let html = render_with(w.clone());
    assert!(html.contains("class=\"h-captcha\""), "{html}");
    assert!(
        !html.contains("data-theme"),
        "Auto is omitted for hCaptcha:\n{html}"
    );
    assert!(
        html.contains("src=\"https://js.hcaptcha.com/1/api.js\""),
        "{html}"
    );

    let html = render_with(
        w.with_theme(ChallengeTheme::Dark)
            .with_language("fr")
            .unwrap(),
    );
    assert!(html.contains("data-theme=\"dark\""), "{html}");
    assert!(
        html.contains("src=\"https://js.hcaptcha.com/1/api.js?hl=fr\""),
        "{html}"
    );
}

#[test]
fn recaptcha_v2_renders_its_element_and_script_and_omits_auto_theme() {
    let w = ChallengeWidget::new(ChallengeProvider::RecaptchaV2, "SITE").unwrap();
    let html = render_with(w.clone());
    assert!(html.contains("class=\"g-recaptcha\""), "{html}");
    assert!(!html.contains("data-theme"), "{html}");
    assert!(
        html.contains("src=\"https://www.google.com/recaptcha/api.js\""),
        "{html}"
    );

    let html = render_with(w.with_theme(ChallengeTheme::Light));
    assert!(html.contains("data-theme=\"light\""), "{html}");
}

#[test]
fn recaptcha_v3_renders_the_hidden_input_render_url_and_submit_script() {
    let w = ChallengeWidget::new(
        ChallengeProvider::RecaptchaV3 {
            action: "contact".into(),
        },
        "SITE",
    )
    .unwrap();
    let html = render_with(w);
    assert!(
        html.contains("type=\"hidden\" name=\"g-recaptcha-response\""),
        "{html}"
    );
    assert!(html.contains("api.js?render=SITE"), "{html}");
    assert!(html.contains("grecaptcha.execute('SITE'"), "{html}");
    assert!(html.contains("action:'contact'"), "{html}");
    assert_eq!(
        count(&html, "<script"),
        2,
        "vendor script and submit script"
    );
}

#[test]
fn without_script_emits_no_vendor_script() {
    let html = render_with(turnstile().without_script());
    assert!(!html.contains("<script"), "{html}");
    assert!(
        html.contains("cf-turnstile"),
        "the element is still rendered"
    );
}

/// v3 cannot work without its submit hook, so only the vendor tag goes.
#[test]
fn recaptcha_v3_without_script_keeps_only_the_submit_script() {
    let w = ChallengeWidget::new(
        ChallengeProvider::RecaptchaV3 {
            action: "contact".into(),
        },
        "SITE",
    )
    .unwrap()
    .without_script();
    let html = render_with(w);
    assert_eq!(count(&html, "<script"), 1, "{html}");
    assert!(!html.contains("api.js"), "{html}");
}

#[test]
fn reject_renders_noscript_and_accept_does_not() {
    let html = render_with(turnstile());
    assert!(html.contains("<noscript>"), "{html}");
    assert!(
        html.contains("This form needs JavaScript to verify you are human."),
        "{html}"
    );
    assert!(html.contains("role=\"alert\""), "{html}");

    let html = render_with(turnstile().with_no_js(NoJsPolicy::AcceptWithHoneypotOnly));
    assert!(!html.contains("<noscript"), "{html}");
}

/// The label and the class are integrator text inside raw HTML.
#[test]
fn the_noscript_message_escapes_label_and_class() {
    use crate::config::{ContactFormClasses, ContactFormLabels};

    let mut labels = ContactFormLabels::default();
    labels.errors.challenge_requires_js = "<b>JS</b> & \"more\"".into();
    let classes = ContactFormClasses {
        error: "err\" onclick=\"x".into(),
        ..Default::default()
    };
    let widget = turnstile();
    let html = render(move || {
        view! { <ContactForm challenge=Some(widget) labels=labels classes=classes /> }.into_any()
    });
    let noscript = &html[html.find("<noscript>").unwrap()..html.find("</noscript>").unwrap()];
    assert!(
        noscript.contains("&lt;b&gt;JS&lt;/b&gt; &amp; &quot;more&quot;"),
        "{noscript}"
    );
    assert!(
        noscript.contains("class=\"err&quot; onclick=&quot;x\""),
        "{noscript}"
    );
    assert!(!noscript.contains("<b>"), "{noscript}");
}

#[test]
fn the_nonce_is_on_every_script_tag_when_set() {
    let w = ChallengeWidget::new(
        ChallengeProvider::RecaptchaV3 {
            action: "contact".into(),
        },
        "SITE",
    )
    .unwrap()
    .with_script_nonce("bm9uY2U=")
    .unwrap();
    let html = render_with(w);
    assert_eq!(count(&html, "nonce=\"bm9uY2U=\""), 2, "{html}");

    let html = render_with(turnstile());
    assert!(!html.contains("nonce="), "no nonce unless set:\n{html}");
}

/// FR-ABUSE-10, runtime report 2026-09-16 §4.1: a base64url nonce, as
/// Leptos generates, reaches every script tag.
#[test]
fn a_url_safe_nonce_is_on_every_script_tag() {
    let w = ChallengeWidget::new(
        ChallengeProvider::RecaptchaV3 {
            action: "contact".into(),
        },
        "SITE",
    )
    .unwrap()
    .with_script_nonce("abc-DEF_123")
    .unwrap();
    let html = render_with(w);
    assert_eq!(count(&html, "nonce=\"abc-DEF_123\""), 2, "{html}");
}

#[test]
fn escape_html_escapes_the_four_characters() {
    use crate::components::escape_html;
    assert_eq!(escape_html(r#"a&b<c>d"e'f"#), "a&amp;b&lt;c&gt;d&quot;e'f");
    assert_eq!(escape_html("plain"), "plain");
}

/// Review C1: the prop takes an `Option`, so configuration-driven code passes
/// one value and `None` renders exactly as the absent prop does.
#[test]
fn challenge_none_renders_as_the_absent_prop() {
    let none: Option<ChallengeWidget> = None;
    let with_none = render(move || view! { <ContactForm challenge=none /> }.into_any());
    let absent = render(|| view! { <ContactForm /> }.into_any());
    assert_eq!(with_none, absent);
}
