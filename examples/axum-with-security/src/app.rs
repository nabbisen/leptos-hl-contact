// examples/axum-with-security/src/app.rs

use leptos::prelude::*;
use leptos_meta::{MetaTags, provide_meta_context};
use leptos_router::{
    components::{A, Route, Router, Routes},
    path,
};

use leptos_hl_contact::{
    ContactForm,
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
};

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

    view! {
        <main style="max-width: 600px; margin: 2rem auto; font-family: sans-serif; padding: 0 1rem;">
            <h1>"Contact us (secured)"</h1>
            <p style="color: #666; font-size: 0.9rem;">
                "This form is protected by: rate limiting, a form token, and Origin validation."
            </p>
            <ContactForm
                classes=ContactFormClasses {
                    root:     "contact-form".into(),
                    field:    "contact-field".into(),
                    label:    "contact-label".into(),
                    input:    "contact-input".into(),
                    textarea: "contact-textarea".into(),
                    button:   "contact-button".into(),
                    error:    "contact-error".into(),
                    success:  "contact-success".into(),
                }
                labels=ContactFormLabels::default()
                options=options
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
