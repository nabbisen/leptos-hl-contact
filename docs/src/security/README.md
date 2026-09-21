# Security

This section explains what the crate protects against, what your
application must add, and why.  The condensed action list is the
[Production Checklist](../getting-started/production-checklist.md); the
full threat model is in [External Design](../development/external-design.md#5-security-external-design).

## Who does what

| Concern | Crate | Your application |
|---------|-------|------------------|
| Server-side validation on every submission | ✅ | — |
| Honeypot bot filter | ✅ | — |
| Email header injection prevention | ✅ | — |
| Credentials and recipient kept out of WASM | ✅ | load them from the environment |
| Generic error messages, details only in logs | ✅ | — |
| PII-free log events | ✅ | log retention |
| Form token (`form-token` feature) | ✅ | provide secret and context |
| **Origin / Referer validation** — the CSRF control | example | ✅ middleware |
| Rate limiting | example | ✅ middleware |
| Request body limit | example | ✅ layer |
| TLS | — | ✅ proxy |
| [Challenge](./challenge.md) — Turnstile, hCaptcha, reCAPTCHA | ✅ widget and verification (`challenge-http`) | ✅ vendor keys, if needed |
| [Filter](./filter.md) — your own content rules | ✅ the hook | ✅ your rules, if needed |

## Which layer decides what

Six mechanisms inside the crate can stop a submission.  Each answers one
question; pick the one whose question is yours.

| Mechanism | The question it answers | Configured by | Runs | Visitor sees on failure |
|-----------|-------------------------|---------------|------|-------------------------|
| [Honeypot](../reference/api.md#contactinput) | Did a bot fill the hidden field? | nothing | always | success (silent) |
| [Form token](./form-token.md) | Did the sender fetch our page recently and wait before submitting — and, with binding, from this browser? | `FormTokenContext` | when configured | "reload" / "wait a moment" |
| [Site-field allow-list](../guides/customization.md#site-defined-fields) | Is every field key, and every choice, one this site defined? | `ContactServerPolicy::site_fields` | always | generic rejection for an unknown key or too many keys; field error for a bad value or an unlisted choice |
| [Server policy](../guides/customization.md#contactserverpolicy) | Does the input meet this site's structural limits? | `ContactServerPolicy` | when configured | field error |
| [Challenge](./challenge.md) | Did a vendor judge the sender human? | `ChallengeContext` + `challenge` prop | when configured | "complete the check" |
| [Filter](./filter.md) | Does this site want this content? | `ContactFilterContext` | when configured | generic rejection or silent |

Every value in "Configured by" is provided the same way, in the one context
closure passed to `leptos_routes_with_context`; none of them reads
environment variables or global state.  Each mechanism is explained on its
own page only.

## Layers, from the edge inward

Cheap checks first, so expensive ones rarely run:

1. TLS termination and WAF at the proxy
2. [Request body limit](./hardening.md#request-body-limit)
3. [Rate limit](./hardening.md#rate-limiting) keyed by real client IP
4. [Origin / Referer validation](./hardening.md#origin--referer-validation) on POST
5. [Form token](./form-token.md)
6. Honeypot
7. Field validation and [server policy](../guides/customization.md#contactserverpolicy)
8. Optional [challenge](./challenge.md)
9. Optional [filter](./filter.md)

## Pages in this section

- [Form Token](./form-token.md) — the `form-token` feature
- [Hardening](./hardening.md) — rate limiting, origin validation, body limit, secrets
- [Challenge](./challenge.md) — Turnstile, hCaptcha or reCAPTCHA
- [Filter](./filter.md) — your own content rules, before delivery
