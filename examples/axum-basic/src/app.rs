// examples/axum-basic/src/app.rs
//
// Minimal Leptos application that mounts the ContactForm component.

use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use leptos_hl_contact::{
    ContactForm,
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
};

/// Root application component.
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

/// A simple page that centres the contact form.
#[component]
fn ContactPage() -> impl IntoView {
    view! {
        <main style="max-width: 600px; margin: 2rem auto; padding: 0 1rem; font-family: sans-serif;">
            <h1>"Contact us"</h1>
            <ContactForm
                classes=ContactFormClasses {
                    root: "contact-form".into(),
                    field: "contact-field".into(),
                    label: "contact-label".into(),
                    input: "contact-input".into(),
                    textarea: "contact-textarea".into(),
                    button: "contact-button".into(),
                    error: "contact-error".into(),
                    success: "contact-success".into(),
                }
                labels=ContactFormLabels::default()
                options=ContactFormOptions::default()
            />
        </main>
    }
}
