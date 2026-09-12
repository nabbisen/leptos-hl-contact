// components.rs — Leptos UI components for the contact form.

use leptos::prelude::*;

use crate::{
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
    error::ContactFieldErrors,
    server::SubmitContact,
};

#[cfg(feature = "csrf")]
use crate::csrf::CsrfToken;

// ---------------------------------------------------------------------------
// Helper — small inline error paragraph
// ---------------------------------------------------------------------------

/// Renders an inline field-level error message with the appropriate ARIA
/// attributes so that screen readers announce it when the field is invalid.
///
/// `input_id` is the `id` of the sibling `<input>` — callers must set
/// `aria-describedby="{input_id}-error"` on that element.
///
/// `message` is a signal so the paragraph appears and disappears in place;
/// the surrounding form is built once and never rebuilt.
#[component]
fn FieldError(
    /// The `id` of the associated input; the error element id is `{input_id}-error`.
    input_id: &'static str,
    /// CSS class applied to the error paragraph.
    class: String,
    /// The error message to display.  When `None`, the element is not rendered.
    message: Signal<Option<String>>,
) -> impl IntoView {
    move || {
        message.get().map(|msg| {
            view! {
                <p
                    id=format!("{input_id}-error")
                    class=class.clone()
                    role="alert"
                    aria-live="polite"
                >
                    {msg}
                </p>
            }
        })
    }
}

// ---------------------------------------------------------------------------
// ContactForm
// ---------------------------------------------------------------------------

/// A fully accessible, customisable contact form component.
///
/// Uses `<ActionForm/>` for progressive-enhancement-friendly submission;
/// the form works as a plain HTML POST even without WebAssembly.
///
/// # Props
///
/// | Prop      | Type                    | Required | Default      |
/// |-----------|-------------------------|----------|--------------|
/// | `classes` | [`ContactFormClasses`]  | No       | all empty    |
/// | `labels`  | [`ContactFormLabels`]   | No       | English text |
/// | `options` | [`ContactFormOptions`]  | No       | show subject |
///
/// # State model
///
/// The `<form>` and its inputs are created once and kept across submissions.
/// Only the success region, the generic-error region, and each field's error
/// paragraph and ARIA attributes react to the action's value.  Keeping the
/// form itself out of the reactive closure is what preserves the hidden
/// anti-automation token, which the server renders once and the client cannot
/// regenerate.
///
/// # Accessibility
///
/// - Every input has an associated `<label>`.
/// - Required fields carry `aria-required="true"`.
/// - Fields with errors carry `aria-invalid="true"` and
///   `aria-describedby="{id}-error"`.
/// - After a failed submission focus moves to the first invalid input, unless
///   [`ContactFormOptions::focus_first_error`] is `false`.
/// - The honeypot field is hidden from sighted users and screen readers.
/// - Success and error messages use `role="status"` and `role="alert"`.
/// - The submit button is disabled while a submission is in flight.
///
/// # Example
///
/// ```rust,ignore
/// use leptos_hl_contact::{ContactForm, config::ContactFormClasses};
///
/// view! {
///     <ContactForm
///         classes=ContactFormClasses {
///             button: "btn btn-primary".into(),
///             ..Default::default()
///         }
///     />
/// }
/// ```
#[component]
pub fn ContactForm(
    /// CSS class overrides for structural elements.
    #[prop(optional, into)]
    classes: ContactFormClasses,
    /// User-visible text strings.
    #[prop(optional, into)]
    labels: ContactFormLabels,
    /// Behavioural options.
    #[prop(optional, into)]
    options: ContactFormOptions,
) -> impl IntoView {
    let submit_action = ServerAction::<SubmitContact>::new();
    let pending = submit_action.pending();
    let value = submit_action.value();

    // Read the token once, at creation.  SSR has it in context; the browser
    // does not, and the SSR-rendered attribute survives hydration because the
    // form is never rebuilt.
    #[cfg(feature = "csrf")]
    let csrf_token_value: String = leptos::context::use_context::<CsrfToken>()
        .map(|t| t.0.clone())
        .unwrap_or_default();
    #[cfg(not(feature = "csrf"))]
    let csrf_token_value: String = String::new();

    let classes = StoredValue::new(classes);
    let labels = StoredValue::new(labels);
    let options = StoredValue::new(options);

    // ---- derived state -----------------------------------------------------

    let succeeded = Memo::new(move |_| value.with(|v| matches!(v, Some(Ok(())))));

    let field_errors = Memo::new(move |_| {
        value.with(|v| match v {
            Some(Err(e)) => ContactFieldErrors::from_server_fn_error(e).unwrap_or_default(),
            _ => ContactFieldErrors::default(),
        })
    });

    // Every error that carries no field errors — the token failure and every
    // `ServerError` — shows the generic banner instead.
    let generic_error = Memo::new(move |_| {
        value.with(|v| match v {
            Some(Err(e))
                if ContactFieldErrors::from_server_fn_error(e).is_none_or(|f| f.is_empty()) =>
            {
                Some(labels.with_value(|l| l.error.clone()))
            }
            _ => None,
        })
    });

    // ---- focus the first invalid input (client only) -----------------------

    // `not(ssr)` as well as `hydrate`: a real client build has `ssr` off, but
    // `--all-features` (tests, rustdoc) turns both on, and this effect must
    // never run where there is no DOM.
    #[cfg(all(feature = "hydrate", not(feature = "ssr")))]
    if options.with_value(|o| o.focus_first_error) {
        Effect::new(move |_| {
            let first = field_errors.with(|f| {
                [
                    ("contact-name", f.name.is_some()),
                    ("contact-email", f.email.is_some()),
                    ("contact-subject", f.subject.is_some()),
                    ("contact-message", f.message.is_some()),
                ]
                .into_iter()
                .find(|(_, has)| *has)
                .map(|(id, _)| id)
            });
            if let Some(id) = first
                && let Some(el) = document().get_element_by_id(id)
            {
                use leptos::wasm_bindgen::JsCast;
                if let Ok(el) = el.dyn_into::<leptos::web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        });
    }

    // ---- markup ------------------------------------------------------------

    view! {
        <div class=move || classes.with_value(|c| c.root.clone())>

            // Success message
            {move || succeeded.get().then(|| {
                let sc = classes.with_value(|c| c.success.clone());
                let sm = labels.with_value(|l| l.success.clone());
                view! {
                    <div class=sc role="status" aria-live="polite">{sm}</div>
                }
            })}

            // Generic delivery-failure error (not a field validation error)
            {move || {
                let ec = classes.with_value(|c| c.error.clone());
                generic_error.get().map(|msg| view! {
                    <div class=ec role="alert" aria-live="assertive">{msg}</div>
                })
            }}

            // Form — built once, hidden after a successful submission.
            {move || (!succeeded.get()).then(|| {
                // Read once per build of the form.  This closure depends only
                // on `succeeded`, so a validation error never re-runs it.
                let (fc, lc, ic, tac, bc, ec) = classes.with_value(|c| (
                    c.field.clone(), c.label.clone(), c.input.clone(),
                    c.textarea.clone(), c.button.clone(), c.error.clone(),
                ));
                let (l_name, l_email, l_subject, l_message, l_submit, l_sending, l_honey) =
                    labels.with_value(|l| (
                        l.name.clone(), l.email.clone(), l.subject.clone(), l.message.clone(),
                        l.submit.clone(), l.sending.clone(), l.honeypot_label.clone(),
                    ));
                let show_subject = options.with_value(|o| o.show_subject);
                let require_subject = options.with_value(|o| o.require_subject);
                let max_msg_len = options.with_value(|o| o.effective_max_message_len());
                let csrf_token_value = csrf_token_value.clone();

                view! {
                    <ActionForm action=submit_action>

                        // Name
                        <div class=fc.clone()>
                            <label for="contact-name" class=lc.clone()>{l_name}</label>
                            <input
                                id="contact-name"
                                name="name"
                                type="text"
                                class=ic.clone()
                                required
                                maxlength="80"
                                autocomplete="name"
                                aria-required="true"
                                aria-invalid=move || field_errors.with(|f| f.name.is_some()).then_some("true")
                                aria-describedby=move || field_errors.with(|f| f.name.is_some()).then_some("contact-name-error")
                            />
                            <FieldError
                                input_id="contact-name"
                                class=ec.clone()
                                message=Signal::derive(move || field_errors.with(|f| f.name.clone()))
                            />
                        </div>

                        // Email
                        <div class=fc.clone()>
                            <label for="contact-email" class=lc.clone()>{l_email}</label>
                            <input
                                id="contact-email"
                                name="email"
                                type="email"
                                class=ic.clone()
                                required
                                maxlength="254"
                                autocomplete="email"
                                aria-required="true"
                                aria-invalid=move || field_errors.with(|f| f.email.is_some()).then_some("true")
                                aria-describedby=move || field_errors.with(|f| f.email.is_some()).then_some("contact-email-error")
                            />
                            <FieldError
                                input_id="contact-email"
                                class=ec.clone()
                                message=Signal::derive(move || field_errors.with(|f| f.email.clone()))
                            />
                        </div>

                        // Subject (conditional)
                        {show_subject.then(|| view! {
                            <div class=fc.clone()>
                                <label for="contact-subject" class=lc.clone()>{l_subject}</label>
                                <input
                                    id="contact-subject"
                                    name="subject"
                                    type="text"
                                    class=ic.clone()
                                    maxlength="120"
                                    required=require_subject
                                    aria-required=if require_subject { "true" } else { "false" }
                                    aria-invalid=move || field_errors.with(|f| f.subject.is_some()).then_some("true")
                                    aria-describedby=move || field_errors.with(|f| f.subject.is_some()).then_some("contact-subject-error")
                                />
                                <FieldError
                                    input_id="contact-subject"
                                    class=ec.clone()
                                    message=Signal::derive(move || field_errors.with(|f| f.subject.clone()))
                                />
                            </div>
                        })}

                        // Message
                        <div class=fc.clone()>
                            <label for="contact-message" class=lc.clone()>{l_message}</label>
                            <textarea
                                id="contact-message"
                                name="message"
                                class=tac
                                required
                                maxlength=max_msg_len.to_string()
                                rows="6"
                                aria-required="true"
                                aria-invalid=move || field_errors.with(|f| f.message.is_some()).then_some("true")
                                aria-describedby=move || field_errors.with(|f| f.message.is_some()).then_some("contact-message-error")
                            />
                            <FieldError
                                input_id="contact-message"
                                class=ec.clone()
                                message=Signal::derive(move || field_errors.with(|f| f.message.clone()))
                            />
                        </div>

                        // Anti-automation token — hidden field, rendered by SSR.
                        // Empty when the `csrf` feature is disabled or
                        // CsrfConfigContext is not provided.
                        <input
                            type="hidden"
                            name="csrf_token"
                            value=csrf_token_value
                        />

                        // Honeypot — visually hidden; excluded from assistive tech.
                        <div
                            aria-hidden="true"
                            style="position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden"
                        >
                            <label for="contact-website">{l_honey}</label>
                            <input
                                id="contact-website"
                                name="website"
                                type="text"
                                tabindex="-1"
                                autocomplete="off"
                            />
                        </div>

                        // Submit button
                        <div class=fc.clone()>
                            <button
                                type="submit"
                                class=bc
                                disabled=pending
                                aria-busy=move || if pending.get() { "true" } else { "false" }
                            >
                                {move || if pending.get() { l_sending.clone() } else { l_submit.clone() }}
                            </button>
                        </div>

                    </ActionForm>
                }
            })}

        </div>
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, feature = "ssr"))]
mod tests;
