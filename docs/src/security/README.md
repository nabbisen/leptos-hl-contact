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

## Layers, from the edge inward

Cheap checks first, so expensive ones rarely run:

1. TLS termination and WAF at the proxy
2. [Request body limit](./hardening.md#request-body-limit)
3. [Rate limit](./hardening.md#rate-limiting) keyed by real client IP
4. [Origin / Referer validation](./hardening.md#origin--referer-validation) on POST
5. [Form token](./form-token.md): proves the sender fetched a page from this server within the last hour, and waited at least a moment before submitting
6. Honeypot
7. Field validation and [server policy](../guides/customization.md#contactserverpolicy)
8. Optional [challenge](./challenge.md), verified with the vendor before delivery

## About the token

The `form-token` feature issues a stateless HMAC-SHA256 token per page render and
verifies it on submit.  By default it is **not bound to the visitor's
browser**, so on its own it does not stop a cross-site request: an attacker
can fetch a token and place it in a form on another site.  What it does well
is force bots to fetch a page first and wait before submitting.

`Binding::Cookie` ties the token to an `HttpOnly`, `SameSite=Lax`,
`__Host-`-prefixed cookie, which makes it a second check against cross-site
submissions.  It is defence in depth: **the control that rejects cross-site
POSTs is the Origin / Referer check, with or without binding.**  Read
[Form Token](./form-token.md) for the exact guarantees and their limits.

## Pages in this section

- [Form Token](./form-token.md) — the `form-token` feature
- [Hardening](./hardening.md) — rate limiting, origin validation, body limit, secrets
- [Challenge](./challenge.md) — Turnstile, hCaptcha or reCAPTCHA
