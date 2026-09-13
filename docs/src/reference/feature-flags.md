# Feature Flags

`default = []`.  The crate compiles with no server or client dependencies;
downstream crates choose exactly what they activate.

| Flag | Implies | Enables |
|------|---------|---------|
| `hydrate` | — | Leptos client-side hydration |
| `ssr` | — | Server-side rendering and the `submit_contact` body |
| `islands` | — | Leptos Islands architecture |
| `smtp-lettre` | `ssr` | `delivery::smtp` (`lettre`, `tokio`) |
| `axum-helpers` | `ssr` | `axum_helpers` (`axum`, `leptos_axum`) |
| `form-token` | `ssr` | `form_token` module; token verification in `submit_contact` (`hmac`, `sha2`, `rand`, `hex`) |
| `challenge-http` | `ssr` | `HttpChallengeVerifier`, which calls the vendors' siteverify endpoints (`reqwest` with rustls only) |

## Recommended combinations

| Binary | Features |
|--------|----------|
| Server, minimal | `ssr`, `smtp-lettre` |
| Server, Axum, production | `ssr`, `smtp-lettre`, `axum-helpers`, `form-token` |
| … with a challenge | the above, plus `challenge-http` |
| WASM client | `hydrate` |
| Islands server | `islands`, `ssr`, `smtp-lettre` |
| Library tests | `--all-features` |

Features are additive.  Enabling one never removes an API or changes the
behaviour of another, with one deliberate exception: enabling `form-token`
makes `submit_contact` **require** `FormTokenContext` (fail-closed).
