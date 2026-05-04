// components.rs — Leptos UI components for the contact form.

use leptos::prelude::*;

use crate::{
    config::{ContactFormClasses, ContactFormLabels, ContactFormOptions},
    server::SubmitContact,
};

/// A fully accessible, customisable contact form component.
///
/// Uses `<ActionForm/>` for progressive-enhancement-friendly submission;
/// the form functions as a plain HTML POST even without WebAssembly.
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

    // Store config in reactive storage so closures can clone cheaply.
    let classes = StoredValue::new(classes);
    let labels = StoredValue::new(labels);
    let options = StoredValue::new(options);

    let succeeded = move || value.with(|v| matches!(v, Some(Ok(()))));

    let error_msg = move || {
        value.with(|v| match v {
            Some(Err(e)) => {
                let msg = e.to_string();
                let fallback = labels.with_value(|l| l.error.clone());
                if msg.contains("Please check the fields") {
                    msg
                } else {
                    fallback
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

            // Error message
            {move || {
                let msg = error_msg();
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

                view! {
                    <ActionForm action=submit_action>

                        // Name
                        <div class=fc.clone()>
                            <label for="contact-name" class=lc.clone()>{l_name}</label>
                            <input
                                id="contact-name" name="name" type="text"
                                class=ic.clone() required maxlength="80"
                                autocomplete="name" aria-required="true"
                            />
                        </div>

                        // Email
                        <div class=fc.clone()>
                            <label for="contact-email" class=lc.clone()>{l_email}</label>
                            <input
                                id="contact-email" name="email" type="email"
                                class=ic.clone() required maxlength="254"
                                autocomplete="email" aria-required="true"
                            />
                        </div>

                        // Subject (conditional)
                        {show_subject.then(|| view! {
                            <div class=fc.clone()>
                                <label for="contact-subject" class=lc.clone()>{l_subject}</label>
                                <input
                                    id="contact-subject" name="subject" type="text"
                                    class=ic.clone() maxlength="120"
                                    required=require_subject
                                    aria-required=if require_subject { "true" } else { "false" }
                                />
                            </div>
                        })}

                        // Message
                        <div class=fc.clone()>
                            <label for="contact-message" class=lc.clone()>{l_message}</label>
                            <textarea
                                id="contact-message" name="message"
                                class=tac required maxlength=max_msg_len.to_string()
                                rows="6" aria-required="true"
                            />
                        </div>

                        // Honeypot — visually hidden; excluded from assistive tech.
                        <div
                            aria-hidden="true"
                            style="position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden"
                        >
                            <label for="contact-website">{l_honey}</label>
                            <input
                                id="contact-website" name="website" type="text"
                                tabindex="-1" autocomplete="off"
                            />
                        </div>

                        // Submit button
                        <div class=fc>
                            <button
                                type="submit" class=bc
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
