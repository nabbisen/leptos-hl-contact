# Production Checklist

The Quick Start form is safe to develop against but not to expose.  Work
through this list before the form is public.  Each item links to the page
that explains it.

| Done | Item | Where |
|------|------|-------|
| ☐ | TLS terminated in front of the application | your proxy |
| ☐ | Request body limited (32 KiB is plenty) | [Hardening](../security/hardening.md#request-body-limit) |
| ☐ | Rate limiting on POST, keyed by real client IP | [Hardening](../security/hardening.md#rate-limiting) |
| ☐ | Origin / Referer strictly validated on POST — **this is the CSRF control** | [Hardening](../security/hardening.md#origin--referer-validation) |
| ☐ | `form-token` feature enabled with a 32-byte random `FORM_TOKEN_SECRET`; config and token provided in the context closure | [Form Token](../security/form-token.md) |
| ☐ | Cookie binding on (`Binding::Cookie`) with `Secure` left at its default; the contact page not cached | [Cookie binding](../security/form-token.md#cookie-binding) |
| ☐ | Reverse proxy sets and validates `X-Forwarded-For` | [Hardening](../security/hardening.md#rate-limiting) |
| ☐ | SMTP credentials loaded from environment or a secret store, never source | [Hardening](../security/hardening.md#secrets) |
| ☐ | `SmtpTlsMode::StartTls` or `Tls`, never `DangerousPlaintext` | [Delivery Backends](../guides/delivery-backends.md) |
| ☐ | Using `ResendDelivery`: the API key comes from the environment; the sender's domain is verified at the provider; a delivery failure has been seen once in staging, not only imagined | [Delivery Backends](../guides/delivery-backends.md#resenddelivery) |
| ☐ | Server policy set if the UI requires a subject or caps the message | [Customization](../guides/customization.md#contactserverpolicy) |
| ☐ | Logs reviewed: no message bodies, addresses, or secrets | [Security](../security/README.md) |
| ☐ | Success page configured (`success_redirect`) if visitors without JavaScript must see a confirmation | [Customization](../guides/customization.md#success-page) |
| ☐ | Form smoke-tested with JavaScript disabled | [Troubleshooting](../help/troubleshooting.md) |
| ☐ | Optional: a challenge for high-value forms — widget and `ChallengeContext` both set, and the widget's no-JS policy matching the server's; `ChallengeContext` provided whenever the widget renders, with an empty secret if the secret is missing | [Challenge](../security/challenge.md), [A missing secret](../security/challenge.md#what-the-server-decides) |

## Cloudflare Workers

In addition to the rows above that apply (TLS and body limits are Cloudflare's):

| Done | Item | Where |
|------|------|-------|
| ☐ | Features: `ssr`, `form-token`, `delivery-resend` (or your own backend), `axum-helpers`, `delivery-timeout`; not `smtp-lettre`, which cannot run here | [Cloudflare Workers](../guides/cloudflare-workers.md#what-works) |
| ☐ | `ChallengeClientIp` provided from `CF-Connecting-IP`, if you use a challenge | [The visitor's IP](../guides/cloudflare-workers.md#the-visitors-ip) |
| ☐ | Rate limit on `POST` verified on the deployed Worker: the Rate Limiting binding before the router, or a rate-limiting rule | [Rate limiting](../guides/cloudflare-workers.md#rate-limiting) |
| ☐ | Content Security Policy per the challenge provider; honeypot class and `honeypot_inline_style: false` without `'unsafe-inline'` | [Challenge: CSP](../security/challenge.md#content-security-policy) |
| ☐ | Your delivery backend wrapped in `DeliveryTimeout` | [The delivery deadline](../guides/cloudflare-workers.md#the-delivery-deadline) |
| ☐ | Runtime smoke test on `wrangler dev`: with and without JavaScript, with the challenge, with a failing delivery | [Testing locally](../guides/cloudflare-workers.md#testing-locally) |

The [`axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
example implements every non-optional item.
