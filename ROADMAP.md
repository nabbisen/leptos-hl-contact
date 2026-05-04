# Roadmap

## MVP

Initial release targeting Leptos v0.8.

- [x] `ContactForm` component
- [x] `submit_contact` server function
- [x] `ContactInput` model with server-side validation
- [x] Honeypot field
- [x] `ContactDelivery` trait
- [x] `NoopDelivery`
- [x] `LettreSmtpDelivery`
- [x] Class and labels injection via `ContactFormClasses` / `ContactFormLabels`
- [x] `ContactFormOptions` (subject visibility, max message length)
- [x] Basic documentation (`docs/`)
- [x] Axum example (`examples/axum-basic`)
- [x] Apache-2.0 licence

## Near-term

- [ ] Axum helper: `provide_contact_delivery` convenience function
- [ ] CSRF guidance and example middleware configuration
- [ ] Rate limit guide (tower-governor / axum-governor)
- [ ] Better client-side error detail for individual field validation
- [ ] Cloudflare Turnstile integration guide

## Future

- [ ] SendGrid adapter (`sendgrid` feature)
- [ ] AWS SES adapter (`ses` feature)
- [ ] Resend adapter (`resend` feature)
- [ ] Database persistence adapter
- [ ] Queue-based delivery adapter
- [ ] Advanced slot / render prop API (submit button, success/error slots)
- [ ] Multi-language preset library
- [ ] mdBook-published documentation site

## Not planned for now

- GUI admin panel for managing submissions
- Attachment / file upload support
- Complex form builder UI
