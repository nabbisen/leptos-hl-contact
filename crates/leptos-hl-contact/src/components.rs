// components.rs — Leptos UI components for the contact form.

use leptos::either::Either;
use leptos::prelude::*;

use crate::{
    config::{
        ChallengeProvider, ChallengeWidget, ContactFormClasses, ContactFormLabels,
        ContactFormOptions, NoJsPolicy, SiteField, SiteFieldKind, SiteFields,
    },
    error::{ContactErrorCode, ContactField, ContactFieldErrors},
    server::SubmitContact,
};

#[cfg(feature = "form-token")]
use crate::form_token::FormToken;

// ---------------------------------------------------------------------------
// Helper — small inline error paragraph
// ---------------------------------------------------------------------------

/// Renders an inline field-level error message with the appropriate ARIA
/// attributes so that screen readers announce it when the field is invalid.
///
/// `input_id` is the `id` of the sibling control — callers must set
/// `aria-describedby="{input_id}-error"` on that element.
///
/// `message` is a signal so the paragraph appears and disappears in place;
/// the surrounding form is built once and never rebuilt.
#[component]
fn FieldError(
    /// The `id` of the associated input; the error element id is `{input_id}-error`.
    #[prop(into)]
    input_id: String,
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
// Challenge widget markup
// ---------------------------------------------------------------------------

/// Escape text for an HTML attribute value or element content.
///
/// Used only where the component writes raw HTML — the `<noscript>` message —
/// because Leptos escapes everything else itself.
pub(crate) fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// reCAPTCHA v3's submit hook: cancel the first submit, fetch a token, write
/// it, and submit again.  v3 tokens are single-use, so this runs every time.
///
/// `site_key` and `action` are validated to `[A-Za-z0-9_-]+` by
/// [`ChallengeWidget::new`], which is what makes embedding them safe.
pub(crate) fn recaptcha_v3_script(site_key: &str, action: &str) -> String {
    debug_assert!([site_key, action].iter().all(|s| {
        s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }));
    format!(
        "(function(){{\
var s=document.currentScript,f=s.closest('form'),\
i=f.querySelector('input[name=\"g-recaptcha-response\"]');\
f.addEventListener('submit',function(e){{\
if(i.dataset.fresh==='1'){{i.dataset.fresh='';return;}}\
e.preventDefault();\
grecaptcha.ready(function(){{\
grecaptcha.execute('{site_key}',{{action:'{action}'}}).then(function(t){{\
i.value=t;i.dataset.fresh='1';f.requestSubmit();}});}});\
}},true);}})();"
    )
}

// ---------------------------------------------------------------------------
// Challenge widget in the browser
// ---------------------------------------------------------------------------

/// What the browser has to do so the vendor renders the widget.
///
/// Every vendor scans the page for its widget class once, when its script
/// loads.  An element that arrives later — client-side navigation — is never
/// scanned, so it needs the vendor's explicit `render`.  And a second copy of
/// the script loads the vendor twice, which Turnstile warns about.
#[cfg(all(feature = "hydrate", not(feature = "ssr")))]
mod challenge_client {
    use leptos::prelude::document;
    use leptos::wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use leptos::web_sys::{
        self,
        js_sys::{Function, Object, Reflect},
    };

    use crate::config::{ChallengeProvider, ChallengeWidget};

    fn global_name(provider: &ChallengeProvider) -> &'static str {
        match provider {
            ChallengeProvider::Turnstile => "turnstile",
            ChallengeProvider::HCaptcha => "hcaptcha",
            ChallengeProvider::RecaptchaV2 | ChallengeProvider::RecaptchaV3 { .. } => "grecaptcha",
        }
    }

    fn vendor_global(widget: &ChallengeWidget) -> Option<JsValue> {
        let window = web_sys::window()?;
        Reflect::get(&window, &global_name(&widget.provider).into())
            .ok()
            .filter(JsValue::is_object)
    }

    /// Whether this render is hydrating server markup.
    pub(super) fn hydrating() -> bool {
        leptos::reactive::owner::Owner::current_shared_context()
            .is_some_and(|context| context.during_hydration())
    }

    /// Whether the vendor script has already run.
    pub(super) fn vendor_loaded(widget: &ChallengeWidget) -> bool {
        vendor_global(widget).is_some()
    }

    /// Whether a `<script>` with this exact `src` is in the document.
    ///
    /// `src` comes from [`ChallengeWidget::script_src`]: fixed URLs plus
    /// validated `[A-Za-z0-9_-]` values, so it holds no `"` or `\` and is safe
    /// inside a quoted attribute selector.
    pub(super) fn script_present(src: &str) -> bool {
        document()
            .query_selector(&format!("script[src=\"{src}\"]"))
            .ok()
            .flatten()
            .is_some()
    }

    /// Add the vendor script to `<head>`, where navigation does not remove it.
    pub(super) fn append_script(src: &str, nonce: Option<&str>) {
        let doc = document();
        let Ok(script) = doc.create_element("script") else {
            return;
        };
        let _ = script.set_attribute("src", src);
        let _ = script.set_attribute("async", "");
        let _ = script.set_attribute("defer", "");
        if let Some(nonce) = nonce {
            let _ = script.set_attribute("nonce", nonce);
        }
        if let Some(head) = doc.head() {
            let _ = head.append_child(&script);
        }
    }

    /// Ask an already-loaded vendor to render into `el`.
    ///
    /// Returns the widget id where the vendor can later remove the widget by
    /// it (Turnstile, hCaptcha).
    pub(super) fn render_explicitly(
        widget: &ChallengeWidget,
        el: &web_sys::HtmlElement,
    ) -> Option<String> {
        let global = vendor_global(widget)?;
        let params = Object::new();
        let set = |k: &str, v: &str| {
            let _ = Reflect::set(&params, &k.into(), &v.into());
        };
        set("sitekey", &widget.site_key);
        match &widget.provider {
            ChallengeProvider::Turnstile => {
                set("theme", widget.theme.turnstile_value());
                if let Some(size) = widget.size.size_value(&widget.provider) {
                    set("size", size);
                }
                if let Some(language) = &widget.language {
                    set("language", language);
                }
            }
            ChallengeProvider::HCaptcha | ChallengeProvider::RecaptchaV2 => {
                if let Some(theme) = widget.theme.explicit_value() {
                    set("theme", theme);
                }
                if let Some(size) = widget.size.size_value(&widget.provider) {
                    set("size", size);
                }
            }
            // No element to render: v3 only fetches tokens on submit.
            ChallengeProvider::RecaptchaV3 { .. } => return None,
        }
        let render = Reflect::get(&global, &"render".into())
            .ok()?
            .dyn_into::<Function>()
            .ok()?;
        let el: JsValue = el.clone().into();

        match &widget.provider {
            // reCAPTCHA's `render` is usable only inside `ready`, which runs
            // the callback at once when the script has already loaded.
            ChallengeProvider::RecaptchaV2 => {
                let ready = Reflect::get(&global, &"ready".into())
                    .ok()
                    .and_then(|f| f.dyn_into::<Function>().ok());
                match ready {
                    Some(ready) => {
                        let target = global.clone();
                        let callback = Closure::once_into_js(move || {
                            let _ = render.call2(&target, &el, &params);
                        });
                        let _ = ready.call1(&global, &callback);
                    }
                    None => {
                        let _ = render.call2(&global, &el, &params);
                    }
                }
                None
            }
            // Turnstile does not run a `ready` callback registered after it
            // has loaded, and warns that such a call "would break".  This path
            // runs only once the vendor is loaded, so render directly.
            _ => render
                .call2(&global, &el, &params)
                .ok()
                .and_then(|id| id.as_string()),
        }
    }

    /// Remove a widget the component rendered, so the vendor stops tracking
    /// an element that navigation has taken out of the page.
    pub(super) fn remove_widget(provider: &ChallengeProvider, id: &str) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(global) = Reflect::get(&window, &global_name(provider).into())
            .ok()
            .filter(JsValue::is_object)
        else {
            return;
        };
        if let Some(remove) = Reflect::get(&global, &"remove".into())
            .ok()
            .and_then(|f| f.dyn_into::<Function>().ok())
        {
            let _ = remove.call1(&global, &id.into());
        }
    }
}

/// The widget, its scripts, and the no-JavaScript explanation, in one field
/// wrapper.
fn challenge_markup(
    widget: &ChallengeWidget,
    field_class: String,
    error_class: String,
    requires_js: String,
) -> AnyView {
    let key = widget.site_key.clone();
    let widget_ref = NodeRef::<leptos::html::Div>::new();
    let element = match &widget.provider {
        ChallengeProvider::Turnstile => view! {
            <div
                node_ref=widget_ref
                class="cf-turnstile"
                data-sitekey=key.clone()
                data-theme=widget.theme.turnstile_value()
                data-size=widget.size.size_value(&widget.provider)
                data-language=widget.language.clone()
            ></div>
        }
        .into_any(),
        ChallengeProvider::HCaptcha => view! {
            <div
                node_ref=widget_ref
                class="h-captcha"
                data-sitekey=key.clone()
                data-theme=widget.theme.explicit_value()
                data-size=widget.size.size_value(&widget.provider)
            ></div>
        }
        .into_any(),
        ChallengeProvider::RecaptchaV2 => view! {
            <div
                node_ref=widget_ref
                class="g-recaptcha"
                data-sitekey=key.clone()
                data-theme=widget.theme.explicit_value()
                data-size=widget.size.size_value(&widget.provider)
            ></div>
        }
        .into_any(),
        ChallengeProvider::RecaptchaV3 { .. } => view! {
            <input type="hidden" name="g-recaptcha-response" />
        }
        .into_any(),
    };

    // The server always renders the vendor script, and hydration has to find
    // it.  A client-side render leaves it to the effect below, which adds it
    // to `<head>` once instead of once per visit to the form.
    #[cfg(all(feature = "hydrate", not(feature = "ssr")))]
    let script_in_view = widget.load_script && challenge_client::hydrating();
    #[cfg(not(all(feature = "hydrate", not(feature = "ssr"))))]
    let script_in_view = widget.load_script;

    #[cfg(all(feature = "hydrate", not(feature = "ssr")))]
    {
        // Both facts are taken now, in the same task that mounts the element,
        // so no script `onload` can run in between.
        let hydrating = challenge_client::hydrating();
        let loaded = challenge_client::vendor_loaded(widget);
        let rendered_id = StoredValue::new(None::<String>);
        let provider = widget.provider.clone();
        let widget = widget.clone();
        Effect::new(move |_| {
            if hydrating {
                // The server-rendered script scans this element when it loads,
                // or already has.
                return;
            }
            if loaded {
                // The vendor scanned the page before this element existed.
                if let Some(el) = widget_ref.get() {
                    rendered_id.set_value(challenge_client::render_explicitly(&widget, &el));
                }
                return;
            }
            let src = widget.script_src();
            if widget.load_script && !challenge_client::script_present(&src) {
                challenge_client::append_script(&src, widget.script_nonce.as_deref());
            }
        });
        on_cleanup(move || {
            if let Some(id) = rendered_id.get_value() {
                challenge_client::remove_widget(&provider, &id);
            }
        });
    }
    #[cfg(not(all(feature = "hydrate", not(feature = "ssr"))))]
    let _ = widget_ref;

    let vendor_script = script_in_view.then(|| {
        view! {
            <script src=widget.script_src() async defer nonce=widget.script_nonce.clone()></script>
        }
    });

    // The only raw script: its two inserted values are validated.
    let v3_script = match &widget.provider {
        ChallengeProvider::RecaptchaV3 { action } => Some(view! {
            <script
                nonce=widget.script_nonce.clone()
                inner_html=recaptcha_v3_script(&key, action)
            ></script>
        }),
        _ => None,
    };

    // `inner_html`, not child views: with scripting on, the HTML parser keeps
    // `<noscript>` content as one text node, so hydrating a `<p>` child would
    // fail.  tachys leaves `inner_html` content alone when hydrating.
    let no_js = (widget.no_js == NoJsPolicy::Reject).then(|| {
        let html = format!(
            "<p class=\"{}\" role=\"alert\">{}</p>",
            escape_html(&error_class),
            escape_html(&requires_js)
        );
        view! { <noscript inner_html=html></noscript> }
    });

    view! {
        <div class=field_class>
            {element}
            {vendor_script}
            {v3_script}
            {no_js}
        </div>
    }
    .into_any()
}

// ---------------------------------------------------------------------------
// Site-defined fields (RFC 015)
// ---------------------------------------------------------------------------

/// The classes a site field's row is built from: the same ones the built-in
/// rows use, and nothing new.
struct RowClasses {
    field: String,
    label: String,
    input: String,
    textarea: String,
    error: String,
}

/// The `id` of a site field's control (RFC 015 A7); its error paragraph is
/// `{id}-error`, following the pattern of the built-in rows.
fn site_field_id(key: &str) -> String {
    format!("contact-field-{key}")
}

/// One row for a site-defined field: a label, the control for its kind, and an
/// error paragraph, built like the built-in rows.
///
/// The control's `name` is `fields[{key}]`, which `submit_contact` receives as
/// one map.  A `Choice` starts with an empty option, `—`, so a required choice
/// is refused by the browser until one is picked.
fn site_field_row(
    field: &SiteField,
    classes: &RowClasses,
    field_errors: Memo<ContactFieldErrors>,
    labels: StoredValue<ContactFormLabels>,
) -> AnyView {
    let id = site_field_id(&field.key);
    let name = format!("fields[{}]", field.key);
    let required = field.required;
    let max_len = field.max_len.to_string();

    let key = field.key.clone();
    let invalid = Signal::derive(move || field_errors.with(|f| f.site_fields.contains_key(&key)));
    let error_id = format!("{id}-error");
    let described_by = move || invalid.get().then(|| error_id.clone());

    let key = field.key.clone();
    let message = Signal::derive(move || {
        field_errors.with(|f| {
            f.site_fields.get(&key).map(|e| {
                // Only an email field words `format` differently, and this is
                // not one.
                labels.with_value(|l| l.errors.field_text(ContactField::Message, e))
            })
        })
    });

    let control = match &field.kind {
        SiteFieldKind::Line => view! {
            <input
                id=id.clone()
                name=name
                type="text"
                class=classes.input.clone()
                required=required
                maxlength=max_len
                aria-required=required.then_some("true")
                aria-invalid=move || invalid.get().then_some("true")
                aria-describedby=described_by
            />
        }
        .into_any(),
        SiteFieldKind::Text => view! {
            <textarea
                id=id.clone()
                name=name
                class=classes.textarea.clone()
                required=required
                maxlength=max_len
                aria-required=required.then_some("true")
                aria-invalid=move || invalid.get().then_some("true")
                aria-describedby=described_by
            />
        }
        .into_any(),
        SiteFieldKind::Choice(choices) => {
            let options: Vec<_> = choices
                .iter()
                .map(|choice| {
                    view! { <option value=choice.key.clone()>{choice.label.clone()}</option> }
                })
                .collect();
            view! {
                <select
                    id=id.clone()
                    name=name
                    class=classes.input.clone()
                    required=required
                    aria-required=required.then_some("true")
                    aria-invalid=move || invalid.get().then_some("true")
                    aria-describedby=described_by
                >
                    <option value="">"—"</option>
                    {options}
                </select>
            }
            .into_any()
        }
    };

    view! {
        <div class=classes.field.clone()>
            <label for=id.clone() class=classes.label.clone()>{field.label.clone()}</label>
            {control}
            <FieldError input_id=id class=classes.error.clone() message=message />
        </div>
    }
    .into_any()
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
/// | `challenge` | [`ChallengeWidget`]   | No       | none         |
/// | `site_fields` | [`SiteFields`]      | No       | none         |
///
/// # State model
///
/// The `<form>` and its inputs are created once and kept across submissions.
/// Only the success region, the generic-error region, and each field's error
/// paragraph and ARIA attributes react to the action's value.  Keeping the
/// form itself out of the reactive closure is what preserves the hidden
/// anti-automation token across submissions.
///
/// # Form token in the browser
///
/// The hidden token field is a signal, initialised from the server render.
/// The browser contacts the token endpoint only when
/// [`ContactFormOptions::token_refresh_secs`] is `Some` — set it when the
/// server issues form tokens.  Then, after mount, the browser reads the
/// field's **DOM** value: a token the server put there is kept; an empty
/// field — the form was created by client-side navigation — fetches one from
/// [`issue_form_token_fn`](crate::server::issue_form_token_fn).
///
/// Refreshes follow.  The token the form mounted with is refreshed once its
/// refresh point is reached, immediately if that is already past.  Every
/// token fetched after that is refreshed `token_refresh_secs` after it
/// *arrived*, on the browser clock alone, so a wrong clock cannot make the
/// refreshes loop.
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
    /// A CAPTCHA widget, rendered after the message field.  Without it the
    /// form renders and loads nothing from any vendor.
    ///
    /// Takes the `Option` itself, so a widget that exists only when keys are
    /// configured is one prop: `challenge=widget_from_config()`, or
    /// `challenge=Some(widget)` for a fixed one.
    #[prop(optional_no_strip)]
    challenge: Option<ChallengeWidget>,
    /// The site's own fields (RFC 015), rendered between the subject and the
    /// message in the order they were defined.
    ///
    /// Give this the same [`SiteFields`] as
    /// [`ContactServerPolicy::site_fields`](crate::config::ContactServerPolicy::site_fields),
    /// built once in shared code: the server accepts only the keys its
    /// definition has, so a form and a server that disagree are refused, not
    /// silently accepted.  An error the server reports for a key this form
    /// did not render is shown in the generic error message.
    #[prop(optional, into)]
    site_fields: SiteFields,
) -> impl IntoView {
    let submit_action = ServerAction::<SubmitContact>::new();
    let pending = submit_action.pending();
    let value = submit_action.value();

    // SSR has the token in context; the browser does not, and hydration keeps
    // the server-rendered attribute until the signal changes.  In the browser
    // the acquisition effect below copies that attribute into the signal.
    #[cfg(feature = "form-token")]
    let initial_token: String = leptos::context::use_context::<FormToken>()
        .map(|t| t.0.clone())
        .unwrap_or_default();
    #[cfg(not(feature = "form-token"))]
    let initial_token: String = String::new();
    let token = RwSignal::new(initial_token);
    let token_ref = NodeRef::<leptos::html::Input>::new();

    let classes = StoredValue::new(classes);
    let labels = StoredValue::new(labels);
    let options = StoredValue::new(options);
    let challenge = StoredValue::new(challenge);
    let site_fields = StoredValue::new(site_fields);

    // ---- derived state -----------------------------------------------------

    let succeeded = Memo::new(move |_| value.with(|v| matches!(v, Some(Ok(())))));

    let field_errors = Memo::new(move |_| {
        value.with(|v| match v {
            Some(Err(e)) => ContactFieldErrors::from_server_fn_error(e).unwrap_or_default(),
            _ => ContactFieldErrors::default(),
        })
    });

    // Every error that carries no field errors shows the generic banner: a
    // recognised `contact_error:` code renders its own label, anything else
    // falls back to `labels.error`.
    //
    // So does a field error for a site field this form did not render: there is
    // no row to show it in, and dropping it would hide a mismatch between the
    // form and the server (RFC 015 D3).
    let generic_error = Memo::new(move |_| {
        value.with(|v| match v {
            Some(Err(e)) => {
                let fields = ContactFieldErrors::from_server_fn_error(e);
                let unrendered = fields.as_ref().is_some_and(|f| {
                    site_fields.with_value(|d| f.site_fields.keys().any(|k| d.get(k).is_none()))
                });
                if unrendered || fields.is_none_or(|f| f.is_empty()) {
                    Some(match ContactErrorCode::from_server_fn_error(e) {
                        Some(code) => labels.with_value(|l| l.errors.code_text(code)),
                        None => labels.with_value(|l| l.error.clone()),
                    })
                } else {
                    None
                }
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
            // Document order: the site's fields sit between the subject and
            // the message.  Only fields this form renders are candidates.
            let first = field_errors.with(|f| {
                let site = site_fields.with_value(|d| {
                    d.iter()
                        .map(|field| {
                            (
                                site_field_id(&field.key),
                                f.site_fields.contains_key(&field.key),
                            )
                        })
                        .collect::<Vec<_>>()
                });
                [
                    ("contact-name".to_owned(), f.name.is_some()),
                    ("contact-email".to_owned(), f.email.is_some()),
                    ("contact-subject".to_owned(), f.subject.is_some()),
                ]
                .into_iter()
                .chain(site)
                .chain([("contact-message".to_owned(), f.message.is_some())])
                .find(|(_, has)| *has)
                .map(|(id, _)| id)
            });
            if let Some(id) = first
                && let Some(el) = document().get_element_by_id(&id)
            {
                use leptos::wasm_bindgen::JsCast;
                if let Ok(el) = el.dyn_into::<leptos::web_sys::HtmlElement>() {
                    let _ = el.focus();
                }
            }
        });
    }

    // ---- form token in the browser: only when the page asks for it ---------

    // Same gate as the focus effect above: this needs a DOM and a real client.
    // `token_refresh_secs` is the switch.  The browser cannot tell a server
    // without form tokens from a form reached by client-side navigation — both
    // leave the field empty — so with `None` it never calls the endpoint.
    #[cfg(all(feature = "hydrate", not(feature = "ssr")))]
    if let Some(refresh) = options.with_value(|o| o.token_refresh_secs) {
        use crate::server::{IssueFormTokenFn, mounted_refresh_delay, token_issued_at};

        let issue = ServerAction::<IssueFormTokenFn>::new();
        // Values of 60 or less mean a TTL of two minutes or less, where a
        // refresh would race the expiry: acquire a missing token, never refresh.
        let refreshes = refresh > 60;

        // At most one refresh is ever pending; scheduling replaces it.
        let pending = StoredValue::new(None::<TimeoutHandle>);
        let schedule = move |delay_secs: u64| {
            if let Some(handle) = pending.get_value() {
                handle.clear();
            }
            let handle = set_timeout_with_handle(
                move || {
                    pending.set_value(None);
                    // The form is gone after a successful submission.
                    if token_ref.get_untracked().is_some() {
                        issue.dispatch(IssueFormTokenFn {});
                    }
                },
                std::time::Duration::from_secs(delay_secs),
            )
            .ok();
            pending.set_value(handle);
        };
        on_cleanup(move || {
            if let Some(handle) = pending.get_value() {
                handle.clear();
            }
        });

        // Mount.  The DOM value, not a hydration flag, decides: a server render
        // leaves a token in the field, client-side navigation leaves it empty.
        Effect::new(move |_| {
            let Some(el) = token_ref.get() else {
                return;
            };
            let rendered = el.value();
            if rendered.is_empty() {
                issue.dispatch(IssueFormTokenFn {});
                return;
            }
            // The browser build has no `FormToken` context, so the signal
            // starts empty even when the server rendered a token.  Adopt it;
            // the DOM already shows this value, so nothing visible changes.
            if token.with_untracked(|t| *t != rendered) {
                token.set(rendered.clone());
            }
            // The only place a server timestamp meets the browser clock.  An
            // overdue token refreshes once, now; that cannot repeat, because
            // every later refresh is timed from arrival, below.
            if refreshes && let Some(issued) = token_issued_at(&rendered) {
                let now = (leptos::web_sys::js_sys::Date::now() / 1000.0) as u64;
                schedule(mounted_refresh_delay(issued, now, refresh));
            }
        });

        // Arrival.  Timed on the browser clock alone, never from the token's
        // server timestamp, so a skewed clock cannot shorten the interval.
        Effect::new(move |_| {
            if let Some(Ok(t)) = issue.value().get() {
                token.set(t);
                if refreshes {
                    schedule(refresh);
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
                // The honeypot wrapper: a class only when one is set, and the
                // inline style unless the site hides the wrapper itself (RFC 012).
                let honeypot_class = classes
                    .with_value(|c| (!c.honeypot.is_empty()).then(|| c.honeypot.clone()));
                let honeypot_style = options.with_value(|o| o.honeypot_inline_style).then_some(
                    "position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden",
                );
                let show_subject = options.with_value(|o| o.show_subject);
                let require_subject = options.with_value(|o| o.require_subject);
                let max_msg_len = options.with_value(|o| o.effective_max_message_len());
                let row_classes = RowClasses {
                    field: fc.clone(),
                    label: lc.clone(),
                    input: ic.clone(),
                    textarea: tac.clone(),
                    error: ec.clone(),
                };
                let site_rows: Vec<AnyView> = site_fields.with_value(|d| {
                    d.iter()
                        .map(|field| site_field_row(field, &row_classes, field_errors, labels))
                        .collect()
                });
                // Message.
                let message_row = view! {
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
                            message=Signal::derive(move || field_errors.with(|f| f.message.as_ref().map(|e| labels.with_value(|l| l.errors.field_text(ContactField::Message, e)))))
                        />
                    </div>
                };
                // With no site fields the message row stands alone.  An empty
                // list of rows would still leave a hydration marker between
                // the subject and the message, and a site that defines no
                // fields must render exactly what 0.7 did.
                let message_block = if site_rows.is_empty() {
                    Either::Left(message_row)
                } else {
                    Either::Right((site_rows, message_row))
                };
                let challenge_view = challenge.with_value(|c| {
                    c.as_ref().map(|w| {
                        challenge_markup(
                            w,
                            fc.clone(),
                            ec.clone(),
                            labels.with_value(|l| l.errors.challenge_requires_js.clone()),
                        )
                    })
                });

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
                                message=Signal::derive(move || field_errors.with(|f| f.name.as_ref().map(|e| labels.with_value(|l| l.errors.field_text(ContactField::Name, e)))))
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
                                message=Signal::derive(move || field_errors.with(|f| f.email.as_ref().map(|e| labels.with_value(|l| l.errors.field_text(ContactField::Email, e)))))
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
                                    message=Signal::derive(move || field_errors.with(|f| f.subject.as_ref().map(|e| labels.with_value(|l| l.errors.field_text(ContactField::Subject, e)))))
                                />
                            </div>
                        })}

                        // The site's own fields, in the order it defined them,
                        // then the message.
                        {message_block}

                        // Challenge widget — only when the prop is set.
                        {challenge_view}

                        // Form token — hidden field.  Rendered by SSR, or
                        // fetched by the browser when it arrives empty.  Empty
                        // when the `form-token` feature is disabled or
                        // FormTokenContext is not provided.
                        <input
                            type="hidden"
                            name="form_token"
                            value=move || token.get()
                            node_ref=token_ref
                        />

                        // Honeypot — visually hidden; excluded from assistive tech.
                        // Without a class the wrapper renders exactly as in 0.6:
                        // an empty `class` would still be rendered as `class=""`.
                        {
                            let body = view! {
                                <label for="contact-website">{l_honey}</label>
                                <input
                                    id="contact-website"
                                    name="website"
                                    type="text"
                                    tabindex="-1"
                                    autocomplete="off"
                                />
                            };
                            match honeypot_class {
                                Some(class) => view! {
                                    <div class=class aria-hidden="true" style=honeypot_style>
                                        {body}
                                    </div>
                                }
                                .into_any(),
                                None => view! {
                                    <div aria-hidden="true" style=honeypot_style>
                                        {body}
                                    </div>
                                }
                                .into_any(),
                            }
                        }

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
