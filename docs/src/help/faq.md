# FAQ

## General

**Does it work without JavaScript?**
Yes.  The form is an `<ActionForm/>` and submits as a normal POST.
Validation and delivery are identical.  Configure a
[success page](../guides/customization.md#success-page) and a successful
submission redirects there with or without JavaScript; without one, a no-JS
submission reloads the form page and shows no confirmation.

**Which Leptos versions?**
v0.8 only.

**Actix Web instead of Axum?**
Yes.  The core is framework-neutral; provide the same context values
through your framework's Leptos integration.  Only the optional
`axum-helpers` feature is Axum-specific.

**Can I have two forms on one page?**
Not yet.  Element ids are fixed (`contact-name`, …).

## Delivery

**How do I test without a mail server?**
Use `NoopDelivery`, or run MailHog locally — see
[Delivery Backends](../guides/delivery-backends.md#testing-delivery-locally).

**SendGrid, SES, Resend?**
Implement `ContactDelivery`; a template is in
[Delivery Backends](../guides/delivery-backends.md#writing-your-own-backend).

**Where is the recipient address configured?**
`SmtpConfig::to_address`, server side only.  It is never sent to the
browser.

## Security

**Is the honeypot enough?**
No.  Add rate limiting and Origin validation before going public, and a
CAPTCHA for high-value forms.  Start with the
[Production Checklist](../getting-started/production-checklist.md).

**Does the crate handle CSRF?**
Partly.  The `csrf` feature adds a signed, expiring token that stops bots
which never fetched the page, but the token is not tied to the visitor's
browser, so the real cross-site control is Origin / Referer validation in
your middleware.  Details: [Security](../security/README.md#about-the-token).

**Can an attacker inject email headers?**
No.  `name` and `subject` are rejected if they contain line breaks and
sanitised again when the message is built; the visitor's address is only
ever used in `Reply-To`.

**Are SMTP credentials safe?**
They live in `SmtpConfig` in your server binary, are never serialised, and
are redacted in `Debug` output.  Load them from the environment.

## Customization

**Other languages?**
Override `ContactFormLabels`; see [Localization](../guides/localization.md)
for what can and cannot be translated today.

**Tailwind?**
Pass classes through `ContactFormClasses`; see [Styling](../guides/styling.md).

**Hide or require the subject?**
`ContactFormOptions { show_subject: false, .. }` hides it.  To require it,
set `require_subject` in **both** `ContactFormOptions` (UI) and
`ContactServerPolicy` (server); the first alone can be bypassed by a
direct POST.  See [Customization](../guides/customization.md).

## Errors

**The visitor sees a generic error. Where is the real one?**
In your server logs, at `error` level with the category and transport
detail.  Nothing internal is sent to the browser by design.

**"Contact form is not configured."**
`ContactDeliveryContext` is missing from the server-function handler.
See [Axum Integration](../guides/axum-integration.md#the-two-context-sites).

**"Contact form security is not configured."**
The `csrf` feature is on but `CsrfConfigContext` is missing.  Same fix,
for the token config.
