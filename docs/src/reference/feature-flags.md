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
| `csrf` | `ssr` | `csrf` module; token verification in `submit_contact` (`hmac`, `sha2`, `rand`, `hex`) |

## Recommended combinations

| Binary | Features |
|--------|----------|
| Server, minimal | `ssr`, `smtp-lettre` |
| Server, Axum, production | `ssr`, `smtp-lettre`, `axum-helpers`, `csrf` |
| WASM client | `hydrate` |
| Islands server | `islands`, `ssr`, `smtp-lettre` |
| Library tests | `--all-features` |

Features are additive.  Enabling one never removes an API or changes the
behaviour of another, with one deliberate exception: enabling `csrf`
makes `submit_contact` **require** `CsrfConfigContext` (fail-closed).
