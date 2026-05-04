// components.rs — Leptos UI components for the contact form.

use leptos::prelude::*;

use crate::{
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
    error::{ContactFieldErrors, FIELD_ERROR_PREFIX},
    server::SubmitContact,
};

// ---------------------------------------------------------------------------
// Helper — small inline error paragraph
// ---------------------------------------------------------------------------

/// Renders an inline field-level error message with the appropriate ARIA
/// attributes so that screen readers announce it when the field is invalid.
///
/// `input_id` is the `id` of the sibling `<input>` — callers must set
/// `aria-describedby="{input_id}-error"` on that element.
#[component]
fn FieldError(
    /// The `id` of the associated input; the error element id is `{input_id}-error`.
    input_id: &'static str,
    /// CSS class applied to the error paragraph.
    class: String,
    /// The error message to display.  When empty, the element is not rendered.
    message: Option<String>,
) -> impl IntoView {
    message.map(|msg| {
        view! {
            <p
                id=format!("{input_id}-error")
                class=class
                role="alert"
                aria-live="polite"
            >
                {msg}
            </p>
        }
    })
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
/// # Accessibility
///
/// - Every input has an associated `<label>`.
/// - Required fields carry `aria-required="true"`.
/// - Fields with errors carry `aria-invalid="true"` and
///   `aria-describedby="{id}-error"`.
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

    let classes = StoredValue::new(classes);
    let labels = StoredValue::new(labels);
    let options = StoredValue::new(options);

    // True when the last submission succeeded.
    let succeeded = move || value.with(|v| matches!(v, Some(Ok(()))));

    // Parse field-level errors from the ServerFnError payload.
    let field_errors = move || {
        value.with(|v| match v {
            Some(Err(e)) => {
                let s = e.to_string();
                if s.starts_with(FIELD_ERROR_PREFIX) {
                    ContactFieldErrors::from_error_str(&s).unwrap_or_default()
                } else {
                    ContactFieldErrors::default()
                }
            }
            _ => ContactFieldErrors::default(),
        })
    };

    // Generic delivery-failure message (non-field errors only).
    let generic_error = move || {
        value.with(|v| match v {
            Some(Err(e)) => {
                let s = e.to_string();
                if s.starts_with(FIELD_ERROR_PREFIX) {
                    String::new() // handled per-field
                } else if !s.is_empty() {
                    labels.with_value(|l| l.error.clone())
                } else {
                    String::new()
                }
            }
            _ => String::new(),
        })
    };

    view! {
        <div class=move || classes.with_value(|c| c.root.clone())>

            // Success message
            {move || succeeded().then(|| {
                let sc = classes.with_value(|c| c.success.clone());
                let sm = labels.with_value(|l| l.success.clone());
                view! {
                    <div class=sc role="status" aria-live="polite">{sm}</div>
                }
            })}

            // Generic delivery-failure error (not a field validation error)
            {move || {
                let msg = generic_error();
                (!msg.is_empty()).then(|| {
                    let ec = classes.with_value(|c| c.error.clone());
                    view! {
                        <div class=ec role="alert" aria-live="assertive">{msg}</div>
                    }
                })
            }}

            // Form — hidden after successful submission.
            {move || (!succeeded()).then(|| {
                let fc  = classes.with_value(|c| c.field.clone());
                let lc  = classes.with_value(|c| c.label.clone());
                let ic  = classes.with_value(|c| c.input.clone());
                let tac = classes.with_value(|c| c.textarea.clone());
                let bc  = classes.with_value(|c| c.button.clone());
                let ec  = classes.with_value(|c| c.error.clone());

                let l_name    = labels.with_value(|l| l.name.clone());
                let l_email   = labels.with_value(|l| l.email.clone());
                let l_subject = labels.with_value(|l| l.subject.clone());
                let l_message = labels.with_value(|l| l.message.clone());
                let l_submit  = labels.with_value(|l| l.submit.clone());
                let l_sending = labels.with_value(|l| l.sending.clone());
                let l_honey   = labels.with_value(|l| l.honeypot_label.clone());

                let show_subject    = options.with_value(|o| o.show_subject);
                let require_subject = options.with_value(|o| o.require_subject);
                let max_msg_len     = options.with_value(|o| o.max_message_len);

                let fe = field_errors();

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
                                aria-invalid=fe.name.as_ref().map(|_| "true")
                                aria-describedby=fe.name.as_ref().map(|_| "contact-name-error")
                            />
                            <FieldError input_id="contact-name" class=ec.clone() message=fe.name />
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
                                aria-invalid=fe.email.as_ref().map(|_| "true")
                                aria-describedby=fe.email.as_ref().map(|_| "contact-email-error")
                            />
                            <FieldError input_id="contact-email" class=ec.clone() message=fe.email />
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
                                    aria-invalid=fe.subject.as_ref().map(|_| "true")
                                    aria-describedby=fe.subject.as_ref().map(|_| "contact-subject-error")
                                />
                                <FieldError input_id="contact-subject" class=ec.clone() message=fe.subject />
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
                                aria-invalid=fe.message.as_ref().map(|_| "true")
                                aria-describedby=fe.message.as_ref().map(|_| "contact-message-error")
                            />
                            <FieldError input_id="contact-message" class=ec.clone() message=fe.message />
                        </div>

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
                        <div class=fc>
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
