// examples/axum-with-security/src/app.rs

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use leptos_hl_contact::{
    ContactForm,
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <p>"Page not found."</p> }>
                <Route path=path!("/") view=ContactPage />
            </Routes>
        </Router>
    }
}

#[component]
fn ContactPage() -> impl IntoView {
    view! {
        <main style="max-width: 600px; margin: 2rem auto; font-family: sans-serif; padding: 0 1rem;">
            <h1>"Contact us (secured)"</h1>
            <p style="color: #666; font-size: 0.9rem;">
                "This form is protected by: rate limiting, CSRF tokens, and Origin validation."
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
                options=ContactFormOptions::default()
            />
        </main>
    }
}
