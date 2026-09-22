# Feature Flags

`default = []`.  The crate compiles with no server or client dependencies;
downstream crates choose exactly what they activate.

| Flag | Implies | Enables | Workers |
|------|---------|---------|---------|
| `hydrate` | — | Leptos client-side hydration | — (browser build) |
| `ssr` | — | Server-side rendering and the `submit_contact` body | supported |
| `islands` | — | Leptos Islands architecture | not verified |
| `smtp-lettre` | `ssr`, `delivery-timeout` | `delivery::smtp` (`lettre`, `tokio`) | **not supported** |
| `delivery-timeout` | `ssr` | `DeliveryTimeout`, a deadline for any delivery backend (`tokio` with `time` natively; a JavaScript timer on a wasm32 server) | supported |
| `axum-helpers` | `ssr` | `axum_helpers` (`axum`, `leptos_axum`) | supported |
| `form-token` | `ssr` | `form_token` module; token verification in `submit_contact` (`hmac`, `sha2`, `rand`, `hex`) | supported |
| `challenge-http` | `ssr` | `HttpChallengeVerifier`, which calls the vendors' siteverify endpoints (`reqwest` with rustls natively; `fetch` on a wasm32 server) | supported |
| `delivery-resend` | `ssr` | `delivery::resend` (`ResendConfig`, `ResendDelivery`), which posts to Resend's HTTP API (`reqwest` with rustls natively; `fetch` on a wasm32 server — the same dependency set `challenge-http` names) | supported |
| `email-domain-check` | `ssr` | `email_domain::EmailDomainCheck`, an opt-in check that a visitor's email domain can receive mail, over DNS over HTTPS (`reqwest` with rustls natively; `fetch` on a wasm32 server — the same dependency set `challenge-http` and `delivery-resend` name) | supported |

**Workers** is Cloudflare Workers, a Leptos server on wasm32 with no tokio
runtime; see [Cloudflare Workers](../guides/cloudflare-workers.md).

## Recommended combinations

| Binary | Features |
|--------|----------|
| Server, minimal | `ssr`, `smtp-lettre` |
| Server, Axum, production | `ssr`, `smtp-lettre`, `axum-helpers`, `form-token` |
| Server, Cloudflare Workers | `ssr`, `form-token`, `delivery-resend`, `axum-helpers`, `delivery-timeout` |
| … with a challenge | the above, plus `challenge-http` |
| … with the email domain check | the above, plus `email-domain-check` |
| WASM client | `hydrate` |
| Islands server | `islands`, `ssr`, `smtp-lettre` |
| Library tests | `--all-features` |

Features are additive.  Enabling one never removes an API or changes the
behaviour of another, with one deliberate exception: enabling `form-token`
makes `submit_contact` **require** `FormTokenContext` (fail-closed).
