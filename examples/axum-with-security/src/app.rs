// examples/axum-with-security/src/app.rs

use leptos::prelude::*;
use leptos_meta::{MetaTags, provide_meta_context};
use leptos_router::{
    components::{A, Route, Router, Routes},
    path,
};

use leptos_hl_contact::{
    ContactForm,
    config::{
        ChallengeProvider, ChallengeWidget, ContactFormClasses, ContactFormLabels,
        ContactFormOptions,
    },
};

// ---------------------------------------------------------------------------
// Challenge configuration
// ---------------------------------------------------------------------------
//
// CHALLENGE_PROVIDER  turnstile | hcaptcha | recaptcha-v2 | recaptcha-v3
// CHALLENGE_SITE_KEY  public; rendered into the page
// CHALLENGE_SECRET    server only; see main.rs
//
// The site key lives in the server's environment, which the browser build
// cannot read.  The shell renders the two public values into `<meta>` tags and
// the browser reads them back, so the server render, hydration and any later
// client-side navigation all build the same widget.

/// The reCAPTCHA v3 action this example sends and the server requires.
pub const RECAPTCHA_V3_ACTION: &str = "contact";

/// Parse `CHALLENGE_PROVIDER`.
pub fn parse_provider(name: &str) -> Option<ChallengeProvider> {
    match name {
        "turnstile" => Some(ChallengeProvider::Turnstile),
        "hcaptcha" => Some(ChallengeProvider::HCaptcha),
        "recaptcha-v2" => Some(ChallengeProvider::RecaptchaV2),
        "recaptcha-v3" => Some(ChallengeProvider::RecaptchaV3 {
            action: RECAPTCHA_V3_ACTION.into(),
        }),
        _ => None,
    }
}

/// The public challenge settings: provider name and site key.
#[cfg(feature = "ssr")]
fn challenge_settings() -> Option<(String, String)> {
    let get = |k| std::env::var(k).ok().filter(|v: &String| !v.is_empty());
    Some((get("CHALLENGE_PROVIDER")?, get("CHALLENGE_SITE_KEY")?))
}

/// The public challenge settings, read back from the shell's `<meta>` tags.
#[cfg(not(feature = "ssr"))]
fn challenge_settings() -> Option<(String, String)> {
    let meta = |name: &str| {
        document()
            .query_selector(&format!("meta[name=\"{name}\"]"))
            .ok()
            .flatten()?
            .get_attribute("content")
    };
    Some((meta("challenge-provider")?, meta("challenge-site-key")?))
}

/// The widget for this deployment, or `None` when no challenge is configured.
pub fn challenge_widget() -> Option<ChallengeWidget> {
    let (provider, site_key) = challenge_settings()?;
    ChallengeWidget::new(parse_provider(&provider)?, site_key).ok()
}

/// The HTML document the server renders around [`App`].
///
/// `HydrationScripts` emits the `<script>` tags that load
/// `/pkg/axum-with-security.js` and its `.wasm`, which is what makes the
/// page hydrate in the browser.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <AutoReload options=options.clone() />
                <HydrationScripts options=options.clone() />
                <MetaTags />
                // Only a configuration that builds a valid widget is published.
                {challenge_widget().and(challenge_settings()).map(|(provider, site_key)| view! {
                    <meta name="challenge-provider" content=provider />
                    <meta name="challenge-site-key" content=site_key />
                })}
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Router>
            <Routes fallback=|| view! { <p>"Page not found."</p> }>
                <Route path=path!("/") view=HomePage />
                <Route path=path!("/contact") view=ContactPage />
                <Route path=path!("/thanks") view=ThanksPage />
            </Routes>
        </Router>
    }
}

/// A page without the form, linking to it with a client-side `<A>`.
///
/// Following that link creates the form in the browser with no server render,
/// so its token field starts empty and `ContactForm` fetches a token.  Loading
/// `/contact` directly takes the other path: the server renders the token.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <main style="max-width: 600px; margin: 2rem auto; font-family: sans-serif; padding: 0 1rem;">
            <h1>"Welcome"</h1>
            <p><A href="/contact">"Contact us"</A></p>
        </main>
    }
}

#[component]
fn ContactPage() -> impl IntoView {
    // This server issues form tokens, so let the browser fetch one for a form
    // reached by client-side navigation and refresh it: `ttl_secs - 60`.
    let options = ContactFormOptions {
        token_refresh_secs: Some(3540),
        ..Default::default()
    };
    let classes = ContactFormClasses {
        root: "contact-form".into(),
        field: "contact-field".into(),
        label: "contact-label".into(),
        input: "contact-input".into(),
        textarea: "contact-textarea".into(),
        button: "contact-button".into(),
        error: "contact-error".into(),
        success: "contact-success".into(),
        ..Default::default()
    };

    view! {
        <main style="max-width: 600px; margin: 2rem auto; font-family: sans-serif; padding: 0 1rem;">
            <h1>"Contact us (secured)"</h1>
            <p style="color: #666; font-size: 0.9rem;">
                "This form is protected by: rate limiting, a form token, Origin validation, and a challenge when configured."
            </p>
            <ContactForm
                classes=classes
                labels=ContactFormLabels::default()
                options=options
                // `None` unless CHALLENGE_PROVIDER and CHALLENGE_SITE_KEY are set.
                challenge=challenge_widget()
            />
        </main>
    }
}

/// Where a successful submission lands, with or without JavaScript.
#[component]
fn ThanksPage() -> impl IntoView {
    view! {
        <main style="max-width: 600px; margin: 2rem auto; font-family: sans-serif; padding: 0 1rem;">
            <h1>"Thank you"</h1>
            <p>"Your message has been sent. We will get back to you soon."</p>
            <p><a href="/contact">"Back to the form"</a></p>
        </main>
    }
}
