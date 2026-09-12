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
| ☐ | `csrf` feature enabled with a 32-byte random `CSRF_SECRET`; context provided at both sites | [Anti-automation Token](../security/csrf.md) |
| ☐ | Reverse proxy sets and validates `X-Forwarded-For` | [Hardening](../security/hardening.md#rate-limiting) |
| ☐ | SMTP credentials loaded from environment or a secret store, never source | [Hardening](../security/hardening.md#secrets) |
| ☐ | `SmtpTlsMode::StartTls` or `Tls`, never `DangerousPlaintext` | [Delivery Backends](../guides/delivery-backends.md) |
| ☐ | Server policy set if the UI requires a subject or caps the message | [Customization](../guides/customization.md#contactserverpolicy) |
| ☐ | Logs reviewed: no message bodies, addresses, or secrets | [Security](../security/README.md) |
| ☐ | Success page configured (`success_redirect`) if visitors without JavaScript must see a confirmation | [Customization](../guides/customization.md#success-page) |
| ☐ | Form smoke-tested with JavaScript disabled | [Troubleshooting](../help/troubleshooting.md) |
| ☐ | Optional: CAPTCHA for high-value forms | [Turnstile](../security/turnstile.md) |

The [`axum-with-security`](https://github.com/nabbisen/leptos-hl-contact/tree/main/examples/axum-with-security)
example implements every non-optional item.
