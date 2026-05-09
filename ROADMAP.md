# Roadmap

## MVP (v0.1.0 — released)

- [x] `ContactForm` component
- [x] `submit_contact` server function
- [x] `ContactInput` model with server-side validation
- [x] Honeypot field
- [x] `ContactDelivery` trait
- [x] `NoopDelivery`
- [x] `LettreSmtpDelivery`
- [x] Class and labels injection
- [x] `ContactFormOptions`
- [x] Basic documentation
- [x] Axum example
- [x] Apache-2.0 licence

## Near-term (v0.2.x / v0.3.0 — released)

- [x] Per-field validation errors (`ContactFieldErrors`)
- [x] `axum-helpers` feature: `delivery_context_fn`, `provide_contact_delivery`
- [x] CSRF guidance and example middleware
- [x] Rate limit guide (tower-governor / axum-governor)
- [x] Cloudflare Turnstile integration guide
- [x] CSRF token helper (`csrf` feature) — HMAC-SHA256 stateless tokens
- [x] Rate limit integration example (`examples/axum-with-security`)

## Future

- [ ] SendGrid adapter (`sendgrid` feature)
- [ ] AWS SES adapter (`ses` feature)
- [ ] Resend adapter (`resend` feature)
- [ ] Database persistence adapter
- [ ] Queue-based delivery adapter
- [ ] Advanced slot / render prop API (submit button, success/error slots)
- [ ] Multi-language label preset library
- [ ] mdBook-published documentation site

## Not planned for now

- GUI admin panel for managing submissions
- Attachment / file upload support
- Complex form builder UI
